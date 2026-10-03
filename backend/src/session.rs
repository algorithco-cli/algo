//! Backend session tokens: short-lived HS256 JWTs minted after a successful
//! GitHub/Google OAuth login. These are the accepted credential on
//! authenticated paths — legacy `valid-token-*` credentials are never valid.
//!
//! Key: `ALGO_SESSION_JWT_SECRET` (minimum 32 bytes). Production startup
//! refuses missing or weak secrets; tests use a compile-time test-only key.

use std::sync::OnceLock;

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// Session lifetime: 1 hour (fail-closed, short).
pub const SESSION_TTL_SECS: i64 = 3600;

/// Fixed issuer: sessions are only ever valid at this backend. Validating it
/// prevents cross-system confusion if a secret were ever shared/reused.
pub const SESSION_ISSUER: &str = "algo-backend";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionClaims {
    pub iss: String,
    /// Stable identity: `github:<id>` or `google:<sub>`.
    pub sub: String,
    /// `github` | `google`.
    pub provider: String,
    /// GitHub login or Google display name (if known).
    pub login: Option<String>,
    /// Google email (if known).
    pub email: Option<String>,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    Invalid,
    Expired,
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Generic: never echo token material.
        match self {
            Self::Invalid => write!(f, "unauthorized"),
            Self::Expired => write!(f, "session expired"),
        }
    }
}
impl std::error::Error for SessionError {}

static SESSION_SECRET: OnceLock<Vec<u8>> = OnceLock::new();

fn load_secret() -> Result<Vec<u8>, String> {
    match std::env::var("ALGO_SESSION_JWT_SECRET") {
        Ok(value) if value.len() >= 32 => Ok(value.into_bytes()),
        Ok(_) => Err("ALGO_SESSION_JWT_SECRET must be at least 32 bytes".to_string()),
        Err(_) => {
            #[cfg(test)]
            {
                Ok(vec![b'x'; 32])
            }
            #[cfg(not(test))]
            {
                Err("ALGO_SESSION_JWT_SECRET is required".to_string())
            }
        }
    }
}

fn secret() -> &'static [u8] {
    SESSION_SECRET.get_or_init(|| {
        load_secret().unwrap_or_else(|error| panic!("session secret misconfigured: {error}"))
    })
}

/// Validate and cache the session secret before binding the listener.
pub fn ensure_session_secret_at_startup() -> Result<(), String> {
    let loaded = load_secret()?;
    let _ = SESSION_SECRET.get_or_init(|| loaded);
    Ok(())
}

/// Mint a session token with the default TTL.
pub fn mint_session(provider: &str, sub: &str, login: Option<&str>, email: Option<&str>) -> String {
    mint_session_with_ttl(provider, sub, login, email, SESSION_TTL_SECS)
}

/// Mint with explicit TTL (tests use negative TTL for expiry).
pub fn mint_session_with_ttl(
    provider: &str,
    sub: &str,
    login: Option<&str>,
    email: Option<&str>,
    ttl_secs: i64,
) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = SessionClaims {
        iss: SESSION_ISSUER.to_string(),
        sub: sub.to_string(),
        provider: provider.to_string(),
        login: login.map(|s| s.to_string()),
        email: email.map(|s| s.to_string()),
        iat: now,
        exp: now.saturating_add(ttl_secs),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret()),
    )
    .unwrap_or_default()
}

/// Verify a session token (signature + expiry + shape). Fail-closed.
pub fn verify_session(token: &str) -> Result<SessionClaims, SessionError> {
    if token.is_empty() || token.len() > 4096 {
        return Err(SessionError::Invalid);
    }
    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = true;
    // Zero leeway: mint and verify run on the same host, fail-closed on expiry.
    validation.leeway = 0;
    validation.validate_aud = false;
    validation.set_issuer(&[SESSION_ISSUER]);
    match decode::<SessionClaims>(token, &DecodingKey::from_secret(secret()), &validation) {
        Ok(data) => {
            if data.claims.sub.is_empty()
                || data.claims.provider.is_empty()
                || data.claims.iss != SESSION_ISSUER
            {
                return Err(SessionError::Invalid);
            }
            Ok(data.claims)
        }
        Err(e) => {
            use jsonwebtoken::errors::ErrorKind;
            match e.kind() {
                ErrorKind::ExpiredSignature => Err(SessionError::Expired),
                _ => Err(SessionError::Invalid),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_sync;

    #[test]
    fn mint_verify_roundtrip() {
        let _guard = test_sync::lock();
        let tok = mint_session("github", "github:1", Some("octocat"), None);
        assert_eq!(tok.split('.').count(), 3);
        let claims = verify_session(&tok).expect("fresh session verifies");
        assert_eq!(claims.sub, "github:1");
        assert_eq!(claims.provider, "github");
        assert_eq!(claims.login.as_deref(), Some("octocat"));
    }

    #[test]
    fn proves_ask_on_expired_session() {
        let _guard = test_sync::lock();
        let tok = mint_session_with_ttl("google", "google:abc", None, Some("a@b.c"), -10);
        assert_eq!(verify_session(&tok).unwrap_err(), SessionError::Expired);
    }

    #[test]
    fn proves_ask_on_foreign_issuer_or_secret() {
        let _guard = test_sync::lock();
        let now = chrono::Utc::now().timestamp();
        // Correct shape, foreign issuer, foreign key.
        let foreign = SessionClaims {
            iss: "evil-system".to_string(),
            sub: "github:1".to_string(),
            provider: "github".to_string(),
            login: None,
            email: None,
            iat: now,
            exp: now + 3600,
        };
        let tok = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &foreign,
            &jsonwebtoken::EncodingKey::from_secret(b"some-other-secret-min-16"),
        )
        .unwrap();
        assert!(verify_session(&tok).is_err());
        // Correct issuer, wrong key.
        let mut local_iss = foreign.clone();
        local_iss.iss = SESSION_ISSUER.to_string();
        let tok2 = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &local_iss,
            &jsonwebtoken::EncodingKey::from_secret(b"some-other-secret-min-16"),
        )
        .unwrap();
        assert!(verify_session(&tok2).is_err());
    }

    #[test]
    fn proves_ask_on_tampered_session() {
        let _guard = test_sync::lock();
        let tok = mint_session("github", "github:1", None, None);
        let mut tampered = tok.clone();
        tampered.push('x');
        assert!(verify_session(&tampered).is_err());
        assert!(verify_session("not-a-jwt").is_err());
        assert!(verify_session("").is_err());
    }
}
