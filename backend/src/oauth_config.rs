//! OAuth provider configuration: GitHub App/OAuth App + Google OIDC.
//!
//! IDs/secrets come from env (never code). Endpoint bases are configurable so
//! tests can point at in-process mock servers instead of the real internet:
//!
//! - `ALGO_GITHUB_CLIENT_ID`
//! - `ALGO_GOOGLE_CLIENT_ID`, `ALGO_GOOGLE_CLIENT_SECRET` (optional; without a
//!   secret the backend acts as a public client with PKCE only)

#[derive(Debug, Clone)]
pub struct OAuthConfig {
    // GitHub
    pub github_client_id: Option<String>,
    pub github_device_code_url: String,
    pub github_access_token_url: String,
    pub github_api_base: String,
    // Google
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    pub google_auth_url: String,
    pub google_token_url: String,
    pub google_userinfo_url: String,
    pub google_jwks_url: String,
}

impl OAuthConfig {
    pub fn from_env() -> Self {
        let non_empty = |v: Result<String, _>| {
            v.ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };
        // Endpoint bases are overridable for staging, offline dev, and
        // GitHub Enterprise Server (e.g. https://ghe.example.com/api/v3).
        let url_or = |var: &str, default: &str| {
            non_empty(std::env::var(var)).unwrap_or_else(|| default.to_string())
        };
        Self {
            github_client_id: non_empty(std::env::var("ALGO_GITHUB_CLIENT_ID")),
            github_device_code_url: url_or(
                "ALGO_GITHUB_DEVICE_CODE_URL",
                "https://github.com/login/device/code",
            ),
            github_access_token_url: url_or(
                "ALGO_GITHUB_ACCESS_TOKEN_URL",
                "https://github.com/login/oauth/access_token",
            ),
            github_api_base: url_or("ALGO_GITHUB_API_BASE", "https://api.github.com"),
            google_client_id: non_empty(std::env::var("ALGO_GOOGLE_CLIENT_ID")),
            google_client_secret: non_empty(std::env::var("ALGO_GOOGLE_CLIENT_SECRET")),
            google_auth_url: url_or(
                "ALGO_GOOGLE_AUTH_URL",
                "https://accounts.google.com/o/oauth2/v2/auth",
            ),
            google_token_url: url_or(
                "ALGO_GOOGLE_TOKEN_URL",
                "https://oauth2.googleapis.com/token",
            ),
            google_userinfo_url: url_or(
                "ALGO_GOOGLE_USERINFO_URL",
                "https://openidconnect.googleapis.com/v1/userinfo",
            ),
            google_jwks_url: url_or(
                "ALGO_GOOGLE_JWKS_URL",
                "https://www.googleapis.com/oauth2/v3/certs",
            ),
        }
    }

    /// Server-pinned Google redirect URI (exact match, RFC 9700 §4.1).
    ///
    /// The CLI listens on exactly this loopback URI. It is NEVER taken from
    /// the caller: an attacker-controlled redirect_uri would let an attacker
    /// complete a login for a victim's account and steal the resulting
    /// session (login-CSRF). Override with `ALGO_GOOGLE_REDIRECT_URI` when
    /// the registered Cloud Console value differs.
    pub fn google_redirect_uri() -> String {
        std::env::var("ALGO_GOOGLE_REDIRECT_URI")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "http://127.0.0.1:51004/oauth2redirect".to_string())
    }

    /// Everything unconfigured: every provider route fails closed with 503.
    /// Used as the safe default in tests that don't touch OAuth.
    #[allow(dead_code)]
    pub fn disabled() -> Self {
        Self {
            github_client_id: None,
            github_device_code_url: "http://127.0.0.1:9/unused".to_string(),
            github_access_token_url: "http://127.0.0.1:9/unused".to_string(),
            github_api_base: "http://127.0.0.1:9/unused".to_string(),
            google_client_id: None,
            google_client_secret: None,
            google_auth_url: "http://127.0.0.1:9/unused".to_string(),
            google_token_url: "http://127.0.0.1:9/unused".to_string(),
            google_userinfo_url: "http://127.0.0.1:9/unused".to_string(),
            google_jwks_url: "http://127.0.0.1:9/unused".to_string(),
        }
    }
}

impl Default for OAuthConfig {
    fn default() -> Self {
        Self::from_env()
    }
}
