//! Algorithco account authentication for Guard's native clients.
//!
//! OAuth is deliberately separate from the local policy path. Authentication
//! errors can disable cloud/team calls, but this crate has no dependency on or
//! ability to change allow/ask/deny decisions.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use rand::{rngs::OsRng, RngCore};
use reqwest::blocking::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use url::Url;

const CLIENT_ID: &str = "guard-cli";
const SCOPES: &str = "openid profile email offline_access";
const DEFAULT_ISSUER: &str = "https://auth.algorithco.com";
const HTTP_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_JSON_BYTES: usize = 256 * 1024;
const MAX_TOKEN_BYTES: usize = 16 * 1024;
const JWKS_CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const LOOPBACK_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const KEYRING_SERVICE: &str = "algorithco-guard";

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("account configuration is invalid: {0}")]
    Config(String),
    #[error("account service is unavailable")]
    Network,
    #[error("account service returned an invalid response")]
    InvalidResponse,
    #[error("account token verification failed")]
    InvalidToken,
    #[error("account authorization failed: {0}")]
    OAuth(String),
    #[error("account sign-in expired")]
    Expired,
    #[error("account sign-in was cancelled")]
    Cancelled,
    #[error("stored credentials are unavailable")]
    CredentialStore,
    #[error("stored session is no longer valid; run `algo login` again")]
    ReloginRequired,
    #[error("loopback callback failed")]
    Loopback,
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    issuer: Url,
    clock_skew_secs: u64,
}

impl AuthConfig {
    pub fn from_env() -> Result<Self, AuthError> {
        let issuer =
            std::env::var("ALGO_ACCOUNT_ISSUER").unwrap_or_else(|_| DEFAULT_ISSUER.to_string());
        let skew = std::env::var("ALGO_ACCOUNT_CLOCK_SKEW_SECS")
            .unwrap_or_else(|_| "5".to_string())
            .parse::<u64>()
            .map_err(|_| AuthError::Config("clock skew must be an integer".into()))?;
        Self::new(&issuer, skew)
    }

    pub fn new(issuer: &str, clock_skew_secs: u64) -> Result<Self, AuthError> {
        if clock_skew_secs > 60 {
            return Err(AuthError::Config(
                "clock skew must be between 0 and 60 seconds".into(),
            ));
        }
        let issuer = trusted_url(issuer, "issuer")?;
        if issuer.query().is_some() || issuer.fragment().is_some() {
            return Err(AuthError::Config(
                "issuer must not contain a query or fragment".into(),
            ));
        }
        Ok(Self {
            issuer,
            clock_skew_secs,
        })
    }

    pub fn issuer(&self) -> &str {
        self.issuer.as_str().trim_end_matches('/')
    }
}

fn trusted_url(raw: &str, name: &str) -> Result<Url, AuthError> {
    let url = Url::parse(raw)
        .map_err(|_| AuthError::Config(format!("{name} must be an absolute URL")))?;
    let loopback = matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "::1"));
    if url.scheme() != "https" && !(loopback && url.scheme() == "http") {
        return Err(AuthError::Config(format!(
            "{name} must use HTTPS except on a loopback host"
        )));
    }
    Ok(url)
}

#[derive(Debug, Clone, Deserialize)]
struct Discovery {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    device_authorization_endpoint: Option<String>,
    jwks_uri: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Debug, Clone, Deserialize)]
struct Jwk {
    kid: String,
    kty: String,
    crv: String,
    x: String,
    y: String,
    alg: Option<String>,
    #[serde(rename = "use")]
    key_use: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum Audience {
    One(String),
    Many(Vec<String>),
}

impl Audience {
    fn values(&self) -> Vec<&str> {
        match self {
            Self::One(value) => vec![value.as_str()],
            Self::Many(values) => values.iter().map(String::as_str).collect(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Claims {
    iss: String,
    aud: Audience,
    exp: u64,
    iat: u64,
    sub: String,
    nbf: Option<u64>,
    azp: Option<String>,
    nonce: Option<String>,
    at_hash: Option<serde_json::Value>,
    auth_time: Option<serde_json::Value>,
    client_id: Option<String>,
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeviceAuthorization {
    device_code: String,
    user_code: String,
    verification_uri: String,
    verification_uri_complete: Option<String>,
    expires_in: u64,
    interval: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    id_token: Option<String>,
    refresh_token: Option<String>,
    token_type: String,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct OAuthErrorBody {
    error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredSession {
    issuer: String,
    client_id: String,
    subject: String,
    access_token: String,
    refresh_token: String,
    expires_at: u64,
}

impl StoredSession {
    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }
}

pub trait CredentialStore: Send + Sync {
    fn load(&self) -> Result<Option<StoredSession>, AuthError>;
    fn save(&self, session: &StoredSession) -> Result<(), AuthError>;
    fn delete(&self) -> Result<(), AuthError>;
}

#[derive(Debug, Clone)]
pub struct NativeCredentialStore {
    user_key: String,
    marker_path: std::path::PathBuf,
}

impl NativeCredentialStore {
    pub fn for_home(home: &Path) -> Self {
        let digest = Sha256::digest(home.to_string_lossy().as_bytes());
        let suffix = digest[..16]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        Self {
            user_key: format!("guard-cli-{suffix}"),
            marker_path: home.join(".algo").join("account-keyring.marker"),
        }
    }

    fn entry(&self) -> Result<keyring::Entry, AuthError> {
        keyring::Entry::new(KEYRING_SERVICE, &self.user_key).map_err(|_| AuthError::CredentialStore)
    }
}

impl CredentialStore for NativeCredentialStore {
    fn load(&self) -> Result<Option<StoredSession>, AuthError> {
        match self.entry()?.get_password() {
            Ok(raw) => serde_json::from_str(&raw)
                .map(Some)
                .map_err(|_| AuthError::CredentialStore),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(AuthError::CredentialStore),
        }
    }

    fn save(&self, session: &StoredSession) -> Result<(), AuthError> {
        let encoded = serde_json::to_string(session).map_err(|_| AuthError::CredentialStore)?;
        let entry = self.entry()?;
        entry
            .set_password(&encoded)
            .map_err(|_| AuthError::CredentialStore)?;
        let marker_result = self
            .marker_path
            .parent()
            .ok_or(AuthError::CredentialStore)
            .and_then(|parent| {
                std::fs::create_dir_all(parent).map_err(|_| AuthError::CredentialStore)
            })
            .and_then(|()| {
                std::fs::write(&self.marker_path, b"native-keyring\n")
                    .map_err(|_| AuthError::CredentialStore)
            });
        if marker_result.is_err() {
            let _ = entry.delete_credential();
        }
        marker_result
    }

    fn delete(&self) -> Result<(), AuthError> {
        let marker_existed = self.marker_path.exists();
        match self
            .entry()
            .and_then(|entry| match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(entry),
                Err(_) => Err(AuthError::CredentialStore),
            }) {
            Ok(_) => {
                if marker_existed {
                    std::fs::remove_file(&self.marker_path)
                        .map_err(|_| AuthError::CredentialStore)?;
                }
                Ok(())
            }
            Err(_) if !marker_existed => Ok(()),
            Err(error) => Err(error),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginEvent {
    Verification {
        verification_uri: String,
        verification_uri_complete: Option<String>,
        user_code: String,
    },
    AuthorizationUrl(String),
    Waiting,
    SlowDown {
        interval_secs: u64,
    },
    Success,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenKind<'a> {
    Access,
    Id { nonce: Option<&'a str> },
}

#[derive(Clone)]
pub struct AccountClient {
    config: AuthConfig,
    http: Client,
    jwks_cache: Arc<Mutex<Option<(Instant, Jwks)>>>,
}

impl AccountClient {
    pub fn new(config: AuthConfig) -> Result<Self, AuthError> {
        let http = Client::builder()
            .connect_timeout(HTTP_TIMEOUT)
            .timeout(HTTP_TIMEOUT)
            .user_agent(concat!("algorithco-guard/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| AuthError::Config("HTTP client could not be created".into()))?;
        Ok(Self {
            config,
            http,
            jwks_cache: Arc::new(Mutex::new(None)),
        })
    }

    pub fn login_device<S, F>(
        &self,
        store: &S,
        cancelled: &AtomicBool,
        emit: F,
    ) -> Result<StoredSession, AuthError>
    where
        S: CredentialStore,
        F: FnMut(LoginEvent),
    {
        self.login_device_with_wait(store, cancelled, emit, |duration, flag| {
            wait_cancelable(duration, flag)
        })
    }

    fn login_device_with_wait<S, F, W>(
        &self,
        store: &S,
        cancelled: &AtomicBool,
        mut emit: F,
        mut wait: W,
    ) -> Result<StoredSession, AuthError>
    where
        S: CredentialStore,
        F: FnMut(LoginEvent),
        W: FnMut(Duration, &AtomicBool) -> Result<(), AuthError>,
    {
        let discovery = self.discovery()?;
        let endpoint = discovery
            .device_authorization_endpoint
            .as_deref()
            .ok_or(AuthError::InvalidResponse)?;
        let response = self
            .http
            .post(endpoint)
            .form(&[("client_id", CLIENT_ID), ("scope", SCOPES)])
            .send()
            .map_err(|_| AuthError::Network)?;
        let device: DeviceAuthorization = success_json(response)?;
        validate_device_authorization(&device)?;
        emit(LoginEvent::Verification {
            verification_uri: device.verification_uri.clone(),
            verification_uri_complete: device.verification_uri_complete.clone(),
            user_code: device.user_code.clone(),
        });

        let deadline = Instant::now() + Duration::from_secs(device.expires_in);
        let mut interval = Duration::from_secs(device.interval.unwrap_or(5).max(1));
        loop {
            if cancelled.load(Ordering::Relaxed) {
                return Err(AuthError::Cancelled);
            }
            if Instant::now() >= deadline {
                return Err(AuthError::Expired);
            }
            wait(interval, cancelled)?;
            if Instant::now() >= deadline {
                return Err(AuthError::Expired);
            }
            let response = self
                .http
                .post(&discovery.token_endpoint)
                .form(&[
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                    ("device_code", device.device_code.as_str()),
                    ("client_id", CLIENT_ID),
                ])
                .send()
                .map_err(|_| AuthError::Network)?;
            if response.status().is_success() {
                let tokens: TokenResponse = response_json(response)?;
                let session = self.accept_initial_tokens(&discovery, tokens, None)?;
                store.save(&session)?;
                emit(LoginEvent::Success);
                return Ok(session);
            }
            let error: OAuthErrorBody = response_json(response)?;
            match error.error.as_str() {
                "authorization_pending" => emit(LoginEvent::Waiting),
                "slow_down" => {
                    interval = interval.saturating_add(Duration::from_secs(5));
                    emit(LoginEvent::SlowDown {
                        interval_secs: interval.as_secs(),
                    });
                }
                "expired_token" => return Err(AuthError::Expired),
                "access_denied" => return Err(AuthError::OAuth("access denied".into())),
                _ => return Err(AuthError::OAuth("token request rejected".into())),
            }
        }
    }

    pub fn login_loopback<S, F>(
        &self,
        store: &S,
        cancelled: &AtomicBool,
        mut emit: F,
    ) -> Result<StoredSession, AuthError>
    where
        S: CredentialStore,
        F: FnMut(LoginEvent),
    {
        let discovery = self.discovery()?;
        let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|_| AuthError::Loopback)?;
        listener
            .set_nonblocking(true)
            .map_err(|_| AuthError::Loopback)?;
        let port = listener
            .local_addr()
            .map_err(|_| AuthError::Loopback)?
            .port();
        let redirect_uri = format!("http://127.0.0.1:{port}/callback");
        let state = random_b64();
        let nonce = random_b64();
        let verifier = random_b64();
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let mut authorization = Url::parse(&discovery.authorization_endpoint)
            .map_err(|_| AuthError::InvalidResponse)?;
        authorization.query_pairs_mut().extend_pairs([
            ("client_id", CLIENT_ID),
            ("redirect_uri", redirect_uri.as_str()),
            ("response_type", "code"),
            ("scope", SCOPES),
            ("state", state.as_str()),
            ("nonce", nonce.as_str()),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
        ]);
        emit(LoginEvent::AuthorizationUrl(authorization.to_string()));

        let deadline = Instant::now() + LOOPBACK_TIMEOUT;
        let code = loop {
            if cancelled.load(Ordering::Relaxed) {
                return Err(AuthError::Cancelled);
            }
            if Instant::now() >= deadline {
                return Err(AuthError::Expired);
            }
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let result = read_loopback_callback(&mut stream, &state);
                    let success = result.is_ok();
                    write_loopback_response(&mut stream, success);
                    break result?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(_) => return Err(AuthError::Loopback),
            }
        };

        let response = self
            .http
            .post(&discovery.token_endpoint)
            .form(&[
                ("grant_type", "authorization_code"),
                ("client_id", CLIENT_ID),
                ("code", code.as_str()),
                ("redirect_uri", redirect_uri.as_str()),
                ("code_verifier", verifier.as_str()),
            ])
            .send()
            .map_err(|_| AuthError::Network)?;
        let tokens: TokenResponse = success_json(response)?;
        let session = self.accept_initial_tokens(&discovery, tokens, Some(&nonce))?;
        store.save(&session)?;
        emit(LoginEvent::Success);
        Ok(session)
    }

    pub fn access_token<S: CredentialStore>(&self, store: &S) -> Result<Option<String>, AuthError> {
        let Some(mut session) = store.load()? else {
            return Ok(None);
        };
        if session.issuer != self.config.issuer() || session.client_id != CLIENT_ID {
            store.delete()?;
            return Err(AuthError::ReloginRequired);
        }
        let now = unix_now()?;
        if session.expires_at > now.saturating_add(30) {
            return Ok(Some(session.access_token));
        }
        let discovery = self.discovery()?;
        let response = self
            .http
            .post(&discovery.token_endpoint)
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", CLIENT_ID),
                ("refresh_token", session.refresh_token.as_str()),
            ])
            .send()
            .map_err(|_| AuthError::Network)?;
        if !response.status().is_success() {
            let body: OAuthErrorBody = response_json(response)?;
            if matches!(body.error.as_str(), "invalid_grant" | "invalid_token") {
                store.delete()?;
                return Err(AuthError::ReloginRequired);
            }
            return Err(AuthError::OAuth("refresh rejected".into()));
        }
        let tokens: TokenResponse = response_json(response)?;
        validate_token_response(&tokens)?;
        let access =
            self.verify_token(&discovery, &tokens.access_token, TokenKind::Access, false)?;
        if access.sub != session.subject {
            store.delete()?;
            return Err(AuthError::ReloginRequired);
        }
        let replacement = tokens.refresh_token.ok_or(AuthError::InvalidResponse)?;
        if replacement.is_empty() || replacement == session.refresh_token {
            store.delete()?;
            return Err(AuthError::ReloginRequired);
        }
        if let Some(id_token) = tokens.id_token.as_deref() {
            let id =
                self.verify_token(&discovery, id_token, TokenKind::Id { nonce: None }, false)?;
            if id.sub != session.subject {
                return Err(AuthError::InvalidToken);
            }
        }
        session.access_token = tokens.access_token;
        session.refresh_token = replacement;
        session.expires_at = access.exp;
        store.save(&session)?;
        Ok(Some(session.access_token))
    }

    pub fn logout<S: CredentialStore>(&self, store: &S) -> Result<(), AuthError> {
        store.delete()
    }

    fn discovery(&self) -> Result<Discovery, AuthError> {
        let mut url = self.config.issuer.clone();
        url.set_path("/.well-known/openid-configuration");
        url.set_query(None);
        url.set_fragment(None);
        let response = self.http.get(url).send().map_err(|_| AuthError::Network)?;
        let discovery: Discovery = success_json(response)?;
        if discovery.issuer != self.config.issuer() {
            return Err(AuthError::InvalidResponse);
        }
        for endpoint in [
            discovery.authorization_endpoint.as_str(),
            discovery.token_endpoint.as_str(),
            discovery.jwks_uri.as_str(),
        ] {
            let endpoint = trusted_url(endpoint, "discovered endpoint")?;
            if endpoint.origin() != self.config.issuer.origin() {
                return Err(AuthError::InvalidResponse);
            }
        }
        if let Some(endpoint) = discovery.device_authorization_endpoint.as_deref() {
            let endpoint = trusted_url(endpoint, "device authorization endpoint")?;
            if endpoint.origin() != self.config.issuer.origin() {
                return Err(AuthError::InvalidResponse);
            }
        }
        Ok(discovery)
    }

    fn accept_initial_tokens(
        &self,
        discovery: &Discovery,
        tokens: TokenResponse,
        nonce: Option<&str>,
    ) -> Result<StoredSession, AuthError> {
        validate_token_response(&tokens)?;
        let id_token = tokens
            .id_token
            .as_deref()
            .ok_or(AuthError::InvalidResponse)?;
        let refresh_token = tokens
            .refresh_token
            .filter(|value| !value.is_empty())
            .ok_or(AuthError::InvalidResponse)?;
        let id = self.verify_token(discovery, id_token, TokenKind::Id { nonce }, false)?;
        let access =
            self.verify_token(discovery, &tokens.access_token, TokenKind::Access, false)?;
        if id.sub != access.sub {
            return Err(AuthError::InvalidToken);
        }
        Ok(StoredSession {
            issuer: self.config.issuer().to_string(),
            client_id: CLIENT_ID.to_string(),
            subject: access.sub,
            access_token: tokens.access_token,
            refresh_token,
            expires_at: access.exp,
        })
    }

    fn verify_token(
        &self,
        discovery: &Discovery,
        token: &str,
        kind: TokenKind<'_>,
        force_jwks: bool,
    ) -> Result<Claims, AuthError> {
        if token.is_empty() || token.len() > MAX_TOKEN_BYTES || token.split('.').count() != 3 {
            return Err(AuthError::InvalidToken);
        }
        let header = decode_header(token).map_err(|_| AuthError::InvalidToken)?;
        if header.alg != Algorithm::ES256 {
            return Err(AuthError::InvalidToken);
        }
        match kind {
            TokenKind::Access if header.typ.as_deref() != Some("at+jwt") => {
                return Err(AuthError::InvalidToken)
            }
            TokenKind::Id { .. } if header.typ.as_deref() == Some("at+jwt") => {
                return Err(AuthError::InvalidToken)
            }
            _ => {}
        }
        let kid = header.kid.as_deref().ok_or(AuthError::InvalidToken)?;
        let jwks = self.jwks(discovery, force_jwks)?;
        let matching = jwks
            .keys
            .iter()
            .filter(|key| {
                key.kid == kid
                    && key.kty == "EC"
                    && key.crv == "P-256"
                    && key.alg.as_deref().map_or(true, |alg| alg == "ES256")
                    && key.key_use.as_deref().map_or(true, |usage| usage == "sig")
            })
            .collect::<Vec<_>>();
        if matching.len() != 1 {
            if !force_jwks {
                return self.verify_token(discovery, token, kind, true);
            }
            return Err(AuthError::InvalidToken);
        }
        let key = DecodingKey::from_ec_components(&matching[0].x, &matching[0].y)
            .map_err(|_| AuthError::InvalidToken)?;
        let mut validation = Validation::new(Algorithm::ES256);
        validation.set_issuer(&[self.config.issuer()]);
        validation.set_audience(&[CLIENT_ID]);
        validation.leeway = self.config.clock_skew_secs;
        validation.validate_nbf = true;
        validation.required_spec_claims =
            HashSet::from(["exp".to_string(), "iat".to_string(), "sub".to_string()]);
        let claims = decode::<Claims>(token, &key, &validation)
            .map_err(|_| AuthError::InvalidToken)?
            .claims;
        validate_claims(
            &claims,
            kind,
            self.config.issuer(),
            self.config.clock_skew_secs,
        )?;
        Ok(claims)
    }

    fn jwks(&self, discovery: &Discovery, force: bool) -> Result<Jwks, AuthError> {
        if !force {
            let cached = self
                .jwks_cache
                .lock()
                .map_err(|_| AuthError::InvalidResponse)?;
            if let Some((fetched, jwks)) = cached.as_ref() {
                if fetched.elapsed() < JWKS_CACHE_TTL {
                    return Ok(jwks.clone());
                }
            }
        }
        let response = self
            .http
            .get(&discovery.jwks_uri)
            .send()
            .map_err(|_| AuthError::Network)?;
        let jwks: Jwks = success_json(response)?;
        if jwks.keys.is_empty() {
            return Err(AuthError::InvalidResponse);
        }
        *self
            .jwks_cache
            .lock()
            .map_err(|_| AuthError::InvalidResponse)? = Some((Instant::now(), jwks.clone()));
        Ok(jwks)
    }
}

fn validate_claims(
    claims: &Claims,
    kind: TokenKind<'_>,
    issuer: &str,
    clock_skew_secs: u64,
) -> Result<(), AuthError> {
    let now = unix_now()?;
    let audiences = claims.aud.values();
    if claims.iss != issuer
        || claims.sub.is_empty()
        || claims.iat > now.saturating_add(clock_skew_secs)
        || !audiences.contains(&CLIENT_ID)
        || (audiences.len() > 1 && claims.azp.as_deref() != Some(CLIENT_ID))
        || claims
            .nbf
            .is_some_and(|nbf| nbf > now.saturating_add(clock_skew_secs))
    {
        return Err(AuthError::InvalidToken);
    }
    match kind {
        TokenKind::Access => {
            if claims.client_id.as_deref() != Some(CLIENT_ID)
                || claims.scope.as_deref().map_or(true, str::is_empty)
                || claims.nonce.is_some()
                || claims.at_hash.is_some()
                || claims.auth_time.is_some()
            {
                return Err(AuthError::InvalidToken);
            }
        }
        TokenKind::Id { nonce } => {
            if claims.client_id.is_some() || claims.scope.is_some() {
                return Err(AuthError::InvalidToken);
            }
            if nonce.is_some_and(|expected| {
                claims
                    .nonce
                    .as_deref()
                    .map_or(true, |actual| !constant_time_equal(actual, expected))
            }) {
                return Err(AuthError::InvalidToken);
            }
        }
    }
    Ok(())
}

fn validate_token_response(tokens: &TokenResponse) -> Result<(), AuthError> {
    if !tokens.token_type.eq_ignore_ascii_case("bearer")
        || tokens.access_token.is_empty()
        || tokens.expires_in == 0
    {
        return Err(AuthError::InvalidResponse);
    }
    Ok(())
}

fn validate_device_authorization(device: &DeviceAuthorization) -> Result<(), AuthError> {
    trusted_url(&device.verification_uri, "verification URI")?;
    if let Some(uri) = device.verification_uri_complete.as_deref() {
        trusted_url(uri, "complete verification URI")?;
    }
    if device.device_code.is_empty()
        || device.device_code.len() > 4096
        || device.user_code.is_empty()
        || device.user_code.len() > 256
        || device.expires_in == 0
        || device.expires_in > 3600
        || device.interval.unwrap_or(5) > 60
    {
        return Err(AuthError::InvalidResponse);
    }
    Ok(())
}

fn success_json<T: DeserializeOwned>(
    response: reqwest::blocking::Response,
) -> Result<T, AuthError> {
    if !response.status().is_success() {
        return Err(AuthError::OAuth(format!(
            "request rejected ({})",
            response.status().as_u16()
        )));
    }
    response_json(response)
}

fn response_json<T: DeserializeOwned>(
    response: reqwest::blocking::Response,
) -> Result<T, AuthError> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_JSON_BYTES as u64)
    {
        return Err(AuthError::InvalidResponse);
    }
    let mut bytes = Vec::new();
    response
        .take(MAX_JSON_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| AuthError::Network)?;
    if bytes.len() > MAX_JSON_BYTES {
        return Err(AuthError::InvalidResponse);
    }
    serde_json::from_slice(&bytes).map_err(|_| AuthError::InvalidResponse)
}

fn wait_cancelable(duration: Duration, cancelled: &AtomicBool) -> Result<(), AuthError> {
    let deadline = Instant::now() + duration;
    while Instant::now() < deadline {
        if cancelled.load(Ordering::Relaxed) {
            return Err(AuthError::Cancelled);
        }
        thread::sleep((deadline - Instant::now()).min(Duration::from_millis(100)));
    }
    Ok(())
}

fn unix_now() -> Result<u64, AuthError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| AuthError::InvalidToken)
}

fn random_b64() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn constant_time_equal(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    let mut diff = left.len() ^ right.len();
    for index in 0..left.len().max(right.len()) {
        diff |= usize::from(
            left.get(index).copied().unwrap_or_default()
                ^ right.get(index).copied().unwrap_or_default(),
        );
    }
    diff == 0
}

fn read_loopback_callback(
    stream: &mut TcpStream,
    expected_state: &str,
) -> Result<String, AuthError> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| AuthError::Loopback)?;
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 1024];
    loop {
        let size = stream.read(&mut chunk).map_err(|_| AuthError::Loopback)?;
        if size == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..size]);
        if bytes.len() > 8192 {
            return Err(AuthError::Loopback);
        }
        if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    if bytes.is_empty() || !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
        return Err(AuthError::Loopback);
    }
    let request = std::str::from_utf8(&bytes).map_err(|_| AuthError::Loopback)?;
    let first = request.lines().next().ok_or(AuthError::Loopback)?;
    let mut parts = first.split_whitespace();
    if parts.next() != Some("GET") {
        return Err(AuthError::Loopback);
    }
    let target = parts.next().ok_or(AuthError::Loopback)?;
    let callback =
        Url::parse(&format!("http://127.0.0.1{target}")).map_err(|_| AuthError::Loopback)?;
    if callback.path() != "/callback" {
        return Err(AuthError::Loopback);
    }
    if let Some(error) = callback.query_pairs().find(|(name, _)| name == "error") {
        return Err(AuthError::OAuth(error.1.into_owned()));
    }
    let states = callback
        .query_pairs()
        .filter(|(name, _)| name == "state")
        .map(|(_, value)| value.into_owned())
        .collect::<Vec<_>>();
    if states.len() != 1 {
        return Err(AuthError::Loopback);
    }
    let state = &states[0];
    if !constant_time_equal(state, expected_state) {
        return Err(AuthError::OAuth("state mismatch".into()));
    }
    let mut codes = callback
        .query_pairs()
        .filter(|(name, _)| name == "code")
        .map(|(_, value)| value.into_owned())
        .collect::<Vec<_>>();
    if codes.len() != 1 || codes[0].is_empty() || codes[0].len() > 4096 {
        return Err(AuthError::Loopback);
    }
    Ok(codes.remove(0))
}

fn write_loopback_response(stream: &mut TcpStream, success: bool) {
    let body = if success {
        "Sign-in complete. You can close this window."
    } else {
        "Sign-in failed. Return to the terminal and try again."
    };
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Security-Policy: default-src 'none'; frame-ancestors 'none'\r\nReferrer-Policy: no-referrer\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
}

/// Verification boundary reserved for account-issued offline licenses.
///
/// The account plan config currently leaves `licenseTtlDays` unset, so PR5
/// intentionally provides no issuer or persistence implementation.
pub trait OfflineLicenseVerifier: Send + Sync {
    fn verify(&self, token: &str) -> Result<VerifiedOfflineLicense, AuthError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedOfflineLicense {
    pub subject: String,
    pub plan: String,
    pub expires_at: u64,
    pub limits: serde_json::Map<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::{
        rand::SystemRandom,
        signature::{EcdsaKeyPair, KeyPair, ECDSA_P256_SHA256_FIXED_SIGNING},
    };
    use serde_json::{json, Value};
    use std::{
        collections::VecDeque,
        net::{Shutdown, TcpListener},
        sync::atomic::{AtomicUsize, Ordering as AtomicOrdering},
    };

    #[derive(Default)]
    struct MemoryStore {
        session: Mutex<Option<StoredSession>>,
        deletes: AtomicUsize,
    }

    impl CredentialStore for MemoryStore {
        fn load(&self) -> Result<Option<StoredSession>, AuthError> {
            Ok(self.session.lock().unwrap().clone())
        }

        fn save(&self, session: &StoredSession) -> Result<(), AuthError> {
            *self.session.lock().unwrap() = Some(session.clone());
            Ok(())
        }

        fn delete(&self) -> Result<(), AuthError> {
            *self.session.lock().unwrap() = None;
            self.deletes.fetch_add(1, AtomicOrdering::Relaxed);
            Ok(())
        }
    }

    struct MockServer {
        issuer: String,
        requests: Arc<Mutex<Vec<String>>>,
        handle: Option<thread::JoinHandle<()>>,
    }

    impl MockServer {
        fn start<F>(build: F) -> Self
        where
            F: FnOnce(&str) -> Vec<(u16, String)>,
        {
            let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
            let issuer = format!("http://{}", listener.local_addr().unwrap());
            let mut responses = VecDeque::from(build(&issuer));
            let requests = Arc::new(Mutex::new(Vec::new()));
            let captured = Arc::clone(&requests);
            let handle = thread::spawn(move || {
                while let Some((status, body)) = responses.pop_front() {
                    let (mut stream, _) = listener.accept().unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let mut request = Vec::new();
                    let mut chunk = [0_u8; 4096];
                    let header_end;
                    loop {
                        let read = stream.read(&mut chunk).unwrap();
                        assert!(read > 0);
                        request.extend_from_slice(&chunk[..read]);
                        if let Some(index) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                            header_end = index + 4;
                            break;
                        }
                    }
                    let headers = String::from_utf8_lossy(&request[..header_end]);
                    let content_length = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    while request.len() < header_end + content_length {
                        let read = stream.read(&mut chunk).unwrap();
                        assert!(read > 0);
                        request.extend_from_slice(&chunk[..read]);
                    }
                    captured
                        .lock()
                        .unwrap()
                        .push(String::from_utf8_lossy(&request).into_owned());
                    let reason = if status == 200 { "OK" } else { "Bad Request" };
                    let response = format!(
                        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    stream.write_all(response.as_bytes()).unwrap();
                }
            });
            Self {
                issuer,
                requests,
                handle: Some(handle),
            }
        }

        fn finish(mut self) -> Vec<String> {
            self.handle.take().unwrap().join().unwrap();
            Arc::try_unwrap(self.requests)
                .unwrap()
                .into_inner()
                .unwrap()
        }
    }

    struct TestSigner {
        key: EcdsaKeyPair,
        rng: SystemRandom,
        jwks: String,
    }

    impl TestSigner {
        fn new() -> Self {
            let rng = SystemRandom::new();
            let pkcs8 =
                EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
            let key =
                EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_ref(), &rng)
                    .unwrap();
            let point = key.public_key().as_ref();
            assert_eq!(point.len(), 65);
            assert_eq!(point[0], 4);
            let x = URL_SAFE_NO_PAD.encode(&point[1..33]);
            let y = URL_SAFE_NO_PAD.encode(&point[33..65]);
            Self {
                key,
                rng,
                jwks: json!({"keys":[{
                    "kid":"test-key", "kty":"EC", "crv":"P-256",
                    "alg":"ES256", "use":"sig", "x":x, "y":y
                }]})
                .to_string(),
            }
        }

        fn token(&self, issuer: &str, token_type: &str, claims: Value) -> String {
            let header = URL_SAFE_NO_PAD.encode(
                serde_json::to_vec(&json!({
                    "alg":"ES256", "typ":token_type, "kid":"test-key"
                }))
                .unwrap(),
            );
            let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap());
            let input = format!("{header}.{payload}");
            let signature = self.key.sign(&self.rng, input.as_bytes()).unwrap();
            let _ = issuer;
            format!("{input}.{}", URL_SAFE_NO_PAD.encode(signature.as_ref()))
        }
    }

    fn claims(issuer: &str, access: bool) -> Value {
        let now = unix_now().unwrap();
        let mut value = json!({
            "iss":issuer, "aud":CLIENT_ID, "exp":now + 300, "iat":now,
            "sub":"subject-123"
        });
        if access {
            value["client_id"] = json!(CLIENT_ID);
            value["scope"] = json!(SCOPES);
        } else {
            value["auth_time"] = json!(now);
        }
        value
    }

    fn discovery(issuer: &str) -> String {
        json!({
            "issuer": issuer,
            "authorization_endpoint": format!("{issuer}/authorize"),
            "token_endpoint": format!("{issuer}/token"),
            "device_authorization_endpoint": format!("{issuer}/device"),
            "jwks_uri": format!("{issuer}/jwks")
        })
        .to_string()
    }

    fn token_response(access: &str, id: &str, refresh: &str) -> String {
        json!({
            "access_token":access, "id_token":id, "refresh_token":refresh,
            "token_type":"Bearer", "expires_in":300
        })
        .to_string()
    }

    #[test]
    fn device_flow_honors_pending_slow_down_and_saves_verified_tokens() {
        let signer = TestSigner::new();
        let jwks = signer.jwks.clone();
        let server = MockServer::start(|issuer| {
            let access = signer.token(issuer, "at+jwt", claims(issuer, true));
            let id = signer.token(issuer, "JWT", claims(issuer, false));
            vec![
                (200, discovery(issuer)),
                (
                    200,
                    json!({
                        "device_code":"device-secret", "user_code":"ABCD-EFGH",
                        "verification_uri":format!("{issuer}/activate"),
                        "verification_uri_complete":format!("{issuer}/activate?user_code=ABCD-EFGH"),
                        "expires_in":60, "interval":1
                    })
                    .to_string(),
                ),
                (400, json!({"error":"authorization_pending"}).to_string()),
                (400, json!({"error":"slow_down"}).to_string()),
                (200, token_response(&access, &id, "refresh-1")),
                (200, jwks),
            ]
        });
        let client = AccountClient::new(AuthConfig::new(&server.issuer, 5).unwrap()).unwrap();
        let store = MemoryStore::default();
        let cancelled = AtomicBool::new(false);
        let events = Mutex::new(Vec::new());
        let waits = Mutex::new(Vec::new());
        let session = client
            .login_device_with_wait(
                &store,
                &cancelled,
                |event| events.lock().unwrap().push(event),
                |duration, _| {
                    waits.lock().unwrap().push(duration.as_secs());
                    Ok(())
                },
            )
            .unwrap();
        assert_eq!(session.subject(), "subject-123");
        assert_eq!(waits.into_inner().unwrap(), vec![1, 1, 6]);
        let events = events.into_inner().unwrap();
        assert!(matches!(events[0], LoginEvent::Verification { .. }));
        assert!(events.contains(&LoginEvent::Waiting));
        assert!(events.contains(&LoginEvent::SlowDown { interval_secs: 6 }));
        assert_eq!(events.last(), Some(&LoginEvent::Success));
        assert!(store.load().unwrap().is_some());
        let requests = server.finish();
        assert!(requests[1].contains("client_id=guard-cli"));
        assert!(requests[1].contains("scope=openid+profile+email+offline_access"));
        assert!(requests[2].contains("device_code=device-secret"));
    }

    #[test]
    fn access_and_id_tokens_cannot_be_confused() {
        let signer = TestSigner::new();
        let jwks = signer.jwks.clone();
        let server = MockServer::start(|_| vec![(200, jwks)]);
        let client = AccountClient::new(AuthConfig::new(&server.issuer, 5).unwrap()).unwrap();
        let discovery = Discovery {
            issuer: server.issuer.clone(),
            authorization_endpoint: format!("{}/authorize", server.issuer),
            token_endpoint: format!("{}/token", server.issuer),
            device_authorization_endpoint: None,
            jwks_uri: format!("{}/jwks", server.issuer),
        };
        let access = signer.token(&server.issuer, "at+jwt", claims(&server.issuer, true));
        let id = signer.token(&server.issuer, "JWT", claims(&server.issuer, false));
        assert!(client
            .verify_token(&discovery, &access, TokenKind::Access, false)
            .is_ok());
        assert!(matches!(
            client.verify_token(&discovery, &id, TokenKind::Access, false),
            Err(AuthError::InvalidToken)
        ));
        assert!(matches!(
            client.verify_token(&discovery, &access, TokenKind::Id { nonce: None }, false),
            Err(AuthError::InvalidToken)
        ));
        server.finish();
    }

    #[test]
    fn refresh_rotates_credentials_and_reuse_forces_relogin() {
        let signer = TestSigner::new();
        let jwks = signer.jwks.clone();
        let server = MockServer::start(|issuer| {
            let access = signer.token(issuer, "at+jwt", claims(issuer, true));
            vec![
                (200, discovery(issuer)),
                (
                    200,
                    token_response(
                        &access,
                        &signer.token(issuer, "JWT", claims(issuer, false)),
                        "refresh-2",
                    ),
                ),
                (200, jwks),
            ]
        });
        let store = MemoryStore::default();
        store
            .save(&StoredSession {
                issuer: server.issuer.clone(),
                client_id: CLIENT_ID.into(),
                subject: "subject-123".into(),
                access_token: "expired".into(),
                refresh_token: "refresh-1".into(),
                expires_at: 0,
            })
            .unwrap();
        let client = AccountClient::new(AuthConfig::new(&server.issuer, 5).unwrap()).unwrap();
        assert!(client.access_token(&store).unwrap().is_some());
        assert_eq!(store.load().unwrap().unwrap().refresh_token, "refresh-2");
        let requests = server.finish();
        assert!(requests[1].contains("grant_type=refresh_token"));
        assert!(requests[1].contains("refresh_token=refresh-1"));

        let signer = TestSigner::new();
        let jwks = signer.jwks.clone();
        let reused = MockServer::start(|issuer| {
            let access = signer.token(issuer, "at+jwt", claims(issuer, true));
            vec![
                (200, discovery(issuer)),
                (
                    200,
                    token_response(
                        &access,
                        &signer.token(issuer, "JWT", claims(issuer, false)),
                        "refresh-1",
                    ),
                ),
                (200, jwks),
            ]
        });
        let reused_store = MemoryStore::default();
        reused_store
            .save(&StoredSession {
                issuer: reused.issuer.clone(),
                client_id: CLIENT_ID.into(),
                subject: "subject-123".into(),
                access_token: "expired".into(),
                refresh_token: "refresh-1".into(),
                expires_at: 0,
            })
            .unwrap();
        let client = AccountClient::new(AuthConfig::new(&reused.issuer, 5).unwrap()).unwrap();
        assert!(matches!(
            client.access_token(&reused_store),
            Err(AuthError::ReloginRequired)
        ));
        assert!(reused_store.load().unwrap().is_none());
        assert_eq!(reused_store.deletes.load(AtomicOrdering::Relaxed), 1);
        reused.finish();
    }

    #[test]
    fn invalid_grant_deletes_session() {
        let server = MockServer::start(|issuer| {
            vec![
                (200, discovery(issuer)),
                (400, json!({"error":"invalid_grant"}).to_string()),
            ]
        });
        let store = MemoryStore::default();
        store
            .save(&StoredSession {
                issuer: server.issuer.clone(),
                client_id: CLIENT_ID.into(),
                subject: "subject-123".into(),
                access_token: "expired".into(),
                refresh_token: "used-refresh".into(),
                expires_at: 0,
            })
            .unwrap();
        let client = AccountClient::new(AuthConfig::new(&server.issuer, 5).unwrap()).unwrap();
        assert!(matches!(
            client.access_token(&store),
            Err(AuthError::ReloginRequired)
        ));
        assert!(store.load().unwrap().is_none());
        server.finish();
    }

    #[test]
    fn device_expiry_and_cancel_stop_before_polling() {
        assert!(matches!(
            validate_device_authorization(&DeviceAuthorization {
                device_code: "d".into(),
                user_code: "u".into(),
                verification_uri: "https://auth.example/activate".into(),
                verification_uri_complete: None,
                expires_in: 0,
                interval: Some(1),
            }),
            Err(AuthError::InvalidResponse)
        ));
        let cancelled = AtomicBool::new(true);
        assert!(matches!(
            wait_cancelable(Duration::from_secs(1), &cancelled),
            Err(AuthError::Cancelled)
        ));
    }

    #[test]
    fn loopback_callback_requires_exact_state_and_path() {
        fn request(target: &str, state: &str) -> Result<String, AuthError> {
            let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
            let address = listener.local_addr().unwrap();
            let target = target.to_string();
            let writer = thread::spawn(move || {
                let mut stream = TcpStream::connect(address).unwrap();
                write!(stream, "GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
                stream.flush().unwrap();
                stream.shutdown(Shutdown::Write).unwrap();
            });
            let (mut stream, _) = listener.accept().unwrap();
            let result = read_loopback_callback(&mut stream, state);
            writer.join().unwrap();
            result
        }
        assert_eq!(
            request("/callback?code=ok&state=expected", "expected").unwrap(),
            "ok"
        );
        assert!(matches!(
            request("/callback?code=ok&state=wrong", "expected"),
            Err(AuthError::OAuth(_))
        ));
        assert!(matches!(
            request(
                "/callback?code=ok&state=expected&state=expected",
                "expected"
            ),
            Err(AuthError::Loopback)
        ));
        assert!(matches!(
            request("/other?code=ok&state=expected", "expected"),
            Err(AuthError::Loopback)
        ));
    }

    #[test]
    fn audience_arrays_require_guard_cli_as_authorized_party() {
        let now = unix_now().unwrap();
        let base = Claims {
            iss: "https://issuer.example".into(),
            aud: Audience::Many(vec![CLIENT_ID.into(), "another-client".into()]),
            exp: now + 60,
            iat: now,
            sub: "subject".into(),
            nbf: None,
            azp: None,
            nonce: None,
            at_hash: None,
            auth_time: None,
            client_id: Some(CLIENT_ID.into()),
            scope: Some(SCOPES.into()),
        };
        assert!(matches!(
            validate_claims(&base, TokenKind::Access, "https://issuer.example", 5),
            Err(AuthError::InvalidToken)
        ));
        let mut valid = base;
        valid.azp = Some(CLIENT_ID.into());
        assert!(validate_claims(&valid, TokenKind::Access, "https://issuer.example", 5).is_ok());
    }

    #[test]
    fn auth_configuration_and_constant_time_comparison_are_strict() {
        assert!(AuthConfig::new("http://account.example", 5).is_err());
        assert!(AuthConfig::new("https://account.example?query=1", 5).is_err());
        assert!(AuthConfig::new("https://account.example", 61).is_err());
        assert!(constant_time_equal("same", "same"));
        assert!(!constant_time_equal("same", "different"));
    }

    #[test]
    fn native_store_marker_proves_logout_leaves_no_guard_residue() {
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
        let home = tempfile::tempdir().unwrap();
        let store = NativeCredentialStore::for_home(home.path());
        let session = StoredSession {
            issuer: "https://auth.example".into(),
            client_id: CLIENT_ID.into(),
            subject: "subject".into(),
            access_token: "access-secret".into(),
            refresh_token: "refresh-secret".into(),
            expires_at: 42,
        };
        store.save(&session).unwrap();
        assert!(store.marker_path.exists());
        let marker = std::fs::read_to_string(&store.marker_path).unwrap();
        assert_eq!(marker, "native-keyring\n");
        assert!(!marker.contains("access-secret"));
        assert!(!marker.contains("refresh-secret"));
        store.delete().unwrap();
        assert!(!store.marker_path.exists());
    }
}
