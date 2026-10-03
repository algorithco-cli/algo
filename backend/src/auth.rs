//! Auth: dual-mode Bearer validation.
//!
//! Legacy mode accepts backend session JWTs. Account mode accepts only
//! Algorithco account access tokens verified by `account_auth`.

use std::sync::Arc;

use crate::account_auth::{AccountAuthConfig, AccountTokenVerifier, AuthMode, CompactEntitlement};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AuthenticatedUser {
    pub sub: String,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub token_iat: Option<i64>,
    pub token_exp: Option<i64>,
    pub entitlements: Option<Vec<CompactEntitlement>>,
}

#[derive(Clone)]
pub enum AuthService {
    Legacy,
    Account(Arc<AccountTokenVerifier>),
    #[cfg(test)]
    TestAccount(AuthenticatedUser),
}

impl AuthService {
    #[cfg(test)]
    pub fn legacy() -> Self {
        Self::Legacy
    }

    #[cfg(test)]
    pub fn test_account(user: AuthenticatedUser) -> Self {
        Self::TestAccount(user)
    }

    pub async fn from_env(client: reqwest::Client) -> Result<Self, String> {
        match AuthMode::from_env()? {
            AuthMode::Legacy => {
                crate::session::ensure_session_secret_at_startup()?;
                Ok(Self::Legacy)
            }
            AuthMode::Account => Ok(Self::Account(Arc::new(
                AccountTokenVerifier::discover(AccountAuthConfig::from_env()?, client).await?,
            ))),
        }
    }

    pub fn is_legacy(&self) -> bool {
        matches!(self, Self::Legacy)
    }

    pub async fn authenticate(
        &self,
        headers: &axum::http::HeaderMap,
    ) -> Result<AuthenticatedUser, AuthError> {
        match self {
            Self::Legacy => require_auth(headers).map(|sub| AuthenticatedUser {
                sub,
                email: None,
                email_verified: None,
                token_iat: None,
                token_exp: None,
                entitlements: None,
            }),
            Self::Account(verifier) => verifier
                .verify_headers(headers)
                .await
                .map(|claims| AuthenticatedUser {
                    sub: claims.sub,
                    email: claims.email,
                    email_verified: claims.email_verified,
                    token_iat: Some(claims.iat),
                    token_exp: Some(claims.exp),
                    entitlements: claims.entitlements,
                })
                .map_err(|error| match error {
                    crate::account_auth::AccountAuthError::Missing => AuthError::MissingToken,
                    crate::account_auth::AccountAuthError::Expired => AuthError::Expired,
                    crate::account_auth::AccountAuthError::Invalid => AuthError::InvalidToken,
                }),
            #[cfg(test)]
            Self::TestAccount(user) => {
                let bearer = headers
                    .get_all(axum::http::header::AUTHORIZATION)
                    .iter()
                    .filter_map(|value| value.to_str().ok())
                    .collect::<Vec<_>>();
                if bearer.as_slice() == ["Bearer test-account"] {
                    Ok(user.clone())
                } else {
                    Err(AuthError::MissingToken)
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AuthError {
    MissingToken,
    InvalidToken,
    Expired,
    RateLimited,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Generic messages only — never echo tokens or device codes back to
        // clients (prevents enumeration/oracle). Full detail goes to server logs.
        match self {
            Self::MissingToken => write!(f, "unauthorized"),
            Self::InvalidToken => write!(f, "unauthorized"),
            Self::Expired => write!(f, "credential expired"),
            Self::RateLimited => write!(f, "rate limited"),
        }
    }
}
impl std::error::Error for AuthError {}

/// Validate Bearer token.
/// Accepts ONLY backend session JWTs (HS256, minted by login).
///
/// Bare tokens (no `Bearer ` scheme) and the loose `valid-` prefix are
/// always rejected.
pub fn validate_bearer_token(header_value: Option<&str>) -> Result<String, AuthError> {
    let header = header_value.ok_or(AuthError::MissingToken)?;
    // Require explicit Bearer scheme — bare tokens rejected.
    let Some(token) = header.strip_prefix("Bearer ") else {
        return Err(AuthError::MissingToken);
    };
    if token.is_empty() || token.len() > 4096 {
        return Err(AuthError::MissingToken);
    }
    // Session JWT: return the stable subject as caller identity.
    match crate::session::verify_session(token) {
        Ok(claims) => Ok(claims.sub),
        Err(crate::session::SessionError::Expired) => Err(AuthError::Expired),
        Err(_) => Err(AuthError::InvalidToken),
    }
}

/// Placeholder for `require_auth` middleware: checks Authorization header.
/// Returns user_id (here token) or 401 error.
pub fn require_auth(headers: &axum::http::HeaderMap) -> Result<String, AuthError> {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    validate_bearer_token(auth_header)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_sync;

    #[test]
    fn validate_bearer_ok_and_rejected() {
        let _guard = test_sync::lock();
        // Real session JWT authenticates (identity = stable sub).
        let token = crate::session::mint_session("test", "test:alice", None, None);
        let header = format!("Bearer {token}");
        assert_eq!(validate_bearer_token(Some(&header)).unwrap(), "test:alice");
        // Legacy stub is DENIED by default (fail-closed, C1).
        assert!(validate_bearer_token(Some("Bearer valid-token-12345")).is_err());
        // Bare token without Bearer scheme is rejected (fail-closed).
        assert!(validate_bearer_token(Some(&token)).is_err());
        // Loose prefix bypass rejected.
        assert!(validate_bearer_token(Some("Bearer valid-xyz")).is_err());
        assert!(validate_bearer_token(Some("Bearer invalid")).is_err());
        assert!(validate_bearer_token(None).is_err());
        assert!(validate_bearer_token(Some("")).is_err());
    }

    #[test]
    fn legacy_stub_is_unconditionally_rejected() {
        let _guard = test_sync::lock();
        assert!(validate_bearer_token(Some("Bearer valid-token-12345")).is_err());
        // A runtime environment variable must never re-enable the bypass.
        std::env::set_var("ALGO_ALLOW_LEGACY_STUB", "1");
        assert!(validate_bearer_token(Some("Bearer valid-token-12345")).is_err());
        std::env::remove_var("ALGO_ALLOW_LEGACY_STUB");
    }

    #[test]
    fn proves_forged_token_denied_on_production_path() {
        let _guard = test_sync::lock();
        // Forged stub, tampered JWT, and foreign-signed JWT all fail.
        assert!(validate_bearer_token(Some("Bearer valid-token-evil")).is_err());
        let mut forged = crate::session::mint_session("test", "test:alice", None, None);
        forged.push('x');
        assert!(validate_bearer_token(Some(&format!("Bearer {forged}"))).is_err());
        assert!(validate_bearer_token(Some("Bearer not-a-jwt")).is_err());
    }

    #[test]
    fn proves_ask_on_valid_prefix_bypass() {
        let _guard = test_sync::lock();
        // `valid-` without `token-` must not authenticate.
        assert!(validate_bearer_token(Some("Bearer valid-xyz")).is_err());
        assert!(validate_bearer_token(Some("Bearer valid-token-")).is_err());
    }

    #[test]
    fn require_auth_rejects_missing() {
        let _guard = test_sync::lock();
        use axum::http::{HeaderMap, HeaderValue};
        let headers = HeaderMap::new();
        let err = require_auth(&headers).unwrap_err();
        assert_eq!(err, AuthError::MissingToken);

        let mut headers2 = HeaderMap::new();
        headers2.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer invalid-token"),
        );
        assert!(require_auth(&headers2).is_err());

        // Real session authenticates; legacy stub denied by default.
        let token = crate::session::mint_session("test", "test:bob", None, None);
        let mut headers3 = HeaderMap::new();
        headers3.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );
        assert_eq!(require_auth(&headers3).unwrap(), "test:bob");

        let mut headers4 = HeaderMap::new();
        headers4.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer valid-token-xyz"),
        );
        assert!(require_auth(&headers4).is_err());
    }

    #[test]
    fn proves_ask_on_unauthed() {
        let _guard = test_sync::lock();
        // Unauthed must be rejected, caller maps to 401/ask.
        let headers = axum::http::HeaderMap::new();
        let res = require_auth(&headers);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), AuthError::MissingToken);
    }
}
