//! GitHub OAuth login (device flow + token validation).
//!
//! Verified against GitHub docs 2026-09-23:
//! - `POST {base}/device/code` + client_id → device_code/user_code/
//!   verification_uri/expires_in/interval (900s/5s on github.com).
//! - Poll `POST {base}/oauth/access_token` + client_id/device_code/
//!   grant_type=urn:ietf:params:oauth:grant-type:device_code. No client_secret
//!   needed for device flow. 200 + `error` field while pending
//!   (authorization_pending / slow_down / expired_token / access_denied).
//! - Validate via `GET {api}/user` (Bearer) → {login, id}.
//!
//! All bases are injectable (`OAuthConfig`) so tests run against in-process
//! mocks — no internet in CI.

use serde::{Deserialize, Serialize};

use crate::oauth_config::OAuthConfig;
use crate::provider_http::{self, HttpError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GithubError {
    NotConfigured,
    Transport,
    Protocol,
    Unauthorized,
    /// Back off: minimum seconds before the next poll.
    SlowDown(u64),
    /// User cancelled at github.com.
    AccessDenied,
    /// Device code expired: restart the flow.
    CodeExpired,
}

impl std::fmt::Display for GithubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => write!(f, "github login not configured"),
            Self::Transport => write!(f, "github unreachable"),
            Self::Protocol => write!(f, "github protocol error"),
            Self::Unauthorized => write!(f, "github token invalid"),
            Self::SlowDown(s) => write!(f, "slow down, retry after {s}s"),
            Self::AccessDenied => write!(f, "github authorization denied"),
            Self::CodeExpired => write!(f, "device code expired, restart login"),
        }
    }
}
impl std::error::Error for GithubError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInit {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: i64,
    pub interval: i64,
}

#[derive(Debug, Clone)]
pub struct DeviceTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<i64>,
}

#[derive(Debug, Clone)]
pub enum PollOutcome {
    Pending,
    Authorized(DeviceTokens),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubUser {
    pub login: String,
    pub id: i64,
}

/// Cap on GitHub JSON bodies (device/token/user).
const MAX_GITHUB_BODY: usize = 256 * 1024;

fn client_id(cfg: &OAuthConfig) -> Result<&str, GithubError> {
    cfg.github_client_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .ok_or(GithubError::NotConfigured)
}

async fn post_form(
    http: &reqwest::Client,
    url: &str,
    params: &[(&str, &str)],
) -> Result<serde_json::Value, GithubError> {
    provider_http::post_form(http, url, params, MAX_GITHUB_BODY)
        .await
        .map_err(|e| match e {
            HttpError::Transport => GithubError::Transport,
            _ => GithubError::Protocol,
        })
}

/// Step 1: request device + user codes.
pub async fn request_device_code(
    http: &reqwest::Client,
    cfg: &OAuthConfig,
) -> Result<DeviceInit, GithubError> {
    let id = client_id(cfg)?.to_string();
    let v = post_form(http, &cfg.github_device_code_url, &[("client_id", &id)]).await?;
    Ok(DeviceInit {
        device_code: v
            .get("device_code")
            .and_then(|x| x.as_str())
            .ok_or(GithubError::Protocol)?
            .to_string(),
        user_code: v
            .get("user_code")
            .and_then(|x| x.as_str())
            .ok_or(GithubError::Protocol)?
            .to_string(),
        verification_uri: v
            .get("verification_uri")
            .and_then(|x| x.as_str())
            .unwrap_or("https://github.com/login/device")
            .to_string(),
        expires_in: v.get("expires_in").and_then(|x| x.as_i64()).unwrap_or(900),
        interval: v.get("interval").and_then(|x| x.as_i64()).unwrap_or(5),
    })
}

/// Step 2: poll for the access token. Never blocks server-side; one poll only.
pub async fn poll_access_token(
    http: &reqwest::Client,
    cfg: &OAuthConfig,
    device_code: &str,
) -> Result<PollOutcome, GithubError> {
    if device_code.trim().is_empty() || device_code.len() > 256 {
        return Err(GithubError::Protocol);
    }
    let id = client_id(cfg)?.to_string();
    let v = post_form(
        http,
        &cfg.github_access_token_url,
        &[
            ("client_id", &id),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ],
    )
    .await?;
    if let Some(err) = v.get("error").and_then(|x| x.as_str()) {
        return match err {
            "authorization_pending" => Ok(PollOutcome::Pending),
            "slow_down" => {
                let retry = v.get("interval").and_then(|x| x.as_u64()).unwrap_or(10);
                Err(GithubError::SlowDown(retry))
            }
            "expired_token" | "incorrect_device_code" => Err(GithubError::CodeExpired),
            "access_denied" => Err(GithubError::AccessDenied),
            "device_flow_disabled" | "unsupported_grant_type" | "incorrect_client_credentials" => {
                Err(GithubError::Protocol)
            }
            _ => Err(GithubError::Protocol),
        };
    }
    // slow_down may also arrive with a fresh interval alongside pending state.
    let access_token = v
        .get("access_token")
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .ok_or(GithubError::Protocol)?
        .to_string();
    Ok(PollOutcome::Authorized(DeviceTokens {
        access_token,
        refresh_token: v
            .get("refresh_token")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
        expires_in: v.get("expires_in").and_then(|x| x.as_i64()),
    }))
}

/// Validate a GitHub user token (`ghu_`/`gho_`/`ghp_`…) via GET /user.
pub async fn fetch_user(
    http: &reqwest::Client,
    cfg: &OAuthConfig,
    access_token: &str,
) -> Result<GithubUser, GithubError> {
    if access_token.trim().is_empty() || access_token.len() > 512 {
        return Err(GithubError::Unauthorized);
    }
    let url = format!("{}/user", cfg.github_api_base.trim_end_matches('/'));
    let v = match provider_http::get_json(
        http,
        &url,
        Some(access_token),
        &[
            ("Accept", "application/vnd.github+json"),
            ("X-GitHub-Api-Version", "2026-03-10"),
        ],
        MAX_GITHUB_BODY,
    )
    .await
    {
        Ok((_status, v)) => v,
        Err(HttpError::Transport) => return Err(GithubError::Transport),
        Err(HttpError::BadStatus(401) | HttpError::BadStatus(403)) => {
            return Err(GithubError::Unauthorized)
        }
        Err(_) => return Err(GithubError::Protocol),
    };
    Ok(GithubUser {
        login: v
            .get("login")
            .and_then(|x| x.as_str())
            .ok_or(GithubError::Protocol)?
            .to_string(),
        id: v
            .get("id")
            .and_then(|x| x.as_i64())
            .ok_or(GithubError::Protocol)?,
    })
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

    async fn device_code_handler() -> impl IntoResponse {
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "device_code": "mock-dev-1",
                "user_code": "WDJB-MJHT",
                "verification_uri": "https://github.com/login/device",
                "expires_in": 900,
                "interval": 0
            })),
        )
    }

    async fn token_handler(
        axum::Form(body): axum::Form<std::collections::HashMap<String, String>>,
    ) -> impl IntoResponse {
        let code = body.get("device_code").map(|s| s.as_str()).unwrap_or("");
        if code == "mock-dev-slow" {
            return (
                StatusCode::OK,
                Json(serde_json::json!({"error": "slow_down", "interval": 10})),
            )
                .into_response();
        }
        if code == "mock-dev-approved" {
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "access_token": "ghu_mock",
                    "expires_in": 28800,
                    "refresh_token": "ghr_mock",
                    "refresh_token_expires_in": 15897600,
                    "scope": "",
                    "token_type": "bearer"
                })),
            )
                .into_response()
        } else {
            (
                StatusCode::OK,
                Json(serde_json::json!({"error": "authorization_pending"})),
            )
                .into_response()
        }
    }

    async fn user_handler(headers: axum::http::HeaderMap) -> impl IntoResponse {
        let ok =
            headers.get("authorization").and_then(|v| v.to_str().ok()) == Some("Bearer ghu_mock");
        if ok {
            (
                StatusCode::OK,
                Json(serde_json::json!({"login": "octocat", "id": 1})),
            )
                .into_response()
        } else {
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"message": "Bad credentials"})),
            )
                .into_response()
        }
    }

    async fn mock_server() -> (String, tokio::task::JoinHandle<()>) {
        let app = Router::new()
            .route("/login/device/code", post(device_code_handler))
            .route("/login/oauth/access_token", post(token_handler))
            .route("/user", get(user_handler));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (format!("http://{addr}"), handle)
    }

    fn test_cfg(base: &str) -> OAuthConfig {
        OAuthConfig {
            github_client_id: Some("mock-client-id".to_string()),
            github_device_code_url: format!("{base}/login/device/code"),
            github_access_token_url: format!("{base}/login/oauth/access_token"),
            github_api_base: base.to_string(),
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
    async fn device_init_shape() {
        let _guard = test_sync::lock();
        let (base, _h) = mock_server().await;
        let init = request_device_code(&http(), &test_cfg(&base))
            .await
            .unwrap();
        assert_eq!(init.device_code, "mock-dev-1");
        assert_eq!(init.user_code, "WDJB-MJHT");
        assert_eq!(init.expires_in, 900);
    }

    #[tokio::test]
    async fn poll_pending_then_authorized() {
        let _guard = test_sync::lock();
        let (base, _h) = mock_server().await;
        let cfg = test_cfg(&base);
        let http = http();
        assert!(matches!(
            poll_access_token(&http, &cfg, "mock-dev-1").await.unwrap(),
            PollOutcome::Pending
        ));
        assert_eq!(
            poll_access_token(&http, &cfg, "mock-dev-slow")
                .await
                .unwrap_err(),
            GithubError::SlowDown(10)
        );
        match poll_access_token(&http, &cfg, "mock-dev-approved")
            .await
            .unwrap()
        {
            PollOutcome::Authorized(t) => {
                assert_eq!(t.access_token, "ghu_mock");
                assert_eq!(t.expires_in, Some(28800));
            }
            PollOutcome::Pending => panic!("expected authorized"),
        }
    }

    #[tokio::test]
    async fn fetch_user_ok_and_unauthorized() {
        let _guard = test_sync::lock();
        let (base, _h) = mock_server().await;
        let cfg = test_cfg(&base);
        let http = http();
        let user = fetch_user(&http, &cfg, "ghu_mock").await.unwrap();
        assert_eq!(user.login, "octocat");
        assert_eq!(user.id, 1);
        assert_eq!(
            fetch_user(&http, &cfg, "ghu_bogus").await.unwrap_err(),
            GithubError::Unauthorized
        );
    }

    #[tokio::test]
    async fn proves_ask_when_not_configured() {
        let _guard = test_sync::lock();
        let cfg = OAuthConfig::disabled();
        let http = http();
        assert_eq!(
            request_device_code(&http, &cfg).await.unwrap_err(),
            GithubError::NotConfigured
        );
        assert_eq!(
            poll_access_token(&http, &cfg, "x").await.unwrap_err(),
            GithubError::NotConfigured
        );
    }
}
