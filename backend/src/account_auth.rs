//! Algorithco account access-token verification.
//!
//! The account service is discovered from its configured issuer. Access
//! tokens are ES256 JWTs with `typ=at+jwt`; keys are selected by `kid` from a
//! bounded, stale-if-error JWKS cache. Authentication always fails closed.

use axum::http::{header::AUTHORIZATION, HeaderMap};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use reqwest::{header::CACHE_CONTROL, Client, Url};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

const MAX_TOKEN_BYTES: usize = 16 * 1024;
const MAX_DISCOVERY_BYTES: usize = 64 * 1024;
const MAX_JWKS_BYTES: usize = 256 * 1024;
const MAX_JWKS_KEYS: usize = 64;
const DEFAULT_JWKS_TTL_SECS: u64 = 60;
const MAX_JWKS_TTL_SECS: u64 = 600;
const DEFAULT_STALE_IF_ERROR_SECS: u64 = 300;
const MAX_STALE_IF_ERROR_SECS: u64 = 300;
const UNKNOWN_KID_COOLDOWN_SECS: u64 = 30;
const ACCOUNT_HTTP_TIMEOUT_SECS: u64 = 5;
const DEFAULT_CLOCK_SKEW_SECS: u64 = 5;
const MAX_CLOCK_SKEW_SECS: u64 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMode {
    Legacy,
    Account,
}

impl AuthMode {
    pub fn from_env() -> Result<Self, String> {
        match std::env::var("GUARD_AUTH_MODE")
            .unwrap_or_else(|_| "legacy".to_string())
            .as_str()
        {
            "legacy" => Ok(Self::Legacy),
            "account" => Ok(Self::Account),
            _ => Err("GUARD_AUTH_MODE must be exactly legacy or account".to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AccountAuthConfig {
    issuer: String,
    audiences: Vec<String>,
    clock_skew_secs: u64,
}

impl AccountAuthConfig {
    pub fn from_env() -> Result<Self, String> {
        let issuer = std::env::var("GUARD_ACCOUNT_ISSUER")
            .map_err(|_| "GUARD_ACCOUNT_ISSUER is required in account mode".to_string())?;
        let audiences = std::env::var("GUARD_ACCOUNT_AUDIENCES")
            .map_err(|_| "GUARD_ACCOUNT_AUDIENCES is required in account mode".to_string())?;
        let skew = std::env::var("GUARD_ACCOUNT_CLOCK_SKEW_SECS")
            .unwrap_or_else(|_| DEFAULT_CLOCK_SKEW_SECS.to_string());
        Self::parse(&issuer, &audiences, &skew)
    }

    fn parse(issuer: &str, audiences: &str, skew: &str) -> Result<Self, String> {
        if issuer.trim() != issuer || issuer.is_empty() {
            return Err(
                "GUARD_ACCOUNT_ISSUER must be a non-empty URL without surrounding whitespace"
                    .to_string(),
            );
        }
        let issuer_url = parse_trusted_url(issuer, "GUARD_ACCOUNT_ISSUER")?;
        if issuer_url.query().is_some() || issuer_url.fragment().is_some() {
            return Err("GUARD_ACCOUNT_ISSUER must not contain a query or fragment".to_string());
        }
        if issuer_url.path() != "/" && !issuer_url.path().is_empty() {
            return Err("GUARD_ACCOUNT_ISSUER must not contain a path".to_string());
        }

        if audiences.trim() != audiences || audiences.is_empty() {
            return Err(
                "GUARD_ACCOUNT_AUDIENCES must be a non-empty comma-separated exact-match list"
                    .to_string(),
            );
        }
        let mut seen = HashSet::new();
        let mut parsed = Vec::new();
        for item in audiences.split(',') {
            if item.is_empty() || item.trim() != item || item.chars().any(char::is_whitespace) {
                return Err(
                    "GUARD_ACCOUNT_AUDIENCES contains an empty or whitespace-bearing item"
                        .to_string(),
                );
            }
            if item.contains('*') || item.contains('?') {
                return Err("GUARD_ACCOUNT_AUDIENCES does not allow wildcards".to_string());
            }
            if !seen.insert(item.to_string()) {
                return Err("GUARD_ACCOUNT_AUDIENCES contains a duplicate item".to_string());
            }
            parsed.push(item.to_string());
        }
        if parsed.is_empty() {
            return Err("GUARD_ACCOUNT_AUDIENCES must contain at least one audience".to_string());
        }

        let clock_skew_secs = skew.parse::<u64>().map_err(|_| {
            "GUARD_ACCOUNT_CLOCK_SKEW_SECS must be an integer from 0 through 300".to_string()
        })?;
        if clock_skew_secs > MAX_CLOCK_SKEW_SECS {
            return Err(
                "GUARD_ACCOUNT_CLOCK_SKEW_SECS must be an integer from 0 through 300".to_string(),
            );
        }

        Ok(Self {
            issuer: issuer_url.as_str().trim_end_matches('/').to_string(),
            audiences: parsed,
            clock_skew_secs,
        })
    }

    fn discovery_url(&self) -> Result<Url, String> {
        let mut url = Url::parse(&self.issuer)
            .map_err(|_| "GUARD_ACCOUNT_ISSUER is not a valid URL".to_string())?;
        url.set_path("/.well-known/openid-configuration");
        url.set_query(None);
        url.set_fragment(None);
        Ok(url)
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct CompactEntitlement {
    pub p: String,
    pub plan: String,
    pub exp: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct AccountClaims {
    pub iss: String,
    pub sub: String,
    pub aud: Audience,
    pub exp: i64,
    pub iat: i64,
    pub nbf: Option<i64>,
    pub client_id: String,
    pub scope: String,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub entitlements: Option<Vec<CompactEntitlement>>,
    pub nonce: Option<serde_json::Value>,
    pub at_hash: Option<serde_json::Value>,
    pub c_hash: Option<serde_json::Value>,
    pub s_hash: Option<serde_json::Value>,
    pub auth_time: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Audience {
    One(String),
    Many(Vec<String>),
}

impl Audience {
    fn contains(&self, value: &str) -> bool {
        match self {
            Self::One(item) => item == value,
            Self::Many(items) => items.iter().any(|item| item == value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountAuthError {
    Missing,
    Invalid,
    Expired,
}

impl std::fmt::Display for AccountAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing | Self::Invalid => write!(f, "unauthorized"),
            Self::Expired => write!(f, "credential expired"),
        }
    }
}

impl std::error::Error for AccountAuthError {}

#[derive(Clone)]
pub struct AccountTokenVerifier {
    config: AccountAuthConfig,
    client: Client,
    jwks_uri: Url,
    cache: Arc<Mutex<JwksCache>>,
}

#[derive(Default)]
struct JwksCache {
    keys: HashMap<String, EcJwk>,
    fresh_until: Option<Instant>,
    stale_until: Option<Instant>,
    last_unknown_refresh: Option<Instant>,
}

#[derive(Debug, Clone, Deserialize)]
struct DiscoveryDocument {
    issuer: String,
    jwks_uri: String,
}

#[derive(Debug, Deserialize)]
struct JwksDocument {
    keys: Vec<EcJwk>,
}

#[derive(Debug, Clone, Deserialize)]
struct EcJwk {
    kid: String,
    kty: String,
    crv: String,
    alg: String,
    #[serde(default, rename = "use")]
    use_: Option<String>,
    x: String,
    y: String,
}

impl AccountTokenVerifier {
    pub async fn discover(config: AccountAuthConfig, client: Client) -> Result<Self, String> {
        let response = client
            .get(config.discovery_url()?)
            .timeout(Duration::from_secs(ACCOUNT_HTTP_TIMEOUT_SECS))
            .send()
            .await
            .map_err(|_| "account discovery request failed".to_string())?;
        if !response.status().is_success() {
            return Err(format!(
                "account discovery returned HTTP {}",
                response.status()
            ));
        }
        let body = read_limited(response, MAX_DISCOVERY_BYTES)
            .await
            .map_err(|_| "account discovery response was invalid or too large".to_string())?;
        let discovery: DiscoveryDocument = serde_json::from_slice(&body)
            .map_err(|_| "account discovery response was invalid JSON".to_string())?;
        if discovery.issuer != config.issuer {
            return Err("account discovery issuer did not match GUARD_ACCOUNT_ISSUER".to_string());
        }
        let jwks_uri = parse_trusted_url(&discovery.jwks_uri, "discovered jwks_uri")?;
        Ok(Self {
            config,
            client,
            jwks_uri,
            cache: Arc::new(Mutex::new(JwksCache::default())),
        })
    }

    pub async fn verify_headers(
        &self,
        headers: &HeaderMap,
    ) -> Result<AccountClaims, AccountAuthError> {
        let mut values = headers.get_all(AUTHORIZATION).iter();
        let value = values.next().ok_or(AccountAuthError::Missing)?;
        if values.next().is_some() {
            return Err(AccountAuthError::Invalid);
        }
        let header = value.to_str().map_err(|_| AccountAuthError::Invalid)?;
        let token = header
            .strip_prefix("Bearer ")
            .ok_or(AccountAuthError::Missing)?;
        self.verify_token(token).await
    }

    pub async fn verify_token(&self, token: &str) -> Result<AccountClaims, AccountAuthError> {
        if token.is_empty()
            || token.len() > MAX_TOKEN_BYTES
            || token.bytes().any(|byte| byte.is_ascii_whitespace())
        {
            return Err(AccountAuthError::Invalid);
        }
        let header = decode_header(token).map_err(|_| AccountAuthError::Invalid)?;
        if header.alg != Algorithm::ES256 || header.typ.as_deref() != Some("at+jwt") {
            return Err(AccountAuthError::Invalid);
        }
        let kid = header
            .kid
            .as_deref()
            .filter(|value| !value.is_empty())
            .ok_or(AccountAuthError::Invalid)?;
        let key = self.key_for(kid).await?;
        let decoding_key = DecodingKey::from_ec_components(&key.x, &key.y)
            .map_err(|_| AccountAuthError::Invalid)?;

        let mut validation = Validation::new(Algorithm::ES256);
        validation.leeway = self.config.clock_skew_secs;
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.set_issuer(&[self.config.issuer.as_str()]);
        validation.set_audience(&self.config.audiences);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        let claims = match decode::<AccountClaims>(token, &decoding_key, &validation) {
            Ok(data) => data.claims,
            Err(error) => {
                use jsonwebtoken::errors::ErrorKind;
                return match error.kind() {
                    ErrorKind::ExpiredSignature => Err(AccountAuthError::Expired),
                    _ => Err(AccountAuthError::Invalid),
                };
            }
        };

        let now = chrono::Utc::now().timestamp();
        let latest_iat = now.saturating_add(self.config.clock_skew_secs as i64);
        if claims.sub.is_empty()
            || claims.iat < 0
            || claims.iat > latest_iat
            || claims.scope.trim().is_empty()
            || claims.client_id.is_empty()
            || !self
                .config
                .audiences
                .iter()
                .any(|allowed| allowed == &claims.client_id)
            || !claims.aud.contains(&claims.client_id)
            || claims.nonce.is_some()
            || claims.at_hash.is_some()
            || claims.c_hash.is_some()
            || claims.s_hash.is_some()
            || claims.auth_time.is_some()
        {
            return Err(AccountAuthError::Invalid);
        }
        Ok(claims)
    }

    async fn key_for(&self, kid: &str) -> Result<EcJwk, AccountAuthError> {
        let mut cache = self.cache.lock().await;
        let now = Instant::now();
        let fresh = cache.fresh_until.is_some_and(|until| now <= until);
        let cached_key = cache.keys.get(kid).cloned();
        if fresh && cached_key.is_some() {
            return cached_key.ok_or(AccountAuthError::Invalid);
        }
        if cached_key.is_none() && !cache.keys.is_empty() {
            if cache.last_unknown_refresh.is_some_and(|last| {
                now.duration_since(last) < Duration::from_secs(UNKNOWN_KID_COOLDOWN_SECS)
            }) {
                return Err(AccountAuthError::Invalid);
            }
            cache.last_unknown_refresh = Some(now);
        }

        match self.fetch_jwks().await {
            Ok(fetched) => {
                cache.keys = fetched.keys;
                cache.fresh_until = Some(now + fetched.ttl);
                cache.stale_until = Some(now + fetched.ttl + fetched.stale_if_error);
                cache
                    .keys
                    .get(kid)
                    .cloned()
                    .ok_or(AccountAuthError::Invalid)
            }
            Err(()) => {
                let stale_allowed = cache.stale_until.is_some_and(|until| now <= until);
                if stale_allowed {
                    cache
                        .keys
                        .get(kid)
                        .cloned()
                        .ok_or(AccountAuthError::Invalid)
                } else {
                    Err(AccountAuthError::Invalid)
                }
            }
        }
    }

    async fn fetch_jwks(&self) -> Result<FetchedJwks, ()> {
        let response = self
            .client
            .get(self.jwks_uri.clone())
            .timeout(Duration::from_secs(ACCOUNT_HTTP_TIMEOUT_SECS))
            .send()
            .await
            .map_err(|_| ())?;
        if !response.status().is_success() {
            return Err(());
        }
        let cache_control = response
            .headers()
            .get(CACHE_CONTROL)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let body = read_limited(response, MAX_JWKS_BYTES)
            .await
            .map_err(|_| ())?;
        let document: JwksDocument = serde_json::from_slice(&body).map_err(|_| ())?;
        if document.keys.is_empty() || document.keys.len() > MAX_JWKS_KEYS {
            return Err(());
        }
        let mut keys = HashMap::new();
        for key in document.keys {
            if key.kid.is_empty()
                || key.kty != "EC"
                || key.crv != "P-256"
                || key.alg != "ES256"
                || key.use_.as_deref().is_some_and(|value| value != "sig")
                || key.x.is_empty()
                || key.y.is_empty()
                || keys.insert(key.kid.clone(), key).is_some()
            {
                return Err(());
            }
        }
        let (ttl, stale_if_error) = cache_durations(&cache_control);
        Ok(FetchedJwks {
            keys,
            ttl,
            stale_if_error,
        })
    }
}

struct FetchedJwks {
    keys: HashMap<String, EcJwk>,
    ttl: Duration,
    stale_if_error: Duration,
}

fn cache_durations(value: &str) -> (Duration, Duration) {
    let mut ttl = DEFAULT_JWKS_TTL_SECS;
    let mut stale = DEFAULT_STALE_IF_ERROR_SECS;
    for directive in value.split(',').map(str::trim) {
        if let Some(raw) = directive.strip_prefix("max-age=") {
            if let Ok(seconds) = raw.parse::<u64>() {
                ttl = seconds.min(MAX_JWKS_TTL_SECS);
            }
        }
        if let Some(raw) = directive.strip_prefix("stale-if-error=") {
            if let Ok(seconds) = raw.parse::<u64>() {
                stale = seconds.min(MAX_STALE_IF_ERROR_SECS);
            }
        }
    }
    (Duration::from_secs(ttl), Duration::from_secs(stale))
}

async fn read_limited(mut response: reqwest::Response, max: usize) -> Result<Vec<u8>, ()> {
    if response
        .content_length()
        .is_some_and(|length| length > max as u64)
    {
        return Err(());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| ())? {
        if bytes.len().saturating_add(chunk.len()) > max {
            return Err(());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn parse_trusted_url(raw: &str, name: &str) -> Result<Url, String> {
    let url = Url::parse(raw).map_err(|_| format!("{name} is not a valid URL"))?;
    let secure = url.scheme() == "https";
    let loopback_http = url.scheme() == "http"
        && url
            .host_str()
            .is_some_and(|host| host == "127.0.0.1" || host == "[::1]" || host == "::1");
    if !secure && !loopback_http {
        return Err(format!(
            "{name} must use HTTPS (HTTP is allowed only on loopback)"
        ));
    }
    if url.username() != "" || url.password().is_some() || url.host_str().is_none() {
        return Err(format!(
            "{name} must not contain credentials and must have a host"
        ));
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    use jsonwebtoken::{encode, EncodingKey, Header};
    use p256::{
        ecdsa::SigningKey,
        pkcs8::{EncodePrivateKey, LineEnding},
    };
    use rand_core::OsRng;
    use serde_json::{json, Value};
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Mutex as StdMutex,
    };

    struct TestKey {
        kid: String,
        encoding: EncodingKey,
        jwk: Value,
    }

    fn test_key(kid: &str) -> TestKey {
        let signing = SigningKey::random(&mut OsRng);
        let pem = signing.to_pkcs8_pem(LineEnding::LF).unwrap();
        let point = signing.verifying_key().to_encoded_point(false);
        TestKey {
            kid: kid.to_string(),
            encoding: EncodingKey::from_ec_pem(pem.as_bytes()).unwrap(),
            jwk: json!({
                "kid": kid,
                "kty": "EC",
                "crv": "P-256",
                "alg": "ES256",
                "use": "sig",
                "x": URL_SAFE_NO_PAD.encode(point.x().unwrap()),
                "y": URL_SAFE_NO_PAD.encode(point.y().unwrap()),
            }),
        }
    }

    fn claims(aud: Value) -> Value {
        let now = chrono::Utc::now().timestamp();
        json!({
            "iss": "http://127.0.0.1:1",
            "sub": "account-subject",
            "aud": aud,
            "exp": now + 600,
            "iat": now,
            "client_id": "guard-web",
            "scope": "profile email"
        })
    }

    #[derive(Clone)]
    struct MockState {
        issuer: String,
        keys: Arc<StdMutex<Vec<Value>>>,
        down: Arc<AtomicBool>,
        hits: Arc<AtomicUsize>,
        cache_control: String,
    }

    async fn discovery(State(state): State<MockState>) -> Json<Value> {
        Json(json!({"issuer": state.issuer, "jwks_uri": format!("{}/jwks", state.issuer)}))
    }

    async fn jwks(State(state): State<MockState>) -> impl IntoResponse {
        state.hits.fetch_add(1, Ordering::SeqCst);
        if state.down.load(Ordering::SeqCst) {
            return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
        let keys = state.keys.lock().unwrap().clone();
        (
            [(CACHE_CONTROL.as_str(), state.cache_control)],
            Json(json!({"keys": keys})),
        )
            .into_response()
    }

    async fn mock(keys: Vec<Value>, cache_control: &str) -> (AccountTokenVerifier, MockState) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let issuer = format!("http://{}", listener.local_addr().unwrap());
        let state = MockState {
            issuer: issuer.clone(),
            keys: Arc::new(StdMutex::new(keys)),
            down: Arc::new(AtomicBool::new(false)),
            hits: Arc::new(AtomicUsize::new(0)),
            cache_control: cache_control.to_string(),
        };
        let app = Router::new()
            .route("/.well-known/openid-configuration", get(discovery))
            .route("/jwks", get(jwks))
            .with_state(state.clone());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let config = AccountAuthConfig::parse(&issuer, "guard-web,guard-cli", "5").unwrap();
        let verifier = AccountTokenVerifier::discover(config, Client::new())
            .await
            .unwrap();
        (verifier, state)
    }

    fn token_for_issuer(key: &TestKey, issuer: &str, aud: Value) -> String {
        let mut value = claims(aud);
        value["iss"] = Value::String(issuer.to_string());
        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(key.kid.clone());
        header.typ = Some("at+jwt".to_string());
        encode(&header, &value, &key.encoding).unwrap()
    }

    #[test]
    fn account_audience_config_is_exact_and_required() {
        for invalid in [
            "",
            " ",
            "guard-*",
            "guard?web",
            "guard-web,",
            "guard-web, guard-cli",
            "guard-web,guard-web",
        ] {
            assert!(AccountAuthConfig::parse("https://auth.example.test", invalid, "5").is_err());
        }
        let config =
            AccountAuthConfig::parse("https://auth.example.test", "guard-web,guard-cli", "5")
                .unwrap();
        assert_eq!(config.audiences, ["guard-web", "guard-cli"]);
    }

    #[test]
    fn account_mode_startup_config_requires_audiences() {
        let _guard = crate::test_sync::lock();
        let old_issuer = std::env::var("GUARD_ACCOUNT_ISSUER").ok();
        let old_audiences = std::env::var("GUARD_ACCOUNT_AUDIENCES").ok();
        std::env::set_var("GUARD_ACCOUNT_ISSUER", "https://auth.example.test");
        std::env::remove_var("GUARD_ACCOUNT_AUDIENCES");
        assert!(AccountAuthConfig::from_env()
            .unwrap_err()
            .contains("GUARD_ACCOUNT_AUDIENCES is required"));
        match old_issuer {
            Some(value) => std::env::set_var("GUARD_ACCOUNT_ISSUER", value),
            None => std::env::remove_var("GUARD_ACCOUNT_ISSUER"),
        }
        match old_audiences {
            Some(value) => std::env::set_var("GUARD_ACCOUNT_AUDIENCES", value),
            None => std::env::remove_var("GUARD_ACCOUNT_AUDIENCES"),
        }
    }

    #[tokio::test]
    async fn valid_tokens_for_both_clients_and_array_audience_are_accepted() {
        let key = test_key("key-1");
        let (verifier, state) = mock(vec![key.jwk.clone()], "max-age=60").await;
        let web = token_for_issuer(&key, &state.issuer, json!("guard-web"));
        assert_eq!(
            verifier.verify_token(&web).await.unwrap().sub,
            "account-subject"
        );

        let mut cli_claims = claims(json!(["outside", "guard-cli"]));
        cli_claims["iss"] = json!(state.issuer);
        cli_claims["client_id"] = json!("guard-cli");
        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(key.kid.clone());
        header.typ = Some("at+jwt".to_string());
        let cli = encode(&header, &cli_claims, &key.encoding).unwrap();
        assert_eq!(
            verifier.verify_token(&cli).await.unwrap().sub,
            "account-subject"
        );
    }

    #[tokio::test]
    async fn proves_ask_on_time_issuer_audience_and_id_token_failures() {
        let key = test_key("key-1");
        let (verifier, state) = mock(vec![key.jwk.clone()], "max-age=60").await;
        let now = chrono::Utc::now().timestamp();
        for (name, change) in [
            ("expired", json!({"exp": now - 30})),
            ("not-yet-valid", json!({"nbf": now + 60})),
            ("future-iat", json!({"iat": now + 60})),
            ("wrong-issuer", json!({"iss": "https://other.example"})),
            ("wrong-audience", json!({"aud": "voice-web"})),
        ] {
            let mut value = claims(json!("guard-web"));
            value["iss"] = json!(state.issuer);
            for (key, item) in change.as_object().unwrap() {
                value[key] = item.clone();
            }
            let token =
                token_for_issuer(&key, value["iss"].as_str().unwrap(), value["aud"].clone());
            let token = if name == "expired" || name == "not-yet-valid" || name == "future-iat" {
                let mut header = Header::new(Algorithm::ES256);
                header.kid = Some(key.kid.clone());
                header.typ = Some("at+jwt".to_string());
                encode(&header, &value, &key.encoding).unwrap()
            } else {
                token
            };
            assert!(verifier.verify_token(&token).await.is_err(), "{name}");
        }

        let mut id_header = Header::new(Algorithm::ES256);
        id_header.kid = Some(key.kid.clone());
        let mut id_claims = claims(json!("guard-web"));
        id_claims["iss"] = json!(state.issuer);
        id_claims["nonce"] = json!("id-token-marker");
        let id_token = encode(&id_header, &id_claims, &key.encoding).unwrap();
        assert!(verifier.verify_token(&id_token).await.is_err());
    }

    #[tokio::test]
    async fn proves_ask_on_algorithm_confusion_tampering_and_bad_input() {
        let key = test_key("key-1");
        let (verifier, state) = mock(vec![key.jwk.clone()], "max-age=60").await;
        let valid = token_for_issuer(&key, &state.issuer, json!("guard-web"));
        let mut tampered = valid.clone().into_bytes();
        let payload = valid.find('.').unwrap() + 1;
        tampered[payload] = if tampered[payload] == b'a' {
            b'b'
        } else {
            b'a'
        };
        assert!(verifier
            .verify_token(std::str::from_utf8(&tampered).unwrap())
            .await
            .is_err());

        let mut hs_header = Header::new(Algorithm::HS256);
        hs_header.kid = Some(key.kid.clone());
        hs_header.typ = Some("at+jwt".to_string());
        let hs = encode(
            &hs_header,
            &{
                let mut value = claims(json!("guard-web"));
                value["iss"] = json!(state.issuer);
                value
            },
            &EncodingKey::from_secret(key.jwk.to_string().as_bytes()),
        )
        .unwrap();
        assert!(verifier.verify_token(&hs).await.is_err());

        let none_header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none","typ":"at+jwt","kid":"key-1"}"#);
        let none_payload = URL_SAFE_NO_PAD.encode(claims(json!("guard-web")).to_string());
        assert!(verifier
            .verify_token(&format!("{none_header}.{none_payload}."))
            .await
            .is_err());
        assert!(verifier.verify_token("garbage").await.is_err());
        assert!(verifier
            .verify_token(&"x".repeat(MAX_TOKEN_BYTES + 1))
            .await
            .is_err());

        let mut id_marker_claims = claims(json!("guard-web"));
        id_marker_claims["iss"] = json!(state.issuer);
        id_marker_claims["nonce"] = json!("id-token-marker");
        let mut access_header = Header::new(Algorithm::ES256);
        access_header.kid = Some(key.kid.clone());
        access_header.typ = Some("at+jwt".to_string());
        let disguised_id = encode(&access_header, &id_marker_claims, &key.encoding).unwrap();
        assert!(verifier.verify_token(&disguised_id).await.is_err());
    }

    #[tokio::test]
    async fn unknown_kid_rotation_and_outage_behavior_fail_closed() {
        let old = test_key("old");
        let new = test_key("new");
        let (verifier, state) = mock(vec![old.jwk.clone()], "max-age=0, stale-if-error=300").await;
        let old_token = token_for_issuer(&old, &state.issuer, json!("guard-web"));
        assert!(verifier.verify_token(&old_token).await.is_ok());

        *state.keys.lock().unwrap() = vec![old.jwk.clone(), new.jwk.clone()];
        let new_token = token_for_issuer(&new, &state.issuer, json!("guard-web"));
        assert!(verifier.verify_token(&new_token).await.is_ok());
        assert_eq!(state.hits.load(Ordering::SeqCst), 2);

        state.down.store(true, Ordering::SeqCst);
        assert!(verifier.verify_token(&old_token).await.is_ok());

        let (cold, cold_state) = mock(vec![old.jwk.clone()], "max-age=0").await;
        cold_state.down.store(true, Ordering::SeqCst);
        let cold_token = token_for_issuer(&old, &cold_state.issuer, json!("guard-web"));
        assert!(cold.verify_token(&cold_token).await.is_err());

        let unknown = test_key("unknown");
        let unknown_token = token_for_issuer(&unknown, &state.issuer, json!("guard-web"));
        assert!(verifier.verify_token(&unknown_token).await.is_err());
        assert!(verifier.verify_token(&unknown_token).await.is_err());
        assert_eq!(state.hits.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn proves_ask_on_missing_or_duplicate_authorization_headers() {
        let key = test_key("key-1");
        let (verifier, _state) = mock(vec![key.jwk], "max-age=60").await;
        assert_eq!(
            verifier
                .verify_headers(&HeaderMap::new())
                .await
                .unwrap_err(),
            AccountAuthError::Missing
        );
        let mut headers = HeaderMap::new();
        headers.append(AUTHORIZATION, "Bearer one".parse().unwrap());
        headers.append(AUTHORIZATION, "Bearer two".parse().unwrap());
        assert_eq!(
            verifier.verify_headers(&headers).await.unwrap_err(),
            AccountAuthError::Invalid
        );
    }
}
