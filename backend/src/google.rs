//! Google OIDC login: installed-app loopback + PKCE (primary) with server-side
//! code exchange and ID-token verification via JWKS.
//!
//! Verified against Google docs 2026-09-23:
//! - Auth: `GET {auth_url}?response_type=code&client_id&redirect_uri&scope&
//!   state&code_challenge&code_challenge_method=S256&access_type=offline&
//!   prompt=consent`. Loopback `http://127.0.0.1:{any-port}` stays supported
//!   for **Desktop app** clients (RFC 8252 §7.3). OOB is dead — never used.
//! - Exchange: `POST {token_url}` + code/client_id/[client_secret]/
//!   redirect_uri/grant_type=authorization_code/code_verifier.
//! - ID token: RS256 JWT; verify signature via JWKS, `aud` == client_id,
//!   `iss` ∈ {accounts.google.com, https://accounts.google.com}, `exp` fresh.
//!
//! Endpoint bases are injectable (`OAuthConfig`) so tests run offline.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::oauth_config::OAuthConfig;
use crate::provider_http::{self, HttpError};

/// Transaction bindings per RFC 9700 §2.1.1 + OIDC Core: PKCE verifier,
/// OIDC nonce, auth-time window anchor. One-shot, 10-min TTL.
struct PendingFlow {
    verifier: String,
    redirect_uri: String,
    nonce: String,
    /// Unix epoch when the flow was registered (auth_time lower bound).
    created_epoch: i64,
    created_at: Instant,
}

static PENDING_FLOWS: OnceLock<Mutex<HashMap<String, PendingFlow>>> = OnceLock::new();

fn flows() -> &'static Mutex<HashMap<String, PendingFlow>> {
    PENDING_FLOWS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_flows() -> std::sync::MutexGuard<'static, HashMap<String, PendingFlow>> {
    match flows().lock() {
        Ok(g) => g,
        Err(poisoned) => {
            tracing::warn!("google flows mutex poisoned; recovering inner");
            poisoned.into_inner()
        }
    }
}

const FLOW_TTL: Duration = Duration::from_secs(600);
const MAX_FLOWS: usize = 1000;
/// Acceptable clock skew vs the provider (matches MSAL/industry practice).
const CLOCK_SKEW_SECS: i64 = 120;
/// Max ID-token age at use: login must be fresh, never a replayed hour-old token.
const MAX_ID_TOKEN_AGE_SECS: i64 = 900;
/// Cap on provider JSON bodies (token/userinfo/JWKS).
const MAX_PROVIDER_BODY: usize = 256 * 1024;
const MAX_JWKS_BODY: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoogleError {
    NotConfigured,
    Transport,
    Protocol,
    Unauthorized,
    BadRedirectUri,
    BadState,
    FlowExpired,
}

impl std::fmt::Display for GoogleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => write!(f, "google login not configured"),
            Self::Transport => write!(f, "google unreachable"),
            Self::Protocol => write!(f, "google protocol error"),
            Self::Unauthorized => write!(f, "google token invalid"),
            Self::BadRedirectUri => write!(f, "redirect_uri must be loopback http or https"),
            Self::BadState => write!(f, "unknown or reused state"),
            Self::FlowExpired => write!(f, "login flow expired, restart"),
        }
    }
}
impl std::error::Error for GoogleError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleClaims {
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TokenSet {
    pub id_token: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_in: Option<i64>,
}

fn google_client_id(cfg: &OAuthConfig) -> Result<&str, GoogleError> {
    cfg.google_client_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or(GoogleError::NotConfigured)
}

/// PKCE S256 pair: 64-char verifier, base64url-nopad SHA256 challenge.
pub fn pkce_pair() -> (String, String) {
    let verifier = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());
    (verifier, challenge)
}

/// New opaque login state token.
pub fn new_state() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Redirect URIs allowed: loopback http (any port) or https. Fail-closed.
pub fn validate_redirect_uri(uri: &str) -> Result<(), GoogleError> {
    if uri.len() > 512 || uri.is_empty() {
        return Err(GoogleError::BadRedirectUri);
    }
    if uri.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(GoogleError::BadRedirectUri);
    }
    let ok = uri.starts_with("http://127.0.0.1:")
        || uri.starts_with("http://[::1]:")
        || uri.starts_with("http://localhost:")
        || uri.starts_with("https://");
    if ok {
        Ok(())
    } else {
        Err(GoogleError::BadRedirectUri)
    }
}

fn prune_and_register(
    state: String,
    verifier: String,
    redirect_uri: String,
    nonce: String,
    created_epoch: i64,
) {
    let mut map = lock_flows();
    map.retain(|_, f| f.created_at.elapsed() < FLOW_TTL);
    if map.len() >= MAX_FLOWS {
        // Drop oldest to bound memory (fail-safe: affected logins restart).
        if let Some(oldest) = map
            .iter()
            .min_by_key(|(_, f)| f.created_at)
            .map(|(k, _)| k.clone())
        {
            map.remove(&oldest);
        }
    }
    map.insert(
        state,
        PendingFlow {
            verifier,
            redirect_uri,
            nonce,
            created_epoch,
            created_at: Instant::now(),
        },
    );
}

/// One-shot claim of a pending flow: (verifier, redirect_uri, nonce,
/// earliest acceptable auth_time). Unknown/reused/expired → error.
pub fn take_flow(state: &str) -> Result<(String, String, String, i64), GoogleError> {
    if state.trim().is_empty() || state.len() > 256 {
        return Err(GoogleError::BadState);
    }
    let mut map = lock_flows();
    match map.remove(state) {
        Some(f) if f.created_at.elapsed() < FLOW_TTL => {
            Ok((f.verifier, f.redirect_uri, f.nonce, f.created_epoch))
        }
        Some(_) => Err(GoogleError::FlowExpired),
        None => Err(GoogleError::BadState),
    }
}

/// Clear pending flows (tests).
#[allow(dead_code)]
pub fn clear_flows() {
    lock_flows().clear();
}

/// Build the Google authorization URL and register the PKCE + nonce flow.
/// Returns (auth_url, state).
///
/// Security contract (RFC 9700 §2.1.1, OIDC Core):
/// - redirect_uri is the SERVER-PINNED value only. A caller-supplied URI is
///   accepted solely for compatibility and MUST equal the pinned value
///   exactly, otherwise the request is rejected (login-CSRF prevention).
/// - Fresh PKCE S256 pair + fresh nonce per transaction, stored server-side.
/// - `max_age=0` forces the provider to freshly authenticate the user and
///   return `auth_time`, which the callback verifies (no silent SSO reuse).
pub fn build_auth_url(
    cfg: &OAuthConfig,
    redirect_uri: Option<&str>,
    scopes: Option<&str>,
) -> Result<(String, String), GoogleError> {
    let client_id = google_client_id(cfg)?.to_string();
    let pinned = OAuthConfig::google_redirect_uri();
    validate_redirect_uri(&pinned)?;
    if let Some(requested) = redirect_uri {
        if requested != pinned {
            return Err(GoogleError::BadRedirectUri);
        }
    }
    let scopes = scopes
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("openid email profile");
    if scopes.len() > 512 {
        return Err(GoogleError::Protocol);
    }
    let (verifier, challenge) = pkce_pair();
    let state = new_state();
    // OIDC nonce: fresh high-entropy value per transaction (replay/CSRF).
    let nonce = new_state();
    let created_epoch = chrono::Utc::now().timestamp();
    // Minimal percent-encoding for query values (alphanumeric-safe set + path chars).
    let enc = |s: &str| -> String {
        let mut out = String::with_capacity(s.len());
        for b in s.bytes() {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~' | b':' | b'/') {
                out.push(b as char);
            } else {
                out.push_str(&format!("%{b:02X}"));
            }
        }
        out
    };
    let url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&nonce={}&code_challenge={}&code_challenge_method=S256&access_type=offline&prompt=consent&max_age=0",
        cfg.google_auth_url,
        enc(&client_id),
        enc(&pinned),
        enc(&scopes.replace(',', " ")),
        enc(&state),
        enc(&nonce),
        enc(&challenge),
    );
    prune_and_register(state.clone(), verifier, pinned, nonce, created_epoch);
    Ok((url, state))
}

/// Exchange authorization code for tokens (server side, verifier from store).
/// Returns (tokens, expected_nonce, earliest_auth_time) for ID verification.
pub async fn exchange_code(
    http: &reqwest::Client,
    cfg: &OAuthConfig,
    code: &str,
    state: &str,
) -> Result<(TokenSet, String, i64), GoogleError> {
    if code.trim().is_empty() || code.len() > 2048 {
        return Err(GoogleError::Protocol);
    }
    let client_id = google_client_id(cfg)?.to_string();
    let (verifier, redirect_uri, nonce, created_epoch) = take_flow(state)?;
    let mut params = vec![
        ("code", code.to_string()),
        ("client_id", client_id),
        ("redirect_uri", redirect_uri),
        ("grant_type", "authorization_code".to_string()),
        ("code_verifier", verifier),
    ];
    if let Some(secret) = cfg.google_client_secret.clone().filter(|s| !s.is_empty()) {
        params.push(("client_secret", secret));
    }
    let owned: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let v = provider_http::post_form(http, &cfg.google_token_url, &owned, MAX_PROVIDER_BODY)
        .await
        .map_err(|e| match e {
            HttpError::Transport => GoogleError::Transport,
            _ => GoogleError::Protocol,
        })?;
    if v.get("error").is_some() {
        return Err(GoogleError::Protocol);
    }
    if v.get("id_token").and_then(|x| x.as_str()).is_none() {
        return Err(GoogleError::Protocol);
    }
    let tokens = TokenSet {
        id_token: v
            .get("id_token")
            .and_then(|x| x.as_str())
            .ok_or(GoogleError::Protocol)?
            .to_string(),
        access_token: v
            .get("access_token")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
        refresh_token: v
            .get("refresh_token")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
        expires_in: v.get("expires_in").and_then(|x| x.as_i64()),
    };
    if tokens.id_token.len() > 8192 {
        return Err(GoogleError::Protocol);
    }
    Ok((tokens, nonce, created_epoch))
}

// ── JWKS + ID-token verification ──────────────────────────────────────

static JWKS_CACHE: OnceLock<Mutex<HashMap<String, (Instant, serde_json::Value)>>> = OnceLock::new();

fn jwks_cache() -> &'static Mutex<HashMap<String, (Instant, serde_json::Value)>> {
    JWKS_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_jwks() -> std::sync::MutexGuard<'static, HashMap<String, (Instant, serde_json::Value)>> {
    match jwks_cache().lock() {
        Ok(g) => g,
        Err(poisoned) => {
            tracing::warn!("jwks cache mutex poisoned; recovering inner");
            poisoned.into_inner()
        }
    }
}

const JWKS_TTL: Duration = Duration::from_secs(3600);

/// Fetch JWKS (1h cache, size-capped). Pure fetch — no verification.
pub async fn fetch_jwks(
    http: &reqwest::Client,
    jwks_url: &str,
) -> Result<serde_json::Value, GoogleError> {
    {
        let cache = lock_jwks();
        if let Some((at, v)) = cache.get(jwks_url) {
            if at.elapsed() < JWKS_TTL {
                return Ok(v.clone());
            }
        }
    }
    let (_status, v) = provider_http::get_json(http, jwks_url, None, &[], MAX_JWKS_BODY)
        .await
        .map_err(|e| match e {
            HttpError::Transport | HttpError::BadStatus(_) => GoogleError::Transport,
            _ => GoogleError::Protocol,
        })?;
    lock_jwks().insert(jwks_url.to_string(), (Instant::now(), v.clone()));
    Ok(v)
}

/// Verify an RS256 ID token (OIDC Core §3.1.3.7, fail-closed on any mismatch):
/// - header alg MUST be RS256 (no alg-confusion; `none`/HMAC rejected),
/// - `kid` REQUIRED and must match exactly one JWKS RSA key whose `alg`
///   (when present) is RS256 and whose `use` (when present) is `sig`.
///   `jku`/`x5u`/`x5c` are never trusted (no URL fetching, no chain building).
/// - signature valid; `aud` contains our client_id (`azp` enforced when aud
///   has multiple values); `iss` is Google; `exp` fresh (zero leeway).
/// - `nonce` MUST equal `expected_nonce` (replay/CSRF binding).
/// - `iat` within [now-15min, now+skew]; `auth_time` REQUIRED within
///   [`earliest_auth_time`, now+skew] (login must be fresh — with max_age=0
///   the provider freshly authenticated the user for THIS flow).
/// - `at_hash`, when present and the access token is known, MUST match
///   (token-substitution detection).
/// - `sub` non-empty, ≤255 chars. Identity is `sub` only — `email` is never
///   trusted for identity (unverified addresses must not become accounts).
pub fn verify_id_token(
    cfg: &OAuthConfig,
    id_token: &str,
    jwks: &serde_json::Value,
    expected_nonce: &str,
    access_token: Option<&str>,
    earliest_auth_time: i64,
) -> Result<GoogleClaims, GoogleError> {
    let client_id = google_client_id(cfg)?;
    if id_token.is_empty() || id_token.len() > 8192 {
        return Err(GoogleError::Unauthorized);
    }
    if expected_nonce.is_empty() || expected_nonce.len() > 256 {
        return Err(GoogleError::Unauthorized);
    }
    let header = jsonwebtoken::decode_header(id_token).map_err(|_| GoogleError::Unauthorized)?;
    if header.alg != jsonwebtoken::Algorithm::RS256 {
        return Err(GoogleError::Unauthorized);
    }
    let keys = jwks
        .get("keys")
        .and_then(|k| k.as_array())
        .ok_or(GoogleError::Protocol)?;
    // kid is mandatory: never fall back to "the only key" (key-confusion).
    let kid = header.kid.as_deref().ok_or(GoogleError::Unauthorized)?;
    let key = keys
        .iter()
        .find(|k| k.get("kid").and_then(|x| x.as_str()) == Some(kid))
        .ok_or(GoogleError::Unauthorized)?;
    if key.get("kty").and_then(|x| x.as_str()) != Some("RSA") {
        return Err(GoogleError::Unauthorized);
    }
    if let Some(alg) = key.get("alg").and_then(|x| x.as_str()) {
        if alg != "RS256" {
            return Err(GoogleError::Unauthorized);
        }
    }
    if let Some(use_) = key.get("use").and_then(|x| x.as_str()) {
        if use_ != "sig" {
            return Err(GoogleError::Unauthorized);
        }
    }
    let n = key
        .get("n")
        .and_then(|x| x.as_str())
        .ok_or(GoogleError::Protocol)?;
    let e = key
        .get("e")
        .and_then(|x| x.as_str())
        .ok_or(GoogleError::Protocol)?;
    let decoding_key =
        jsonwebtoken::DecodingKey::from_rsa_components(n, e).map_err(|_| GoogleError::Protocol)?;
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_audience(&[client_id]);
    validation.set_issuer(&["https://accounts.google.com", "accounts.google.com"]);
    validation.validate_exp = true;
    // Zero leeway (spec allows a few minutes; stricter is fail-closed).
    validation.leeway = 0;
    let data = jsonwebtoken::decode::<serde_json::Value>(id_token, &decoding_key, &validation)
        .map_err(|_| GoogleError::Unauthorized)?;
    let claims = data.claims;

    // Multi-audience tokens must name us as the authorized party.
    if let Some(aud) = claims.get("aud") {
        if let Some(arr) = aud.as_array() {
            if arr.len() > 1 && claims.get("azp").and_then(|x| x.as_str()) != Some(client_id) {
                return Err(GoogleError::Unauthorized);
            }
        }
    }

    // Nonce binding (OIDC Core: sent ⇒ present ⇒ MUST match).
    if claims.get("nonce").and_then(|x| x.as_str()) != Some(expected_nonce) {
        return Err(GoogleError::Unauthorized);
    }

    let now = chrono::Utc::now().timestamp();
    // Freshness: iat within [now-15min, now+skew].
    let iat = claims
        .get("iat")
        .and_then(|x| x.as_i64())
        .ok_or(GoogleError::Unauthorized)?;
    if iat < now.saturating_sub(MAX_ID_TOKEN_AGE_SECS) || iat > now.saturating_add(CLOCK_SKEW_SECS)
    {
        return Err(GoogleError::Unauthorized);
    }
    // Fresh authentication: auth_time REQUIRED (max_age=0 was sent) and inside
    // the flow window. A stale SSO-session token can never mint a session.
    let auth_time = claims
        .get("auth_time")
        .and_then(|x| x.as_i64())
        .ok_or(GoogleError::Unauthorized)?;
    if auth_time < earliest_auth_time || auth_time > now.saturating_add(CLOCK_SKEW_SECS) {
        return Err(GoogleError::Unauthorized);
    }

    // at_hash: validate whenever the token carries it and we hold the access
    // token (detects access-token/ID-token mix-ups).
    if let (Some(at_hash), Some(access)) =
        (claims.get("at_hash").and_then(|x| x.as_str()), access_token)
    {
        let mut hasher = Sha256::new();
        hasher.update(access.as_bytes());
        let digest = hasher.finalize();
        let expected = URL_SAFE_NO_PAD.encode(&digest[..digest.len() / 2]);
        if expected != at_hash {
            return Err(GoogleError::Unauthorized);
        }
    }

    let sub = claims
        .get("sub")
        .and_then(|x| x.as_str())
        .ok_or(GoogleError::Unauthorized)?;
    if sub.is_empty() || sub.len() > 255 {
        return Err(GoogleError::Unauthorized);
    }
    Ok(GoogleClaims {
        sub: sub.to_string(),
        email: claims
            .get("email")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
        name: claims
            .get("name")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
    })
}

/// Full server-side verify: fetch JWKS then verify. Convenience for handlers.
pub async fn verify_id_token_live(
    http: &reqwest::Client,
    cfg: &OAuthConfig,
    id_token: &str,
    expected_nonce: &str,
    access_token: Option<&str>,
    earliest_auth_time: i64,
) -> Result<GoogleClaims, GoogleError> {
    google_client_id(cfg)?;
    if expected_nonce.is_empty() {
        return Err(GoogleError::Unauthorized);
    }
    let jwks_url = cfg.google_jwks_url.clone();
    let jwks = fetch_jwks(http, &jwks_url).await?;
    verify_id_token(
        cfg,
        id_token,
        &jwks,
        expected_nonce,
        access_token,
        earliest_auth_time,
    )
}

/// Optional userinfo fetch (profile enrichment, not trust root).
pub async fn fetch_userinfo(
    http: &reqwest::Client,
    cfg: &OAuthConfig,
    access_token: &str,
) -> Result<GoogleClaims, GoogleError> {
    if access_token.trim().is_empty() || access_token.len() > 4096 {
        return Err(GoogleError::Unauthorized);
    }
    let url = cfg.google_userinfo_url.clone();
    match provider_http::get_json(http, &url, Some(access_token), &[], MAX_PROVIDER_BODY).await {
        Ok((_status, v)) => Ok(GoogleClaims {
            sub: v
                .get("sub")
                .and_then(|x| x.as_str())
                .ok_or(GoogleError::Protocol)?
                .to_string(),
            email: v
                .get("email")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string()),
            name: v
                .get("name")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string()),
        }),
        Err(HttpError::Transport) => Err(GoogleError::Transport),
        Err(HttpError::BadStatus(401) | HttpError::BadStatus(403)) => {
            Err(GoogleError::Unauthorized)
        }
        Err(_) => Err(GoogleError::Protocol),
    }
}

#[cfg(test)]
#[allow(clippy::await_holding_lock)]
// Intentional: test_sync::lock() serializes tests sharing global provider
// state and MUST be held across await (that is its whole purpose). Test-only.
mod tests {
    use super::*;
    use crate::test_sync;
    use axum::{
        http::StatusCode,
        response::IntoResponse,
        routing::{get, post},
        Json, Router,
    };

    #[test]
    fn pkce_pair_shape() {
        let _guard = test_sync::lock();
        let (v, c) = pkce_pair();
        assert_eq!(v.len(), 64);
        assert!(v.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-'));
        assert_eq!(c.len(), 43); // base64url-nopad SHA256
                                 // Challenge must match verifier.
        let mut h = Sha256::new();
        h.update(v.as_bytes());
        assert_eq!(c, URL_SAFE_NO_PAD.encode(h.finalize()));
    }

    #[test]
    fn redirect_uri_validation() {
        let _guard = test_sync::lock();
        assert!(validate_redirect_uri("http://127.0.0.1:51004/oauth2redirect").is_ok());
        assert!(validate_redirect_uri("http://localhost:8080/cb").is_ok());
        assert!(validate_redirect_uri("https://example.com/cb").is_ok());
        assert!(validate_redirect_uri("http://evil.com/cb").is_err());
        assert!(validate_redirect_uri("urn:ietf:wg:oauth:2.0:oob").is_err());
        assert!(validate_redirect_uri("").is_err());
        assert!(validate_redirect_uri("http://127.0.0.1:1/x\ny").is_err());
    }

    #[test]
    fn flow_take_is_one_shot() {
        let _guard = test_sync::lock();
        clear_flows();
        let epoch = chrono::Utc::now().timestamp();
        prune_and_register(
            "s1".to_string(),
            "v1".to_string(),
            "http://127.0.0.1:1/cb".to_string(),
            "n1".to_string(),
            epoch,
        );
        let (v, _, n, e) = take_flow("s1").expect("first take ok");
        assert_eq!(v, "v1");
        assert_eq!(n, "n1");
        assert_eq!(e, epoch);
        assert_eq!(take_flow("s1").unwrap_err(), GoogleError::BadState);
    }

    // Mock Google: token exchange + JWKS signed by ephemeral RSA key.
    // Mock ID tokens carry nonce "mock-nonce-1", fresh auth_time/iat, and a
    // CORRECT at_hash for access token "ya29.mock".
    struct MockGoogle {
        base: String,
        jwks: serde_json::Value,
        signing_pem: Vec<u8>,
    }

    fn at_hash_for(access_token: &str) -> String {
        let mut h = Sha256::new();
        h.update(access_token.as_bytes());
        let digest = h.finalize();
        URL_SAFE_NO_PAD.encode(&digest[..digest.len() / 2])
    }

    async fn mock_google() -> (MockGoogle, tokio::task::JoinHandle<()>) {
        use rand::SeedableRng;
        use rsa::traits::PublicKeyParts;
        let mut rng = rand::rngs::StdRng::seed_from_u64(0xA16_0A07);
        let privkey = rsa::RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let pubkey = rsa::RsaPublicKey::from(&privkey);
        let n = URL_SAFE_NO_PAD.encode(pubkey.n().to_bytes_be());
        let e = URL_SAFE_NO_PAD.encode(pubkey.e().to_bytes_be());
        let jwks = serde_json::json!({"keys": [{
            "kty": "RSA", "kid": "test-key-1", "use": "sig", "alg": "RS256", "n": n, "e": e
        }]});
        let pem_doc =
            rsa::pkcs8::EncodePrivateKey::to_pkcs8_pem(&privkey, rsa::pkcs8::LineEnding::LF)
                .unwrap();
        let signing_pem = pem_doc.as_bytes().to_vec();

        let jwks_clone = jwks.clone();
        let pem_for_route = signing_pem.clone();
        let app = Router::new()
            .route(
                "/token",
                post(move |axum::Form(body): axum::Form<std::collections::HashMap<String, String>>| {
                    let pem = pem_for_route.clone();
                    async move {
                        let code = body.get("code").map(|s| s.as_str()).unwrap_or("");
                        if code != "mock-auth-code" {
                            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "invalid_grant"}))).into_response();
                        }
                        // Sign a Google-shaped ID token (nonce/auth_time/at_hash included,
                        // as a max_age=0 flow would produce).
                        let now = chrono::Utc::now().timestamp();
                        let claims = serde_json::json!({
                            "iss": "https://accounts.google.com",
                            "sub": "google-sub-123",
                            "aud": "mock-google-client-id",
                            "exp": now + 3600,
                            "iat": now,
                            "auth_time": now,
                            "nonce": "mock-nonce-1",
                            "at_hash": at_hash_for("ya29.mock"),
                            "email": "dev@example.com",
                            "name": "Dev User"
                        });
                        let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
                        header.kid = Some("test-key-1".to_string());
                        let enc = jsonwebtoken::EncodingKey::from_rsa_pem(&pem).unwrap();
                        let id_token = jsonwebtoken::encode(&header, &claims, &enc).unwrap();
                        (StatusCode::OK, Json(serde_json::json!({
                            "access_token": "ya29.mock",
                            "expires_in": 3600,
                            "token_type": "Bearer",
                            "id_token": id_token
                        }))).into_response()
                    }
                }),
            )
            .route("/jwks", get(move || {
                let j = jwks_clone.clone();
                async move { (StatusCode::OK, Json(j)).into_response() }
            }))
            .route(
                "/userinfo",
                get(|headers: axum::http::HeaderMap| async move {
                    let ok = headers.get("authorization").and_then(|v| v.to_str().ok()) == Some("Bearer ya29.mock");
                    if ok {
                        (StatusCode::OK, Json(serde_json::json!({"sub": "google-sub-123", "email": "dev@example.com", "name": "Dev User"}))).into_response()
                    } else {
                        (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "invalid_token"}))).into_response()
                    }
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (
            MockGoogle {
                base: format!("http://{addr}"),
                jwks,
                signing_pem,
            },
            handle,
        )
    }

    /// Sign an arbitrary claims object with the mock key (adversarial tokens).
    fn sign_mock_token(
        mock: &MockGoogle,
        claims: serde_json::Value,
        kid: Option<&str>,
        alg: jsonwebtoken::Algorithm,
    ) -> String {
        let mut header = jsonwebtoken::Header::new(alg);
        header.kid = kid.map(|s| s.to_string());
        let enc = if alg == jsonwebtoken::Algorithm::HS256 {
            jsonwebtoken::EncodingKey::from_secret(b"mock-hmac-secret")
        } else {
            jsonwebtoken::EncodingKey::from_rsa_pem(&mock.signing_pem).unwrap()
        };
        jsonwebtoken::encode(&header, &claims, &enc).unwrap()
    }

    fn fresh_claims(nonce: Option<&str>) -> serde_json::Value {
        let now = chrono::Utc::now().timestamp();
        let mut c = serde_json::json!({
            "iss": "https://accounts.google.com",
            "sub": "google-sub-123",
            "aud": "mock-google-client-id",
            "exp": now + 3600,
            "iat": now,
            "auth_time": now,
            "at_hash": at_hash_for("ya29.mock"),
            "email": "dev@example.com"
        });
        if let Some(n) = nonce {
            c["nonce"] = serde_json::Value::String(n.to_string());
        }
        c
    }

    fn test_cfg(base: &str) -> OAuthConfig {
        OAuthConfig {
            google_client_id: Some("mock-google-client-id".to_string()),
            google_client_secret: None,
            google_auth_url: format!("{base}/auth"),
            google_token_url: format!("{base}/token"),
            google_userinfo_url: format!("{base}/userinfo"),
            google_jwks_url: format!("{base}/jwks"),
            ..OAuthConfig::disabled()
        }
    }

    fn http() -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn auth_url_registers_flow() {
        let _guard = test_sync::lock();
        clear_flows();
        let cfg = test_cfg("http://127.0.0.1:9");
        // No redirect override → pinned default; nonce + max_age present.
        let (url, state) = build_auth_url(&cfg, None, None).unwrap();
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("access_type=offline"));
        assert!(url.contains("mock-google-client-id"));
        assert!(url.contains("nonce="));
        assert!(url.contains("max_age=0"));
        assert!(url.contains("127.0.0.1"));
        // Exact pinned echo accepted.
        let pinned = OAuthConfig::google_redirect_uri();
        assert!(build_auth_url(&cfg, Some(&pinned), None).is_ok());
        // Anything else rejected (login-CSRF prevention).
        assert_eq!(
            build_auth_url(&cfg, Some("https://evil.example/cb"), None).unwrap_err(),
            GoogleError::BadRedirectUri
        );
        assert_eq!(
            build_auth_url(&cfg, Some("http://127.0.0.1:9999/other"), None).unwrap_err(),
            GoogleError::BadRedirectUri
        );
        // Flow registered and retrievable.
        assert!(take_flow(&state).is_ok());
    }

    #[tokio::test]
    async fn exchange_verify_userinfo_roundtrip() {
        let _guard = test_sync::lock();
        clear_flows();
        let (mock, _h) = mock_google().await;
        let cfg = test_cfg(&mock.base);
        let http = http();
        // Register flow as build_auth_url would.
        let state = new_state();
        let epoch = chrono::Utc::now().timestamp();
        prune_and_register(
            state.clone(),
            "mock-verifier".to_string(),
            "http://127.0.0.1:1/cb".to_string(),
            "mock-nonce-1".to_string(),
            epoch,
        );
        let (tokens, nonce, created) = exchange_code(&http, &cfg, "mock-auth-code", &state)
            .await
            .unwrap();
        assert_eq!(nonce, "mock-nonce-1");
        assert_eq!(created, epoch);
        assert!(!tokens.id_token.is_empty());
        // Verify via live JWKS fetch (nonce + at_hash + auth_time enforced).
        let claims = verify_id_token_live(
            &http,
            &cfg,
            &tokens.id_token,
            &nonce,
            tokens.access_token.as_deref(),
            created.saturating_sub(120),
        )
        .await
        .unwrap();
        assert_eq!(claims.sub, "google-sub-123");
        assert_eq!(claims.email.as_deref(), Some("dev@example.com"));
        // Userinfo enrichment.
        let info = fetch_userinfo(&http, &cfg, tokens.access_token.as_deref().unwrap())
            .await
            .unwrap();
        assert_eq!(info.sub, "google-sub-123");
        // State is one-shot: reuse fails.
        assert!(exchange_code(&http, &cfg, "mock-auth-code", &state)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn proves_ask_on_wrong_audience() {
        let _guard = test_sync::lock();
        let (mock, _h) = mock_google().await;
        // Same JWKS/token, but different client_id → aud mismatch.
        let mut cfg = test_cfg(&mock.base);
        cfg.google_client_id = Some("other-client-id".to_string());
        let http = http();
        let state = new_state();
        let epoch = chrono::Utc::now().timestamp();
        prune_and_register(
            state.clone(),
            "v".to_string(),
            "http://127.0.0.1:1/cb".to_string(),
            "mock-nonce-1".to_string(),
            epoch,
        );
        let (tokens, _, _) = exchange_code(&http, &cfg, "mock-auth-code", &state)
            .await
            .unwrap();
        let now = chrono::Utc::now().timestamp();
        assert_eq!(
            verify_id_token(
                &cfg,
                &tokens.id_token,
                &mock.jwks,
                "mock-nonce-1",
                None,
                now - 900
            )
            .unwrap_err(),
            GoogleError::Unauthorized
        );
    }

    #[tokio::test]
    async fn proves_ask_on_adversarial_tokens() {
        let _guard = test_sync::lock();
        let (mock, _h) = mock_google().await;
        let cfg = test_cfg(&mock.base);
        let now = chrono::Utc::now().timestamp();
        let earliest = now - 900;
        let kid = Some("test-key-1");
        let rs256 = jsonwebtoken::Algorithm::RS256;
        // Baseline valid.
        let good = sign_mock_token(&mock, fresh_claims(Some("mock-nonce-1")), kid, rs256);
        assert!(verify_id_token(
            &cfg,
            &good,
            &mock.jwks,
            "mock-nonce-1",
            Some("ya29.mock"),
            earliest
        )
        .is_ok());
        // Missing nonce claim.
        let no_nonce = sign_mock_token(&mock, fresh_claims(None), kid, rs256);
        assert!(
            verify_id_token(&cfg, &no_nonce, &mock.jwks, "mock-nonce-1", None, earliest).is_err()
        );
        // Wrong nonce.
        let wrong_nonce = sign_mock_token(&mock, fresh_claims(Some("other-nonce")), kid, rs256);
        assert!(verify_id_token(
            &cfg,
            &wrong_nonce,
            &mock.jwks,
            "mock-nonce-1",
            None,
            earliest
        )
        .is_err());
        // Empty expected nonce (programmer error → fail closed).
        assert!(verify_id_token(&cfg, &good, &mock.jwks, "", None, earliest).is_err());
        // Stale iat (2h old, still unexpired).
        let mut stale = fresh_claims(Some("mock-nonce-1"));
        stale["iat"] = serde_json::json!(now - 7200);
        stale["auth_time"] = serde_json::json!(now - 7200);
        stale["exp"] = serde_json::json!(now + 3600);
        let stale_tok = sign_mock_token(&mock, stale, kid, rs256);
        assert!(
            verify_id_token(&cfg, &stale_tok, &mock.jwks, "mock-nonce-1", None, earliest).is_err()
        );
        // Missing auth_time.
        let mut no_auth = fresh_claims(Some("mock-nonce-1"));
        no_auth.as_object_mut().unwrap().remove("auth_time");
        let no_auth_tok = sign_mock_token(&mock, no_auth, kid, rs256);
        assert!(verify_id_token(
            &cfg,
            &no_auth_tok,
            &mock.jwks,
            "mock-nonce-1",
            None,
            earliest
        )
        .is_err());
        // Stale auth_time (old SSO session, fresh iat).
        let mut old_auth = fresh_claims(Some("mock-nonce-1"));
        old_auth["auth_time"] = serde_json::json!(now - 7200);
        let old_auth_tok = sign_mock_token(&mock, old_auth, kid, rs256);
        assert!(verify_id_token(
            &cfg,
            &old_auth_tok,
            &mock.jwks,
            "mock-nonce-1",
            None,
            earliest
        )
        .is_err());
        // Wrong at_hash with access token known.
        let mut bad_ath = fresh_claims(Some("mock-nonce-1"));
        bad_ath["at_hash"] = serde_json::json!("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
        let bad_ath_tok = sign_mock_token(&mock, bad_ath, kid, rs256);
        assert!(verify_id_token(
            &cfg,
            &bad_ath_tok,
            &mock.jwks,
            "mock-nonce-1",
            Some("ya29.mock"),
            earliest
        )
        .is_err());
        // Missing kid.
        let no_kid = sign_mock_token(&mock, fresh_claims(Some("mock-nonce-1")), None, rs256);
        assert!(
            verify_id_token(&cfg, &no_kid, &mock.jwks, "mock-nonce-1", None, earliest).is_err()
        );
        // Unknown kid.
        let bad_kid = sign_mock_token(
            &mock,
            fresh_claims(Some("mock-nonce-1")),
            Some("evil-key"),
            rs256,
        );
        assert!(
            verify_id_token(&cfg, &bad_kid, &mock.jwks, "mock-nonce-1", None, earliest).is_err()
        );
        // alg confusion: HS256 token (same claims) must be rejected.
        let hs = sign_mock_token(
            &mock,
            fresh_claims(Some("mock-nonce-1")),
            None,
            jsonwebtoken::Algorithm::HS256,
        );
        assert!(verify_id_token(&cfg, &hs, &mock.jwks, "mock-nonce-1", None, earliest).is_err());
        // Multi-audience without azp.
        let mut multi = fresh_claims(Some("mock-nonce-1"));
        multi["aud"] = serde_json::json!(["mock-google-client-id", "other-aud"]);
        let multi_tok = sign_mock_token(&mock, multi, kid, rs256);
        assert!(
            verify_id_token(&cfg, &multi_tok, &mock.jwks, "mock-nonce-1", None, earliest).is_err()
        );
        // Oversize sub.
        let mut big_sub = fresh_claims(Some("mock-nonce-1"));
        big_sub["sub"] = serde_json::json!("x".repeat(300));
        let big_tok = sign_mock_token(&mock, big_sub, kid, rs256);
        assert!(
            verify_id_token(&cfg, &big_tok, &mock.jwks, "mock-nonce-1", None, earliest).is_err()
        );
    }

    #[tokio::test]
    async fn proves_ask_on_jwk_alg_mismatch() {
        let _guard = test_sync::lock();
        let (mock, _h) = mock_google().await;
        let cfg = test_cfg(&mock.base);
        let now = chrono::Utc::now().timestamp();
        // Same key material, but JWKS declares a different alg.
        let alg = jsonwebtoken::Algorithm::RS256;
        let mut jwks = mock.jwks.clone();
        jwks["keys"][0]["alg"] = serde_json::json!("HS256");
        let tok = sign_mock_token(
            &mock,
            fresh_claims(Some("mock-nonce-1")),
            Some("test-key-1"),
            alg,
        );
        assert!(verify_id_token(&cfg, &tok, &jwks, "mock-nonce-1", None, now - 900).is_err());
        // ... and use != sig.
        let mut jwks2 = mock.jwks.clone();
        jwks2["keys"][0]["use"] = serde_json::json!("enc");
        assert!(verify_id_token(&cfg, &tok, &jwks2, "mock-nonce-1", None, now - 900).is_err());
    }

    #[tokio::test]
    async fn proves_ask_when_not_configured() {
        let _guard = test_sync::lock();
        let cfg = OAuthConfig::disabled();
        assert_eq!(
            build_auth_url(&cfg, None, None).unwrap_err(),
            GoogleError::NotConfigured
        );
        assert_eq!(take_flow("nope").unwrap_err(), GoogleError::BadState);
    }
}
