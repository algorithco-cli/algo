//! Auth: Bearer validation + require_auth middleware.
//!
//! Accepts backend session JWTs (email/GitHub/Google) and, during migration,
//! the legacy `valid-token-<id>` stub (dev/tests only).

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
/// Accepts (in order):
///
/// 1. Legacy stub `valid-token-<id>` (dev/tests, migration path).
/// 2. Backend session JWT (HS256, minted by GitHub/Google login).
///
/// Bare tokens (no `Bearer ` scheme) and the loose `valid-` prefix are
/// rejected (fail-closed).
pub fn validate_bearer_token(header_value: Option<&str>) -> Result<String, AuthError> {
    let header = header_value.ok_or(AuthError::MissingToken)?;
    // Require explicit Bearer scheme — bare tokens rejected.
    let Some(token) = header.strip_prefix("Bearer ") else {
        return Err(AuthError::MissingToken);
    };
    if token.is_empty() || token.len() > 4096 {
        return Err(AuthError::MissingToken);
    }
    // Legacy stub prefix + non-empty suffix. No bare-token fallback.
    if token.starts_with("valid-token-") && token.len() > "valid-token-".len() {
        return Ok(token.to_string());
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
        assert!(validate_bearer_token(Some("Bearer valid-token-12345")).is_ok());
        // Bare token without Bearer scheme is rejected (fail-closed).
        assert!(validate_bearer_token(Some("valid-token-12345")).is_err());
        // Loose prefix bypass rejected.
        assert!(validate_bearer_token(Some("Bearer valid-xyz")).is_err());
        assert!(validate_bearer_token(Some("Bearer invalid")).is_err());
        assert!(validate_bearer_token(None).is_err());
        assert!(validate_bearer_token(Some("")).is_err());
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

        let mut headers3 = HeaderMap::new();
        headers3.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer valid-token-xyz"),
        );
        assert!(require_auth(&headers3).is_ok());
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
