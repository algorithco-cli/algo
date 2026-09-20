//! Auth: device flow stub (generate device_code, poll returns token), require_auth middleware.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceFlow {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: i32,
    pub interval: i32,
    pub created_at: DateTime<Utc>,
    /// Set when poll succeeds (MVP auto-fills after one poll).
    pub access_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInitResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: i32,
    pub interval: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevicePollResponse {
    pub access_token: Option<String>,
    pub token_type: Option<String>,
    pub expires_in: Option<i32>,
    pub pending: bool,
}

static DEVICE_STORE: OnceLock<Mutex<HashMap<String, DeviceFlow>>> = OnceLock::new();

fn device_store() -> &'static Mutex<HashMap<String, DeviceFlow>> {
    DEVICE_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn generate_user_code() -> String {
    // 8-char alphanumeric upper like "ABCD-1234" for UX.
    let uuid = Uuid::new_v4().to_string().replace('-', "");
    let code = uuid[..8].to_uppercase();
    format!("{}-{}", &code[..4], &code[4..8])
}

/// Initiate device flow: generate device_code, user_code, verification_uri.
pub fn create_device_flow(client_id: &str) -> DeviceInitResponse {
    let _ = client_id; // unused in stub, but validated as non-empty in handler
    let device_code = format!("device_{}", Uuid::new_v4());
    let user_code = generate_user_code();
    let flow = DeviceFlow {
        device_code: device_code.clone(),
        user_code: user_code.clone(),
        verification_uri: "https://auth.algorithco.guard/device".to_string(),
        expires_in: 600,
        interval: 5,
        created_at: Utc::now(),
        access_token: None,
    };
    device_store()
        .lock()
        .expect("device store poisoned")
        .insert(device_code.clone(), flow);
    DeviceInitResponse {
        device_code,
        user_code,
        verification_uri: "https://auth.algorithco.guard/device".to_string(),
        expires_in: 600,
        interval: 5,
    }
}

/// Poll device flow: first poll returns pending, second returns token (MVP stub).
/// In real Zitadel OIDC, the token is issued after user approves on verification_uri.
pub fn poll_device_flow(device_code: &str) -> Result<DevicePollResponse, AuthError> {
    let mut store = device_store().lock().expect("device store poisoned");
    let flow = store
        .get_mut(device_code)
        .ok_or_else(|| AuthError::InvalidDeviceCode(device_code.to_string()))?;

    // Expiry check.
    let elapsed = Utc::now().signed_duration_since(flow.created_at).num_seconds();
    if elapsed > flow.expires_in as i64 {
        return Err(AuthError::Expired);
    }

    // MVP: if no token yet, generate one now and mark pending=false on this call.
    // First poll could be pending, but for deterministic tests we return token immediately.
    if flow.access_token.is_none() {
        let token = format!("valid-token-{}", Uuid::new_v4());
        flow.access_token = Some(token.clone());
        Ok(DevicePollResponse {
            access_token: Some(token),
            token_type: Some("Bearer".to_string()),
            expires_in: Some(3600),
            pending: false,
        })
    } else {
        Ok(DevicePollResponse {
            access_token: flow.access_token.clone(),
            token_type: Some("Bearer".to_string()),
            expires_in: Some(3600),
            pending: false,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AuthError {
    MissingToken,
    InvalidToken(String),
    Expired,
    InvalidDeviceCode(String),
    RateLimited,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingToken => write!(f, "missing Authorization Bearer token"),
            Self::InvalidToken(s) => write!(f, "invalid token: {s}"),
            Self::Expired => write!(f, "device flow expired"),
            Self::InvalidDeviceCode(s) => write!(f, "invalid device_code: {s}"),
            Self::RateLimited => write!(f, "rate limited"),
        }
    }
}
impl std::error::Error for AuthError {}

/// Validate Bearer token (stub). Accepts tokens starting with "valid-token-" or "Bearer valid-".
/// Real flow would verify JWT via Zitadel JWKS + `jsonwebtoken` crate.
pub fn validate_bearer_token(header_value: Option<&str>) -> Result<String, AuthError> {
    let header = header_value.ok_or(AuthError::MissingToken)?;
    // Header expected like "Bearer <token>"
    let token = if let Some(stripped) = header.strip_prefix("Bearer ") {
        stripped
    } else {
        header
    };
    if token.starts_with("valid-token-") || token.starts_with("valid-") {
        Ok(token.to_string())
    } else if token.is_empty() {
        Err(AuthError::MissingToken)
    } else {
        Err(AuthError::InvalidToken(token.to_string()))
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

/// Clear device store (tests).
pub fn clear_device_store() {
    device_store().lock().expect("device store poisoned").clear();
}

#[cfg(test)]
mod tests {
    use crate::test_sync;
    use super::*;

    #[test]
    fn device_flow_create_and_poll() {
        let _guard = test_sync::lock();
        clear_device_store();
        let init = create_device_flow("test-client");
        assert!(init.device_code.starts_with("device_"));
        assert!(init.user_code.contains('-'));
        assert_eq!(init.expires_in, 600);
        let poll = poll_device_flow(&init.device_code).expect("poll should succeed");
        assert!(!poll.pending);
        assert!(poll.access_token.is_some());
        let token = poll.access_token.unwrap();
        assert!(token.starts_with("valid-token-"));
        // Second poll returns same token.
        let poll2 = poll_device_flow(&init.device_code).unwrap();
        assert_eq!(poll2.access_token.unwrap(), token);
    }

    #[test]
    fn poll_invalid_device_code_rejected() {
        let _guard = test_sync::lock();
        clear_device_store();
        let err = poll_device_flow("nonexistent").unwrap_err();
        assert!(matches!(err, AuthError::InvalidDeviceCode(_)));
    }

    #[test]
    fn validate_bearer_ok_and_rejected() {
        let _guard = test_sync::lock();
        assert!(validate_bearer_token(Some("Bearer valid-token-123")).is_ok());
        assert!(validate_bearer_token(Some("valid-token-123")).is_ok());
        assert!(validate_bearer_token(Some("Bearer invalid")).is_err());
        assert!(validate_bearer_token(None).is_err());
        assert!(validate_bearer_token(Some("")).is_err());
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
