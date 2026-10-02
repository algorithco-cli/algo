use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    limit::RequestBodyLimitLayer,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

mod account_auth;
mod audit;
mod auth;
mod email;
mod entitlements;
mod github;
mod google;
mod oauth_config;
mod plans;
mod policy;
mod provider_http;
mod session;
mod stats;
mod subscriptions;
mod test_sync;
mod verify;

use audit::{ingest_audit, list_audit, AuditError, ListAuditQuery};
use auth::AuthService;
use policy::PolicyStore;
use stats::{query_stats_series, Granularity, StatsSeriesQuery};
use verify::PolicyBundle;

// ── State & rate limit ──────────────────────────────────────────────

#[derive(Clone)]
struct AppState {
    policy_store: Arc<PolicyStore>,
    oauth: Arc<oauth_config::OAuthConfig>,
    http: reqwest::Client,
    auth: Arc<AuthService>,
    account_entitlements: Option<Arc<entitlements::AccountEntitlementService>>,
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("algo-backend/0.1")
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

static RATE_LIMIT_STORE: OnceLock<Mutex<HashMap<String, (u32, Instant)>>> = OnceLock::new();

fn rate_store() -> &'static Mutex<HashMap<String, (u32, Instant)>> {
    RATE_LIMIT_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_rate_store() -> std::sync::MutexGuard<'static, HashMap<String, (u32, Instant)>> {
    match rate_store().lock() {
        Ok(g) => g,
        Err(poisoned) => {
            tracing::warn!("rate store mutex poisoned; recovering inner");
            poisoned.into_inner()
        }
    }
}

const RATE_LIMIT_PER_MINUTE: u32 = 100;
const MAX_POLICY_CONTENT_LEN: usize = 64 * 1024;
const MAX_B64_LEN: usize = 256 * 1024;
const MAX_HISTORY_IDS: usize = 1000;
const MAX_VERSION_LEN: usize = 64;
const MAX_ORG_ID_LEN: usize = 128;

/// In-handler rate-limit check (fail-closed on overload).
/// Keyed per auth identity when available, else per endpoint.
/// Returns an error response when over limit; callers map to 429+Retry-After.
fn check_rate_limit(key: &str) -> Result<(), (StatusCode, serde_json::Value)> {
    check_rate_limit_key(key, RATE_LIMIT_PER_MINUTE)
}

/// Rate check with an explicit per-minute budget (plan multipliers scale the
/// per-org buckets on gated routes). Always fails closed with 429.
fn check_rate_limit_key(key: &str, limit: u32) -> Result<(), (StatusCode, serde_json::Value)> {
    let mut store = lock_rate_store();
    // Opportunistic eviction to bound memory: drop stale windows.
    if store.len() > 10_000 {
        let now = Instant::now();
        store.retain(|_, (_, t)| now.duration_since(*t) < Duration::from_secs(120));
    }
    let now = Instant::now();
    let entry = store.entry(key.to_string()).or_insert((0, now));
    if now.duration_since(entry.1) > Duration::from_secs(60) {
        *entry = (0, now);
    }
    entry.0 += 1;
    if entry.0 > limit {
        tracing::warn!("rate limit exceeded for {key}: {}", entry.0);
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            serde_json::json!({"error":"rate limited", "retry_after_secs": 60}),
        ));
    }
    Ok(())
}

/// Per-org bucket for gated routes, scaled by the org's plan multiplier.
/// Unknown orgs / invalid entitlements get the base budget (fail-closed
/// direction: never more than paid for).
fn check_rate_limit_org(
    org_id: &str,
    endpoint: &str,
    multiplier: i64,
) -> Result<(), (StatusCode, serde_json::Value)> {
    let budget = RATE_LIMIT_PER_MINUTE.saturating_mul(multiplier.clamp(1, 100) as u32);
    check_rate_limit_key(&format!("org:{org_id}:{endpoint}"), budget)
}

fn rate_key(headers: &HeaderMap, endpoint: &str) -> String {
    // Per-token bucket when authed, else per-endpoint (avoids global DoS key).
    // The credential is HASHED: raw tokens must never reach logs/metrics —
    // this key is emitted in `tracing::warn!` on overload.
    use sha2::{Digest, Sha256};
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("anon");
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let digest = hasher.finalize();
    let mut fingerprint = String::with_capacity(16);
    for b in digest.iter().take(8) {
        fingerprint.push_str(&format!("{b:02x}"));
    }
    format!("{endpoint}:{fingerprint}")
}

// Tower layer stub for rate limiting (MVP: in-memory counter, proves structure).
// Real Valkey-backed limit would be here; this stub counts per-process.
#[derive(Clone, Default)]
struct RateLimitLayer;

impl<S> tower::Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService { inner }
    }
}

#[derive(Clone)]
struct RateLimitService<S> {
    inner: S,
}

impl<S, Req> tower::Service<Req> for RateLimitService<S>
where
    S: tower::Service<Req>,
    Req: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Req) -> Self::Future {
        // Counter layer (observability only). Hard enforcement happens
        // per-handler via `check_rate_limit` so overload fails closed with 429.
        // A Valkey-backed distributed limiter replaces this per P3-03.
        {
            let mut store = lock_rate_store();
            let now = Instant::now();
            let entry = store.entry("global".to_string()).or_insert((0, now));
            if now.duration_since(entry.1) > Duration::from_secs(60) {
                *entry = (0, now);
            }
            entry.0 = entry.0.saturating_add(1);
            if entry.0 > RATE_LIMIT_PER_MINUTE {
                tracing::warn!("rate limit layer: over limit {}", entry.0);
            }
        }
        self.inner.call(req)
    }
}

// WAL-like queue is audit::WAL_QUEUE (VecDeque) — append on ingest. There is
// intentionally no HTTP drain route (cross-org by construction, C2). A
// durable worker replaces this in-memory scaffold in the durability phase.

// ── Request / Response DTOs ─────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CreateOrgPayload {
    org_name: String,
    /// Accepted for backwards-compat but IGNORED: owner is derived from the
    /// authenticated caller (prevents owner spoofing).
    #[allow(dead_code)]
    owner_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct CreateOrgResponse {
    org_id: String,
    org_name: String,
    owner_id: String,
}

#[derive(Debug, Deserialize)]
struct EmailSignupPayload {
    email: String,
    password: String,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EmailLoginPayload {
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct PublishPolicyPayload {
    org_id: Option<String>,
    content: Option<String>,
    // Alternative: raw bundle fields for direct testing
    version: Option<String>,
    signed_bytes_b64: Option<String>,
    sig_b64: Option<String>,
}

#[derive(Debug, Serialize)]
struct PublishPolicyResponse {
    version: String,
    ok: bool,
}

#[derive(Debug, Deserialize)]
struct StatsQueryParams {
    org_id: Option<String>,
    from: Option<String>,
    to: Option<String>,
    granularity: Option<String>,
    limit: Option<usize>,
    top_n: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ListAuditParams {
    org_id: Option<String>,
    limit: Option<usize>,
    cursor: Option<String>,
    decision: Option<String>,
    tool_kind: Option<String>,
    from: Option<String>,
    to: Option<String>,
}

/// In-memory org registry (session-lifetime; restart wipes it).
/// Populated by POST /v1/orgs so GET /v1/orgs[/:id] can serve the dashboard.
#[derive(Debug, Clone, Serialize)]
struct StoredOrg {
    org_id: String,
    org_name: String,
    owner_id: String,
    created_at: String,
}

static ORG_STORE: OnceLock<Mutex<HashMap<String, StoredOrg>>> = OnceLock::new();

fn org_store() -> &'static Mutex<HashMap<String, StoredOrg>> {
    ORG_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_org_store() -> std::sync::MutexGuard<'static, HashMap<String, StoredOrg>> {
    match org_store().lock() {
        Ok(g) => g,
        Err(poisoned) => {
            tracing::warn!("org store mutex poisoned; recovering inner");
            poisoned.into_inner()
        }
    }
}

#[allow(dead_code)]
fn clear_org_store() {
    lock_org_store().clear();
}

/// Org membership gate (C2): the caller must own the org. Strangers and
/// unknown ids are INDISTINGUISHABLE (`404 org not found`, no oracle) —
/// same contract as `get_org_handler`. Callers validate `org_id` shape
/// first (empty/oversize → 400); this function enforces membership.
fn require_org_member(caller: &str, org_id: &str) -> Result<(), (StatusCode, serde_json::Value)> {
    match lock_org_store().get(org_id) {
        Some(org) if org.owner_id == caller => Ok(()),
        _ => Err((
            StatusCode::NOT_FOUND,
            serde_json::json!({"error": "org not found"}),
        )),
    }
}

/// Require a non-empty, length-capped `org_id` (fail-closed 400).
/// Removes cross-org aggregation: no handler may operate without an org.
fn require_org_param(org_id: Option<&str>) -> Result<String, (StatusCode, serde_json::Value)> {
    match org_id.map(str::trim) {
        Some(org) if !org.is_empty() && org.len() <= MAX_ORG_ID_LEN => Ok(org.to_string()),
        _ => Err((
            StatusCode::BAD_REQUEST,
            serde_json::json!({"error": "org_id required"}),
        )),
    }
}

// ── Handlers ────────────────────────────────────────────────────────

async fn health_handler() -> impl IntoResponse {
    Json(serde_json::json!({"status":"ok","service":"algo-backend"}))
}

/// Self-contained API console (no build step, no CDN — works offline).
/// Embedded at compile time so the binary stays a single artifact.
static API_CONSOLE_HTML: &str = include_str!("../static/api-console.html");

async fn api_console_handler() -> impl IntoResponse {
    Html(API_CONSOLE_HTML)
}

async fn create_org_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateOrgPayload>,
) -> impl IntoResponse {
    // P3-03: org creation requires auth; owner is derived from caller, never
    // trusted from the client payload.
    let caller = match state.auth.authenticate(&headers).await {
        Ok(user) => user.sub,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error":"unauthorized"})),
            )
                .into_response();
        }
    };
    if let Err(resp) =
        check_rate_limit(&rate_key(&headers, "create_org")).map_err(|(s, b)| (s, Json(b)))
    {
        return resp.into_response();
    }
    let name = payload.org_name.trim().to_string();
    if name.is_empty() || name.len() > 128 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"org_name required (1..128 chars)"})),
        )
            .into_response();
    }
    if name.chars().any(|c| c.is_control()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"org_name invalid"})),
        )
            .into_response();
    }
    let org_id = format!("org_{}", uuid::Uuid::new_v4());
    // Owner bound to authenticated caller; client-supplied owner_id is ignored
    // (prevents owner spoofing, C2).
    let resp = CreateOrgResponse {
        org_id: org_id.clone(),
        org_name: name.clone(),
        owner_id: caller.clone(),
    };
    // Session-lifetime registry so GET /v1/orgs[/:id] can serve the dashboard.
    // In-memory MVP: restart wipes it (documented in README).
    lock_org_store().insert(
        org_id.clone(),
        StoredOrg {
            org_id,
            org_name: name,
            owner_id: caller,
            created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        },
    );
    (StatusCode::CREATED, Json(resp)).into_response()
}

async fn get_org_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(org_id): Path<String>,
) -> impl IntoResponse {
    let caller = match authed_caller(&state, &headers).await {
        Ok(c) => c,
        Err((s, b)) => return (s, b).into_response(),
    };
    if let Err(resp) =
        check_rate_limit(&rate_key(&headers, "get_org")).map_err(|(s, b)| (s, Json(b)))
    {
        return resp.into_response();
    }
    // No-oracle 404: strangers and unknown ids are indistinguishable (same as
    // subscriptions). Only the owner may read the org.
    match lock_org_store().get(&org_id) {
        Some(org) if org.owner_id == caller => {
            (StatusCode::OK, Json(serde_json::json!({"org": org}))).into_response()
        }
        _ => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "org not found"})),
        )
            .into_response(),
    }
}

async fn list_orgs_handler(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let caller = match authed_caller(&state, &headers).await {
        Ok(c) => c,
        Err((s, b)) => return (s, b).into_response(),
    };
    if let Err(resp) =
        check_rate_limit(&rate_key(&headers, "list_orgs")).map_err(|(s, b)| (s, Json(b)))
    {
        return resp.into_response();
    }
    let orgs: Vec<StoredOrg> = lock_org_store()
        .values()
        .filter(|o| o.owner_id == caller)
        .cloned()
        .collect();
    (
        StatusCode::OK,
        Json(serde_json::json!({"orgs": orgs, "total": orgs.len()})),
    )
        .into_response()
}

async fn email_signup_handler(Json(payload): Json<EmailSignupPayload>) -> impl IntoResponse {
    if let Err((s, b)) = check_rate_limit("email_signup:anon") {
        return (s, Json(b)).into_response();
    }
    match email::signup(&payload.email, &payload.password, payload.name.as_deref()) {
        Ok(session) => (StatusCode::CREATED, Json(session)).into_response(),
        Err(email::EmailError::Exists) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error": email::EmailError::Exists.to_string()})),
        )
            .into_response(),
        Err(email::EmailError::Internal) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": email::EmailError::Internal.to_string()})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn email_login_handler(Json(payload): Json<EmailLoginPayload>) -> impl IntoResponse {
    if let Err((s, b)) = check_rate_limit("email_login:anon") {
        return (s, Json(b)).into_response();
    }
    match email::login(&payload.email, &payload.password) {
        Ok(session) => (StatusCode::OK, Json(session)).into_response(),
        Err(email::EmailError::Internal) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": email::EmailError::Internal.to_string()})),
        )
            .into_response(),
        // Unknown email and wrong password share one message (no oracle).
        Err(_) => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": email::EmailError::InvalidCredentials.to_string()})),
        )
            .into_response(),
    }
}

// ── OAuth login (GitHub device flow + Google OIDC) ───────────────────
// Backend-only: the CLI opens browsers / shows codes; these routes broker the
// provider exchange and mint short-lived backend session JWTs. Provider is
// never trusted blindly: GitHub tokens are validated via GET /user, Google
// ID tokens via JWKS (aud/iss/exp). Unconfigured → 503, provider down → 502.

#[derive(Debug, Deserialize)]
struct GithubPollPayload {
    device_code: String,
}

#[derive(Debug, Deserialize)]
struct GithubValidatePayload {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct GoogleUrlPayload {
    /// Optional compatibility echo: when present it MUST equal the
    /// server-pinned redirect URI exactly, else 400. The server never uses a
    /// caller-supplied redirect target (login-CSRF prevention, RFC 9700 §4.1).
    redirect_uri: Option<String>,
    scopes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleCallbackPayload {
    code: String,
    state: String,
}

#[derive(Debug, Deserialize)]
struct GoogleVerifyPayload {
    id_token: String,
    /// REQUIRED: the nonce from the authentication request that produced this
    /// token. Without it the token cannot be bound to a login transaction
    /// (OIDC Core replay protection) and verification is refused.
    expected_nonce: Option<String>,
}

fn github_err(e: github::GithubError) -> (StatusCode, serde_json::Value) {
    use github::GithubError as E;
    match e {
        E::NotConfigured => (
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({"error": e.to_string()}),
        ),
        E::Unauthorized => (
            StatusCode::UNAUTHORIZED,
            serde_json::json!({"error": e.to_string()}),
        ),
        E::SlowDown(s) => (
            StatusCode::TOO_MANY_REQUESTS,
            serde_json::json!({"error": e.to_string(), "retry_after_secs": s}),
        ),
        E::Transport => (
            StatusCode::BAD_GATEWAY,
            serde_json::json!({"error": e.to_string()}),
        ),
        _ => (
            StatusCode::BAD_REQUEST,
            serde_json::json!({"error": e.to_string()}),
        ),
    }
}

fn google_err(e: google::GoogleError) -> (StatusCode, serde_json::Value) {
    use google::GoogleError as E;
    match e {
        E::NotConfigured => (
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({"error": e.to_string()}),
        ),
        E::Unauthorized => (
            StatusCode::UNAUTHORIZED,
            serde_json::json!({"error": e.to_string()}),
        ),
        E::Transport => (
            StatusCode::BAD_GATEWAY,
            serde_json::json!({"error": e.to_string()}),
        ),
        _ => (
            StatusCode::BAD_REQUEST,
            serde_json::json!({"error": e.to_string()}),
        ),
    }
}

async fn github_device_handler(State(state): State<AppState>) -> impl IntoResponse {
    if let Err((s, b)) = check_rate_limit("github_device:anon") {
        return (s, Json(b)).into_response();
    }
    match github::request_device_code(&state.http, &state.oauth).await {
        Ok(init) => (
            StatusCode::OK,
            Json(serde_json::to_value(init).unwrap_or_default()),
        )
            .into_response(),
        Err(e) => {
            let (s, b) = github_err(e);
            (s, Json(b)).into_response()
        }
    }
}

async fn github_poll_handler(
    State(state): State<AppState>,
    Json(payload): Json<GithubPollPayload>,
) -> impl IntoResponse {
    if let Err((s, b)) = check_rate_limit("github_poll:anon") {
        return (s, Json(b)).into_response();
    }
    match github::poll_access_token(&state.http, &state.oauth, &payload.device_code).await {
        Ok(github::PollOutcome::Pending) => {
            (StatusCode::OK, Json(serde_json::json!({"pending": true}))).into_response()
        }
        Ok(github::PollOutcome::Authorized(t)) => {
            match github::fetch_user(&state.http, &state.oauth, &t.access_token).await {
                Ok(user) => {
                    let sub = format!("github:{}", user.id);
                    let session = session::mint_session("github", &sub, Some(&user.login), None);
                    (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "pending": false,
                            "provider": "github",
                            "login": user.login,
                            "session_token": session,
                            "expires_in": session::SESSION_TTL_SECS,
                            // Provider refresh token stays client-side (BYOK); backend keeps no refresh store.
                            "refresh_token": t.refresh_token,
                            "provider_expires_in": t.expires_in,
                        })),
                    )
                        .into_response()
                }
                Err(e) => {
                    let (s, b) = github_err(e);
                    (s, Json(b)).into_response()
                }
            }
        }
        Err(e) => {
            let (s, b) = github_err(e);
            (s, Json(b)).into_response()
        }
    }
}

async fn github_validate_handler(
    State(state): State<AppState>,
    Json(payload): Json<GithubValidatePayload>,
) -> impl IntoResponse {
    if let Err((s, b)) = check_rate_limit("github_validate:anon") {
        return (s, Json(b)).into_response();
    }
    match github::fetch_user(&state.http, &state.oauth, &payload.access_token).await {
        Ok(user) => {
            let sub = format!("github:{}", user.id);
            let session = session::mint_session("github", &sub, Some(&user.login), None);
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "valid": true,
                    "provider": "github",
                    "login": user.login,
                    "session_token": session,
                    "expires_in": session::SESSION_TTL_SECS,
                })),
            )
                .into_response()
        }
        Err(e) => {
            let (s, b) = github_err(e);
            (s, Json(b)).into_response()
        }
    }
}

async fn google_url_handler(
    State(state): State<AppState>,
    Json(payload): Json<GoogleUrlPayload>,
) -> impl IntoResponse {
    if let Err((s, b)) = check_rate_limit("google_url:anon") {
        return (s, Json(b)).into_response();
    }
    match google::build_auth_url(
        &state.oauth,
        payload.redirect_uri.as_deref(),
        payload.scopes.as_deref(),
    ) {
        Ok((auth_url, flow_state)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "auth_url": auth_url,
                "state": flow_state,
                "redirect_uri": oauth_config::OAuthConfig::google_redirect_uri(),
                "expires_in": 600,
            })),
        )
            .into_response(),
        Err(e) => {
            let (s, b) = google_err(e);
            (s, Json(b)).into_response()
        }
    }
}

async fn google_callback_handler(
    State(state): State<AppState>,
    Json(payload): Json<GoogleCallbackPayload>,
) -> impl IntoResponse {
    if let Err((s, b)) = check_rate_limit("google_callback:anon") {
        return (s, Json(b)).into_response();
    }
    let (tokens, nonce, created_epoch) =
        match google::exchange_code(&state.http, &state.oauth, &payload.code, &payload.state).await
        {
            Ok(t) => t,
            Err(e) => {
                let (s, b) = google_err(e);
                return (s, Json(b)).into_response();
            }
        };
    // auth_time must prove fresh authentication for THIS flow (max_age=0).
    let earliest_auth = created_epoch.saturating_sub(120);
    let claims = match google::verify_id_token_live(
        &state.http,
        &state.oauth,
        &tokens.id_token,
        &nonce,
        tokens.access_token.as_deref(),
        earliest_auth,
    )
    .await
    {
        Ok(c) => c,
        Err(e) => {
            let (s, b) = google_err(e);
            return (s, Json(b)).into_response();
        }
    };
    // Best-effort userinfo enrichment (ID token stays the trust root).
    let mut email = claims.email.clone();
    let mut name = claims.name.clone();
    if let Some(access) = tokens.access_token.as_deref() {
        if let Ok(info) = google::fetch_userinfo(&state.http, &state.oauth, access).await {
            if email.is_none() {
                email = info.email;
            }
            if name.is_none() {
                name = info.name;
            }
        }
    }
    let sub = format!("google:{}", claims.sub);
    let session = session::mint_session("google", &sub, name.as_deref(), email.as_deref());
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "provider": "google",
            "sub": claims.sub,
            "email": email,
            "name": name,
            "session_token": session,
            "expires_in": session::SESSION_TTL_SECS,
            // Provider refresh token stays client-side (BYOK); backend keeps no refresh store.
            "refresh_token": tokens.refresh_token,
            "provider_expires_in": tokens.expires_in,
        })),
    )
        .into_response()
}

async fn google_verify_handler(
    State(state): State<AppState>,
    Json(payload): Json<GoogleVerifyPayload>,
) -> impl IntoResponse {
    if let Err((s, b)) = check_rate_limit("google_verify:anon") {
        return (s, Json(b)).into_response();
    }
    if payload.id_token.len() > 8192 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "id_token too large"})),
        )
            .into_response();
    }
    let Some(expected_nonce) = payload.expected_nonce.as_deref().filter(|s| !s.is_empty()) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "expected_nonce required"})),
        )
            .into_response();
    };
    if expected_nonce.len() > 256 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "expected_nonce too long"})),
        )
            .into_response();
    }
    // Direct-verify freshness window: no flow anchor exists, so the token must
    // prove recent issuance AND recent authentication (15 min each).
    let now = chrono::Utc::now().timestamp();
    match google::verify_id_token_live(
        &state.http,
        &state.oauth,
        &payload.id_token,
        expected_nonce,
        None,
        now.saturating_sub(900),
    )
    .await
    {
        Ok(claims) => {
            let sub = format!("google:{}", claims.sub);
            let session = session::mint_session(
                "google",
                &sub,
                claims.name.as_deref(),
                claims.email.as_deref(),
            );
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "valid": true,
                    "provider": "google",
                    "sub": claims.sub,
                    "email": claims.email,
                    "session_token": session,
                    "expires_in": session::SESSION_TTL_SECS,
                })),
            )
                .into_response()
        }
        Err(e) => {
            let (s, b) = google_err(e);
            (s, Json(b)).into_response()
        }
    }
}

// ── Plans & subscriptions (self-managed billing; MoR deferred) ──────
// Enforcement reads entitlements (plan bundles), never tier strings.
// Core protection (publish/ingest/stats) is NEVER paywalled: gating paid
// features must not weaken the guard. Paid gates: dry-run (pro+),
// audit export (team), seats caps, per-org rate budgets.

#[derive(Debug, Deserialize)]
struct CreateSubPayload {
    org_id: String,
    tier: String,
    cycle: String,
    seats: i64,
}

#[derive(Debug, Deserialize)]
struct ChangeSubPayload {
    tier: Option<String>,
    seats: Option<i64>,
    cycle: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CancelSubPayload {
    at_period_end: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct SeatsPayload {
    seats: i64,
}

#[derive(Debug, Deserialize)]
struct SubQuery {
    org_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EntitlementQuery {
    org_id: Option<String>,
    feature: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExportQuery {
    org_id: Option<String>,
    limit: Option<usize>,
}

/// Gate a paid feature for an org AND caller. Unknown orgs, non-owners,
/// unknown features, lapsed subscriptions → 402 with an upgrade hint
/// (never 200, never allow). Non-owners resolve exactly like strangers:
/// the paywall cannot be spent with someone else's org_id.
struct FeatureGrant {
    rate_multiplier: i64,
}

async fn require_feature(
    state: &AppState,
    org_id: &str,
    caller: &auth::AuthenticatedUser,
    feature: &str,
) -> Result<FeatureGrant, (StatusCode, serde_json::Value)> {
    if org_id.trim().is_empty() || org_id.len() > MAX_ORG_ID_LEN {
        return Err((
            StatusCode::PAYMENT_REQUIRED,
            serde_json::json!({"error": "org_id required", "upgrade": {"tier": "pro", "feature": feature}}),
        ));
    }
    if let Some(service) = &state.account_entitlements {
        require_org_member(&caller.sub, org_id)?;
        let ent = service.resolve(caller).await;
        if ent.has_feature(feature) {
            return Ok(FeatureGrant {
                rate_multiplier: ent.rate_multiplier(),
            });
        }
        let tier = entitlements::min_plan_for_feature(feature).unwrap_or("pro");
        return Err((
            StatusCode::PAYMENT_REQUIRED,
            serde_json::json!({"error": "account entitlement required", "upgrade": {"tier": tier, "feature": feature}}),
        ));
    }
    let ent = subscriptions::resolve_entitlement(org_id, &caller.sub, Some(feature));
    if ent.valid && ent.features.get(feature) == Some(true) {
        return Ok(FeatureGrant {
            rate_multiplier: ent.rate_multiplier,
        });
    }
    let tier = plans::FeatureSet::min_tier_for(feature).unwrap_or(plans::TIER_PRO);
    Err((
        StatusCode::PAYMENT_REQUIRED,
        serde_json::json!({"error": ent.upgrade_hint.clone().unwrap_or_else(|| "subscription required".to_string()), "upgrade": {"tier": tier, "feature": feature}}),
    ))
}

/// Authenticate and return the caller identity (token or session sub).
/// Every billing handler binds records to this identity (owner).
async fn authed_caller(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    state
        .auth
        .authenticate(headers)
        .await
        .map(|user| user.sub)
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "unauthorized"})),
            )
        })
}

async fn authed_user(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<auth::AuthenticatedUser, (StatusCode, Json<serde_json::Value>)> {
    state.auth.authenticate(headers).await.map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "unauthorized"})),
        )
    })
}

fn sub_err(e: subscriptions::SubError) -> (StatusCode, serde_json::Value) {
    use subscriptions::SubError as E;
    match e {
        E::UnknownSubscription => (
            StatusCode::NOT_FOUND,
            serde_json::json!({"error": e.to_string()}),
        ),
        E::OverSeatCap { .. } | E::CanceledTerminal | E::AlreadySubscribed => (
            StatusCode::CONFLICT,
            serde_json::json!({"error": e.to_string()}),
        ),
        _ => (
            StatusCode::BAD_REQUEST,
            serde_json::json!({"error": e.to_string()}),
        ),
    }
}

async fn plans_handler() -> impl IntoResponse {
    // Public catalog (prices are marketing-visible; no auth needed).
    (
        StatusCode::OK,
        Json(serde_json::json!({"plans": plans::catalog()})),
    )
        .into_response()
}

async fn create_sub_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateSubPayload>,
) -> impl IntoResponse {
    let caller = match authed_caller(&state, &headers).await {
        Ok(c) => c,
        Err((s, b)) => return (s, b).into_response(),
    };
    if let Err((s, b)) = check_rate_limit(&rate_key(&headers, "sub_create")) {
        return (s, Json(b)).into_response();
    }
    if payload.org_id.trim().is_empty() || payload.org_id.len() > MAX_ORG_ID_LEN {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "org_id invalid"})),
        )
            .into_response();
    }
    match subscriptions::create_subscription(
        &payload.org_id,
        &payload.tier,
        &payload.cycle,
        payload.seats,
        &caller,
    ) {
        Ok(sub) => (StatusCode::CREATED, Json(sub)).into_response(),
        Err(e) => {
            let (s, b) = sub_err(e);
            (s, Json(b)).into_response()
        }
    }
}

async fn get_sub_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<SubQuery>,
) -> impl IntoResponse {
    let caller = match authed_caller(&state, &headers).await {
        Ok(c) => c,
        Err((s, b)) => return (s, b).into_response(),
    };
    let Some(org_id) = params.org_id.filter(|s| !s.trim().is_empty()) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "org_id required"})),
        )
            .into_response();
    };
    // Owner-gated: strangers see the free shape (no oracle).
    match subscriptions::get_by_org(&org_id, &caller) {
        Some(sub) => (
            StatusCode::OK,
            Json(serde_json::to_value(sub).unwrap_or_default()),
        )
            .into_response(),
        None => (
            StatusCode::OK,
            Json(serde_json::json!({"tier": "free", "status": "none"})),
        )
            .into_response(),
    }
}

async fn change_sub_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<ChangeSubPayload>,
) -> impl IntoResponse {
    let caller = match authed_caller(&state, &headers).await {
        Ok(c) => c,
        Err((s, b)) => return (s, b).into_response(),
    };
    if let Err((s, b)) = check_rate_limit(&rate_key(&headers, "sub_change")) {
        return (s, Json(b)).into_response();
    }
    if id.len() > MAX_VERSION_LEN {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "id too long"})),
        )
            .into_response();
    }
    match subscriptions::change_subscription(
        &id,
        &caller,
        payload.tier.as_deref(),
        payload.seats,
        payload.cycle.as_deref(),
    ) {
        Ok((sub, scheduled)) => (
            StatusCode::OK,
            Json(serde_json::json!({"subscription": sub, "scheduled": scheduled})),
        )
            .into_response(),
        Err(e) => {
            let (s, b) = sub_err(e);
            (s, Json(b)).into_response()
        }
    }
}

async fn cancel_sub_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<CancelSubPayload>,
) -> impl IntoResponse {
    let caller = match authed_caller(&state, &headers).await {
        Ok(c) => c,
        Err((s, b)) => return (s, b).into_response(),
    };
    if let Err((s, b)) = check_rate_limit(&rate_key(&headers, "sub_cancel")) {
        return (s, Json(b)).into_response();
    }
    if id.len() > MAX_VERSION_LEN {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "id too long"})),
        )
            .into_response();
    }
    let at_period_end = payload.at_period_end.unwrap_or(true);
    match subscriptions::cancel_subscription(&id, &caller, at_period_end) {
        Ok(sub) => (StatusCode::OK, Json(sub)).into_response(),
        Err(e) => {
            let (s, b) = sub_err(e);
            (s, Json(b)).into_response()
        }
    }
}

async fn activate_sub_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let caller = match authed_caller(&state, &headers).await {
        Ok(c) => c,
        Err((s, b)) => return (s, b).into_response(),
    };
    if let Err((s, b)) = check_rate_limit(&rate_key(&headers, "sub_activate")) {
        return (s, Json(b)).into_response();
    }
    if id.len() > MAX_VERSION_LEN {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "id too long"})),
        )
            .into_response();
    }
    match subscriptions::activate_subscription(&id, &caller) {
        Ok(sub) => (StatusCode::OK, Json(sub)).into_response(),
        Err(e) => {
            let (s, b) = sub_err(e);
            (s, Json(b)).into_response()
        }
    }
}

async fn seats_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<SeatsPayload>,
) -> impl IntoResponse {
    let caller = match authed_caller(&state, &headers).await {
        Ok(c) => c,
        Err((s, b)) => return (s, b).into_response(),
    };
    if let Err((s, b)) = check_rate_limit(&rate_key(&headers, "sub_seats")) {
        return (s, Json(b)).into_response();
    }
    if id.len() > MAX_VERSION_LEN {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "id too long"})),
        )
            .into_response();
    }
    match subscriptions::set_seats(&id, &caller, payload.seats) {
        Ok(sub) => (StatusCode::OK, Json(sub)).into_response(),
        Err(e) => {
            let (s, b) = sub_err(e);
            (s, Json(b)).into_response()
        }
    }
}

async fn entitlement_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<EntitlementQuery>,
) -> impl IntoResponse {
    let caller = match authed_user(&state, &headers).await {
        Ok(c) => c,
        Err((s, b)) => return (s, b).into_response(),
    };
    if let Some(service) = &state.account_entitlements {
        let entitlement = service.resolve(&caller).await;
        return (StatusCode::OK, Json(entitlement)).into_response();
    }
    let Some(org_id) = params.org_id.filter(|s| !s.trim().is_empty()) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "org_id required"})),
        )
            .into_response();
    };
    if org_id.len() > MAX_ORG_ID_LEN {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "org_id invalid"})),
        )
            .into_response();
    }
    // Owner-gated inside resolve: strangers see the free shape.
    let ent = subscriptions::resolve_entitlement(&org_id, &caller.sub, params.feature.as_deref());
    (StatusCode::OK, Json(ent)).into_response()
}

async fn refresh_entitlement_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let caller = match authed_user(&state, &headers).await {
        Ok(caller) => caller,
        Err((status, body)) => return (status, body).into_response(),
    };
    let Some(service) = &state.account_entitlements else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match service.refresh(&caller).await {
        Ok(entitlement) => (StatusCode::OK, Json(entitlement)).into_response(),
        Err(entitlements::RefreshError::RateLimited { retry_after_secs }) => {
            let retry_after = HeaderValue::from_str(&retry_after_secs.to_string())
                .unwrap_or_else(|_| HeaderValue::from_static("1"));
            (
                StatusCode::TOO_MANY_REQUESTS,
                [(header::RETRY_AFTER, retry_after)],
                Json(serde_json::json!({"error": "refresh rate limited"})),
            )
                .into_response()
        }
    }
}

async fn billing_webhook_handler() -> impl IntoResponse {
    // Reserved for the deferred MoR vendor. No fake processing: an explicit
    // 501 beats silently dropping provider events (fail-closed, honest).
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({"error": "billing provider not configured (deferred MoR)"})),
    )
        .into_response()
}

async fn export_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<ExportQuery>,
) -> impl IntoResponse {
    let caller = match authed_user(&state, &headers).await {
        Ok(c) => c,
        Err((s, b)) => return (s, b).into_response(),
    };
    let Some(org_id) = params.org_id.filter(|s| !s.trim().is_empty()) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "org_id required"})),
        )
            .into_response();
    };
    // Team gate (SIEM export) + per-org rate budget × plan multiplier.
    // Caller must own the org: no spending someone else's plan.
    let ent = match require_feature(&state, &org_id, &caller, "siem_export").await {
        Ok(e) => e,
        Err((s, b)) => return (s, Json(b)).into_response(),
    };
    if let Err((s, b)) = check_rate_limit_org(&org_id, "export", ent.rate_multiplier) {
        return (s, Json(b)).into_response();
    }
    let limit = params.limit.unwrap_or(1000).min(10_000);
    let records: Vec<_> = audit::all_records()
        .into_iter()
        .filter(|r| r.org_id.as_deref() == Some(org_id.as_str()))
        .take(limit.saturating_add(1))
        .collect();
    let truncated = records.len() > limit;
    let records: Vec<_> = records.into_iter().take(limit).collect();
    let total = records.len();
    (
        StatusCode::OK,
        Json(serde_json::json!({"records": records, "total": total, "truncated": truncated})),
    )
        .into_response()
}

async fn publish_policy_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<PublishPolicyPayload>,
) -> impl IntoResponse {
    // Auth already checked via middleware; double-check for direct handler tests.
    let caller = match state.auth.authenticate(&headers).await {
        Ok(user) => user.sub,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "unauthorized"})),
            )
                .into_response();
        }
    };
    if let Err((s, b)) = check_rate_limit(&rate_key(&headers, "policy_publish")) {
        return (s, Json(b)).into_response();
    }
    // C2: org_id required (no silent "default" cross-org scope) + owner check.
    let org_id = match require_org_param(payload.org_id.as_deref()) {
        Ok(o) => o,
        Err((s, b)) => return (s, Json(b)).into_response(),
    };
    if let Err((s, b)) = require_org_member(&caller, &org_id) {
        return (s, Json(b)).into_response();
    }
    // If raw bundle provided, verify_and_apply directly; else create new bundle from content.
    if let (Some(version), Some(signed_b64), Some(sig_b64)) =
        (payload.version, payload.signed_bytes_b64, payload.sig_b64)
    {
        if version.len() > MAX_VERSION_LEN {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error":"version too long"})),
            )
                .into_response();
        }
        if signed_b64.len() > MAX_B64_LEN || sig_b64.len() > 1024 {
            return (
                StatusCode::PAYLOAD_TOO_LARGE,
                Json(serde_json::json!({"error":"bundle too large"})),
            )
                .into_response();
        }
        use base64::{engine::general_purpose::STANDARD as B64, Engine};
        let signed_bytes = match B64.decode(signed_b64) {
            Ok(b) => b,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "bad signed_bytes_b64"})),
                )
                    .into_response();
            }
        };
        if signed_bytes.len() > MAX_POLICY_CONTENT_LEN {
            return (
                StatusCode::PAYLOAD_TOO_LARGE,
                Json(serde_json::json!({"error":"signed_bytes too large"})),
            )
                .into_response();
        }
        let sig = match B64.decode(sig_b64) {
            Ok(b) => b,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "bad sig_b64"})),
                )
                    .into_response();
            }
        };
        let bundle = PolicyBundle {
            version,
            signed_bytes,
            sig,
        };
        match state.policy_store.verify_and_apply(&org_id, &bundle) {
            Ok(()) => (
                StatusCode::OK,
                Json(serde_json::json!({"version": bundle.version, "ok": true})),
            )
                .into_response(),
            Err(e) => {
                // Fail-closed: tampered/expired/rollback → 400 (caller maps to ask).
                tracing::warn!("policy verify_and_apply failed");
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": e.to_string()})),
                )
                    .into_response()
            }
        }
    } else {
        let content = payload
            .content
            .unwrap_or_else(|| "default policy".to_string());
        if content.len() > MAX_POLICY_CONTENT_LEN {
            return (
                StatusCode::PAYLOAD_TOO_LARGE,
                Json(serde_json::json!({"error":"content too large"})),
            )
                .into_response();
        }
        let bundle = state.policy_store.publish(&org_id, &content);
        let resp = PublishPolicyResponse {
            version: bundle.version.clone(),
            ok: true,
        };
        (StatusCode::OK, Json(resp)).into_response()
    }
}

async fn get_policy_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(version): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let caller = match state.auth.authenticate(&headers).await {
        Ok(user) => user.sub,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "unauthorized"})),
            )
                .into_response();
        }
    };
    if version.len() > MAX_VERSION_LEN {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"version too long"})),
        )
            .into_response();
    }
    // C2: org_id required + owner check (no cross-org policy reads).
    let org_id = match require_org_param(params.get("org_id").map(|s| s.as_str())) {
        Ok(o) => o,
        Err((s, b)) => return (s, Json(b)).into_response(),
    };
    if let Err((s, b)) = require_org_member(&caller, &org_id) {
        return (s, Json(b)).into_response();
    }
    match state.policy_store.get(&org_id, &version) {
        Some(bundle) => {
            use base64::{engine::general_purpose::STANDARD as B64, Engine};
            let body = serde_json::json!({
                "version": bundle.version,
                "signed_bytes_b64": B64.encode(&bundle.signed_bytes),
                "sig_b64": B64.encode(&bundle.sig),
            });
            (StatusCode::OK, Json(body)).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error":"policy not found"})),
        )
            .into_response(),
    }
}

async fn ingest_audit_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let caller = match state.auth.authenticate(&headers).await {
        Ok(user) => user.sub,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "unauthorized"})),
            )
                .into_response();
        }
    };
    if let Err((s, b)) = check_rate_limit(&rate_key(&headers, "audit_ingest")) {
        return (s, Json(b)).into_response();
    }
    // C2: org_id required + owner check. The payload org binds the record;
    // strangers/unknown orgs get 404 (no oracle, no cross-org writes).
    let org_id = match require_org_param(payload.get("org_id").and_then(|v| v.as_str())) {
        Ok(o) => o,
        Err((s, b)) => return (s, Json(b)).into_response(),
    };
    if let Err((s, b)) = require_org_member(&caller, &org_id) {
        return (s, Json(b)).into_response();
    }
    // Normalize: the gate above already validated org_id; stamp the
    // canonical value so trailing-whitespace variants cannot fork an org.
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("org_id".to_string(), serde_json::Value::String(org_id));
    }
    match ingest_audit(payload) {
        Ok(rec) => (
            StatusCode::OK,
            Json(serde_json::json!({"ok": true, "trace_id": rec.trace_id})),
        )
            .into_response(),
        Err(AuditError::NotRedacted(kind)) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("not redacted: {kind}"), "ok": false})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// Parse an RFC3339 bound; 400 on invalid (fail-closed, explicit).
fn parse_time_bound(
    raw: Option<&str>,
    name: &str,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, (StatusCode, serde_json::Value)> {
    match raw {
        None => Ok(None),
        Some(s) => match chrono::DateTime::parse_from_rfc3339(s) {
            Ok(dt) => Ok(Some(dt.with_timezone(&chrono::Utc))),
            Err(_) => Err((
                StatusCode::BAD_REQUEST,
                serde_json::json!({"error": format!("{name} must be RFC3339")}),
            )),
        },
    }
}

async fn stats_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<StatsQueryParams>,
) -> impl IntoResponse {
    let caller = match state.auth.authenticate(&headers).await {
        Ok(user) => user.sub,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "unauthorized"})),
            )
                .into_response();
        }
    };
    // C2: org_id required (no all-org aggregation) + owner check.
    let org_id = match require_org_param(params.org_id.as_deref()) {
        Ok(o) => o,
        Err((s, b)) => return (s, Json(b)).into_response(),
    };
    if let Err((s, b)) = require_org_member(&caller, &org_id) {
        return (s, Json(b)).into_response();
    }
    let from = match parse_time_bound(params.from.as_deref(), "from") {
        Ok(v) => v,
        Err((s, b)) => return (s, Json(b)).into_response(),
    };
    let to = match parse_time_bound(params.to.as_deref(), "to") {
        Ok(v) => v,
        Err((s, b)) => return (s, Json(b)).into_response(),
    };
    // Server caps: buckets ≤ 1000 (default 30), top_n ≤ 100 (default 10).
    let limit = params.limit.unwrap_or(30).min(1000);
    let top_n = params.top_n.unwrap_or(10).min(100);
    let s = query_stats_series(&StatsSeriesQuery {
        org_id: Some(org_id.as_str()),
        from,
        to,
        granularity: Granularity::parse(params.granularity.as_deref()),
        limit,
        top_n,
    });
    (StatusCode::OK, Json(s)).into_response()
}

/// Proto enum numbers (decision.proto / events.proto) for the REST shim.
fn action_number(decision: &str) -> i32 {
    match decision.to_ascii_lowercase().as_str() {
        "allow" => 1,
        "deny" => 2,
        _ => 3, // ask + unknown → ASK failsafe
    }
}

fn tool_kind_number(tool_kind: Option<&str>) -> i32 {
    match tool_kind {
        Some("shell") => 1,
        Some("edit") => 2,
        Some("write") => 3,
        Some("read") => 4,
        Some("net") => 5,
        _ => 6, // other + absent → OTHER
    }
}

fn source_level_number(source: Option<&str>) -> i32 {
    match source {
        Some("rule") => 1,
        Some("cache") => 2,
        Some("local_model") => 3,
        Some("jev") => 4,
        Some("fallback") => 5,
        _ => 0, // absent → UNSPECIFIED (receiver maps to ASK)
    }
}

/// Stored record → proto AuditRecord JSON view (REST shim).
fn audit_record_view(r: &audit::AuditRecord) -> serde_json::Value {
    let action = action_number(&r.decision);
    serde_json::json!({
        "trace_id": r.trace_id,
        "event_id": r.event_id,
        "timestamp": r.ingested_at,
        "tool_kind": tool_kind_number(r.tool_kind.as_deref()),
        "redacted_payload": r.redacted_event,
        "decision": {
            "action": action,
            "reason": r.reason.clone().unwrap_or_default(),
            "confidence_0_1": r.confidence_0_1.unwrap_or(0.0),
            "source_level": source_level_number(r.source_level.as_deref()),
            "latency_ms": r.latency_ms,
            "policy_version": r.policy_version.clone().unwrap_or_default(),
            "trace_id": r.trace_id,
        },
        "privacy_mode": 2,
        "org_id": r.org_id.clone().unwrap_or_default(),
    })
}

async fn list_audit_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<ListAuditParams>,
) -> impl IntoResponse {
    let caller = match state.auth.authenticate(&headers).await {
        Ok(user) => user.sub,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "unauthorized"})),
            )
                .into_response();
        }
    };
    if let Err((s, b)) = check_rate_limit(&rate_key(&headers, "audit_list")) {
        return (s, Json(b)).into_response();
    }
    // C2: org_id required (no all-org reads) + owner check.
    let org_id = match require_org_param(params.org_id.as_deref()) {
        Ok(o) => o,
        Err((s, b)) => return (s, Json(b)).into_response(),
    };
    if let Err((s, b)) = require_org_member(&caller, &org_id) {
        return (s, Json(b)).into_response();
    }
    // Unknown decision values match nothing per contract (200 empty, not 400).
    let decision = params.decision.as_deref().map(str::to_ascii_lowercase);
    let decision = match decision.as_deref() {
        None | Some("") => None,
        Some("allow") | Some("deny") | Some("ask") => decision,
        Some(_) => {
            return (
                StatusCode::OK,
                Json(serde_json::json!({"records": [], "next_cursor": "", "total": 0, "truncated": false})),
            )
                .into_response();
        }
    };
    let tool_kind = params.tool_kind.as_deref().map(str::to_ascii_lowercase);
    let tool_kind = match tool_kind.as_deref() {
        None | Some("") => None,
        Some("shell") | Some("edit") | Some("write") | Some("read") | Some("net")
        | Some("other") => tool_kind,
        Some(_) => {
            return (
                StatusCode::OK,
                Json(serde_json::json!({"records": [], "next_cursor": "", "total": 0, "truncated": false})),
            )
                .into_response();
        }
    };
    let from = match parse_time_bound(params.from.as_deref(), "from") {
        Ok(v) => v,
        Err((s, b)) => return (s, Json(b)).into_response(),
    };
    let to = match parse_time_bound(params.to.as_deref(), "to") {
        Ok(v) => v,
        Err((s, b)) => return (s, Json(b)).into_response(),
    };
    // Opaque offset cursor; invalid → 400 (fail-closed, explicit).
    let cursor = match params.cursor.as_deref() {
        None | Some("") => 0,
        Some(c) => match c.parse::<usize>() {
            Ok(n) => n,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "cursor invalid"})),
                )
                    .into_response();
            }
        },
    };
    // Default 50, cap 1000 (matches export ceiling).
    let limit = params.limit.unwrap_or(50).clamp(1, 1000);
    let page = list_audit(&ListAuditQuery {
        org_id: Some(org_id),
        limit,
        cursor,
        decision,
        tool_kind,
        from,
        to,
    });
    let records: Vec<_> = page.records.iter().map(audit_record_view).collect();
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "records": records,
            "next_cursor": page.next_cursor.unwrap_or_default(),
            "total": page.total,
            "truncated": page.truncated,
        })),
    )
        .into_response()
}

/// Bounded-burst SSE tail (REST shim for SubscribeAudit).
/// Emits up to `limit` records after `cursor` as `record` events with
/// `id:` = resume cursor, then a `ready` event and closes; EventSource
/// reconnects (honoring Last-Event-ID) after `retry: 5000`. No long-lived
/// connection, no new deps — true streaming is a follow-up.
async fn audit_stream_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<ListAuditParams>,
) -> impl IntoResponse {
    let caller = match state.auth.authenticate(&headers).await {
        Ok(user) => user.sub,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "unauthorized"})),
            )
                .into_response();
        }
    };
    if let Err((s, b)) = check_rate_limit(&rate_key(&headers, "audit_stream")) {
        return (s, Json(b)).into_response();
    }
    // C2: org_id required (no all-org stream) + owner check.
    let org_id = match require_org_param(params.org_id.as_deref()) {
        Ok(o) => o,
        Err((s, b)) => return (s, Json(b)).into_response(),
    };
    if let Err((s, b)) = require_org_member(&caller, &org_id) {
        return (s, Json(b)).into_response();
    }
    // Last-Event-ID resume: explicit cursor wins, else the SSE resume header.
    let cursor_raw = params.cursor.clone().filter(|c| !c.is_empty()).or_else(|| {
        headers
            .get("last-event-id")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
    });
    let cursor = match cursor_raw.as_deref() {
        None | Some("") => 0,
        Some(c) => match c.parse::<usize>() {
            Ok(n) => n,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "cursor invalid"})),
                )
                    .into_response();
            }
        },
    };
    let from = match parse_time_bound(params.from.as_deref(), "from") {
        Ok(v) => v,
        Err((s, b)) => return (s, Json(b)).into_response(),
    };
    let to = match parse_time_bound(params.to.as_deref(), "to") {
        Ok(v) => v,
        Err((s, b)) => return (s, Json(b)).into_response(),
    };
    // Bounded burst: at most 200 events per connection.
    let limit = params.limit.unwrap_or(50).clamp(1, 200);
    let page = list_audit(&ListAuditQuery {
        org_id: Some(org_id),
        limit,
        cursor,
        decision: params.decision.as_deref().map(str::to_ascii_lowercase),
        tool_kind: params.tool_kind.as_deref().map(str::to_ascii_lowercase),
        from,
        to,
    });
    let mut body = String::from("retry: 5000\n\n");
    for (i, r) in page.records.iter().enumerate() {
        let id = cursor + i + 1;
        let data = serde_json::to_string(&audit_record_view(r))
            .unwrap_or_else(|_| r#"{"error":"encode"}"#.to_string());
        body.push_str(&format!("id: {id}\nevent: record\ndata: {data}\n\n"));
    }
    body.push_str("event: ready\ndata: {}\n\n");
    (
        StatusCode::OK,
        [
            (
                axum::http::header::CONTENT_TYPE,
                "text/event-stream; charset=utf-8",
            ),
            (axum::http::header::CACHE_CONTROL, "no-cache"),
        ],
        body,
    )
        .into_response()
}

async fn dry_run_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let caller = match authed_user(&state, &headers).await {
        Ok(c) => c,
        Err((s, b)) => return (s, b).into_response(),
    };
    if let Err((s, b)) = check_rate_limit(&rate_key(&headers, "dry_run")) {
        return (s, Json(b)).into_response();
    }
    let bundle_val = payload.get("bundle");
    if bundle_val.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"bundle required"})),
        )
            .into_response();
    }
    let history_ids = payload
        .get("history_ids")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if history_ids.len() > MAX_HISTORY_IDS {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(serde_json::json!({"error":"too many history_ids"})),
        )
            .into_response();
    }
    // Paid gate (pro+): core protection stays open, but policy analysis
    // at scale is a paid feature. Unknown orgs AND non-owners fail closed
    // with 402 (org_id alone never spends someone else's plan).
    let scope_org = payload
        .get("org_id")
        .and_then(|v| v.as_str())
        .unwrap_or("default");
    let ent = match require_feature(&state, scope_org, &caller, "dry_run").await {
        Ok(e) => e,
        Err((s, b)) => return (s, Json(b)).into_response(),
    };
    if let Err((s, b)) = check_rate_limit_org(scope_org, "dry_run", ent.rate_multiplier) {
        return (s, Json(b)).into_response();
    }
    let history_len = history_ids.len();
    // Fail-closed: if a full bundle (version+signed_bytes_b64+sig_b64) is
    // supplied, verify it before reporting. Tampered/expired/rollback → 400
    // (caller maps to ask), never "dry-run ok".
    if let Some(bundle_obj) = bundle_val.and_then(|v| v.as_object()) {
        let version = bundle_obj.get("version").and_then(|v| v.as_str());
        let signed_b64 = bundle_obj.get("signed_bytes_b64").and_then(|v| v.as_str());
        let sig_b64 = bundle_obj.get("sig_b64").and_then(|v| v.as_str());
        if let (Some(version), Some(signed_b64), Some(sig_b64)) = (version, signed_b64, sig_b64) {
            use base64::{engine::general_purpose::STANDARD as B64, Engine};
            let (signed_bytes, sig) = match (B64.decode(signed_b64), B64.decode(sig_b64)) {
                (Ok(sb), Ok(s)) => (sb, s),
                _ => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({"error":"bad bundle encoding"})),
                    )
                        .into_response();
                }
            };
            let bundle = PolicyBundle {
                version: version.to_string(),
                signed_bytes,
                sig,
            };
            // Verify against caller's org scope when provided.
            let scope = payload
                .get("org_id")
                .and_then(|v| v.as_str())
                .unwrap_or("global");
            let pubkey = policy::test_pubkey_bytes();
            if let Err(e) = verify::verify_bundle_scoped(&bundle, &pubkey, scope) {
                tracing::warn!("dry-run rejected unverifiable bundle");
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": e.to_string()})),
                )
                    .into_response();
            }
        }
    }
    // Bounded synthetic replay (real policy evaluation is core-owned per P3-02).
    let decisions: Vec<String> = (0..history_len)
        .map(|i| format!("ask: history {i}"))
        .collect();
    let body = serde_json::json!({
        "result": "dry-run ok",
        "decisions": decisions,
        "evaluated": history_len
    });
    (StatusCode::OK, Json(body)).into_response()
}

// ── WAL queue inspection ──────────────────────────────────────────
// C2: the global `/v1/wal/drain` route is REMOVED. Draining the shared WAL
// queue is cross-org by construction (any authed caller could wipe every
// org's pending records) and has no org scope to gate on. Queue progress
// is observable per-org via list/stats/stream. The in-memory WAL has no
// externally callable drain; a durable worker replaces it later.

// ── Router ──────────────────────────────────────────────────────────

fn app_router(state: AppState) -> Router {
    // Web dev server (Vite http://127.0.0.1:3007, see web/vite.config.ts proxy
    // /api -> 127.0.0.1:8080 with ^/api rewrite). Direct fetches (preview /
    // VITE_BACKEND_URL) need CORS: extra origins via ALGO_CORS_ORIGINS
    // (comma-separated scheme://host:port, no trailing slash; invalid entries
    // are skipped fail-closed). Same-origin prod needs no CORS.
    // Private MVP: loopback origins only.
    let mut origins = vec![
        HeaderValue::from_static("http://127.0.0.1:3007"),
        HeaderValue::from_static("http://localhost:3007"),
        // Dashboard SPA dev server (reference until the web port lands).
        HeaderValue::from_static("http://127.0.0.1:5173"),
        HeaderValue::from_static("http://localhost:5173"),
    ];
    for raw in std::env::var("ALGO_CORS_ORIGINS")
        .unwrap_or_default()
        .split(',')
    {
        let entry = raw.trim().trim_end_matches('/');
        if entry.is_empty() {
            continue;
        }
        match entry.parse::<HeaderValue>() {
            Ok(v) => origins.push(v),
            Err(e) => tracing::warn!("ignoring invalid ALGO_CORS_ORIGINS entry {entry:?}: {e}"),
        }
    }
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .max_age(std::time::Duration::from_secs(600));
    let legacy_mode = state.auth.is_legacy();
    let router = Router::new()
        .route("/", get(api_console_handler))
        .route("/ui", get(api_console_handler))
        .route("/health", get(health_handler))
        .route("/v1/orgs", post(create_org_handler).get(list_orgs_handler))
        .route("/v1/orgs/:id", get(get_org_handler))
        .route("/v1/policy/publish", post(publish_policy_handler))
        .route("/v1/policy/:version", get(get_policy_handler))
        .route("/v1/entitlement", get(entitlement_handler))
        .route("/v1/audit/export", get(export_handler))
        .route("/v1/audit/ingest", post(ingest_audit_handler))
        .route("/v1/audit", get(list_audit_handler))
        .route("/v1/audit/stream", get(audit_stream_handler))
        .route("/v1/stats", get(stats_handler))
        .route("/v1/policy/dry-run", post(dry_run_handler));
    let router = if legacy_mode {
        router
            .route("/v1/plans", get(plans_handler))
            .route("/v1/subscriptions", post(create_sub_handler))
            .route("/v1/subscriptions", get(get_sub_handler))
            .route("/v1/subscriptions/:id/change", post(change_sub_handler))
            .route("/v1/subscriptions/:id/cancel", post(cancel_sub_handler))
            .route("/v1/subscriptions/:id/activate", post(activate_sub_handler))
            .route("/v1/subscriptions/:id/seats", post(seats_handler))
            .route("/v1/billing/webhook", post(billing_webhook_handler))
            .route("/v1/auth/signup", post(email_signup_handler))
            .route("/v1/auth/login", post(email_login_handler))
            .route("/v1/auth/github/device", post(github_device_handler))
            .route("/v1/auth/github/poll", post(github_poll_handler))
            .route("/v1/auth/github/validate", post(github_validate_handler))
            .route("/v1/auth/google/url", post(google_url_handler))
            .route("/v1/auth/google/callback", post(google_callback_handler))
            .route("/v1/auth/google/verify", post(google_verify_handler))
    } else {
        router.route(
            "/v1/entitlements/refresh",
            post(refresh_entitlement_handler),
        )
    };
    router
        .layer(RequestBodyLimitLayer::new(1024 * 1024))
        .layer(TimeoutLayer::new(std::time::Duration::from_secs(10)))
        .layer(RateLimitLayer)
        // cors outside timeout/rate-limit so their 408/429 responses still
        // carry ACAO headers (browser would else surface opaque CORS failure).
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    policy::ensure_signing_key_at_startup()
        .map_err(|error| std::io::Error::other(format!("backend startup refused: {error}")))?;

    let http = http_client();
    let auth = AuthService::from_env(http.clone())
        .await
        .map_err(|error| std::io::Error::other(format!("backend startup refused: {error}")))?;
    let account_entitlements = if auth.is_legacy() {
        None
    } else {
        Some(Arc::new(
            entitlements::AccountEntitlementService::from_env(http.clone()).map_err(|error| {
                std::io::Error::other(format!("backend startup refused: {error}"))
            })?,
        ))
    };
    let state = AppState {
        policy_store: Arc::new(PolicyStore::new()),
        oauth: Arc::new(oauth_config::OAuthConfig::from_env()),
        http,
        auth: Arc::new(auth),
        account_entitlements,
    };
    let app = app_router(state);

    let addr = std::env::var("ALGO_BACKEND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("algo-backend listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

// ── Tests for main router (auth, ingest, policy, stats) ────────────

#[cfg(test)]
#[allow(clippy::await_holding_lock)]
// Intentional: test_sync::lock() serializes tests sharing the in-memory
// store and MUST be held across await (that is its whole purpose). Test-only.
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{HeaderValue, Request, StatusCode};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tower::ServiceExt; // for oneshot

    fn test_state() -> AppState {
        AppState {
            policy_store: Arc::new(PolicyStore::new()),
            oauth: Arc::new(oauth_config::OAuthConfig::disabled()),
            http: http_client(),
            auth: Arc::new(AuthService::legacy()),
            account_entitlements: None,
        }
    }

    fn oauth_state(cfg: oauth_config::OAuthConfig) -> AppState {
        AppState {
            policy_store: Arc::new(PolicyStore::new()),
            oauth: Arc::new(cfg),
            http: http_client(),
            auth: Arc::new(AuthService::legacy()),
            account_entitlements: None,
        }
    }

    #[derive(Clone)]
    struct AccountApiState {
        calls: Arc<AtomicUsize>,
        subjects: Arc<Mutex<Vec<String>>>,
        unavailable: Arc<AtomicUsize>,
    }

    async fn account_entitlement_api(
        State(state): State<AccountApiState>,
        Path(sub): Path<String>,
    ) -> impl IntoResponse {
        state.calls.fetch_add(1, Ordering::SeqCst);
        state
            .subjects
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(sub.clone());
        if state.unavailable.load(Ordering::SeqCst) != 0 {
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
        (
            StatusCode::OK,
            [(header::CACHE_CONTROL, "private, max-age=60")],
            Json(serde_json::json!({"entitlements":[{
                "product":"guard","plan":"team","limits":{},"status":"active",
                "validUntil":"2099-01-01T00:00:00Z","source":"default",
                "sourceId":"default:guard","owner":{"type":"user","id":sub}
            }]})),
        )
            .into_response()
    }

    async fn account_state() -> (AppState, AccountApiState) {
        let api_state = AccountApiState {
            calls: Arc::new(AtomicUsize::new(0)),
            subjects: Arc::new(Mutex::new(Vec::new())),
            unavailable: Arc::new(AtomicUsize::new(0)),
        };
        let api = Router::new()
            .route("/v1/users/:sub/entitlements", get(account_entitlement_api))
            .with_state(api_state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, api).await.unwrap() });
        let service = entitlements::AccountEntitlementService::test(
            reqwest::Url::parse(&format!("http://{address}/")).unwrap(),
            Duration::from_millis(20),
            Duration::from_secs(30),
            16,
        );
        let user = auth::AuthenticatedUser {
            sub: "account-subject".to_string(),
            email: None,
            email_verified: None,
            token_iat: Some(chrono::Utc::now().timestamp()),
            token_exp: Some(chrono::Utc::now().timestamp() + 300),
            entitlements: None,
        };
        (
            AppState {
                policy_store: Arc::new(PolicyStore::new()),
                oauth: Arc::new(oauth_config::OAuthConfig::disabled()),
                http: http_client(),
                auth: Arc::new(AuthService::test_account(user)),
                account_entitlements: Some(Arc::new(service)),
            },
            api_state,
        )
    }

    #[allow(dead_code)]
    fn auth_header(token: &str) -> HeaderMap {
        let mut m = HeaderMap::new();
        m.insert(
            axum::http::header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );
        m
    }

    /// Mint a real session JWT for tests (the legacy stub is denied by
    /// default, so tests must authenticate like production callers).
    /// Identity is the `sub`: distinct subs = distinct owners/strangers.
    fn test_token(sub: &str) -> String {
        crate::session::mint_session("test", sub, None, None)
    }

    /// Register a deterministic org for tests whose subject is already known.
    /// Route-level ownership tests use `create_test_org`; focused audit/stats
    /// tests use this helper to keep stable query ids.
    fn register_test_org(org_id: &str, owner_sub: &str) {
        lock_org_store().insert(
            org_id.to_string(),
            StoredOrg {
                org_id: org_id.to_string(),
                org_name: format!("test {org_id}"),
                owner_id: owner_sub.to_string(),
                created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            },
        );
    }

    /// Create an org via the API and return its id (owner = token identity).
    async fn create_test_org(token: &str, name: &str) -> String {
        let (status, body) = post_json(
            app_router(test_state()),
            "/v1/orgs",
            Some(token),
            serde_json::json!({"org_name": name}),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        body.get("org_id")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string()
    }

    #[tokio::test]
    async fn api_console_served_without_auth() {
        let _guard = test_sync::lock();
        for uri in ["/", "/ui"] {
            let app = app_router(test_state());
            let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
            let ctype = resp
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();
            assert!(
                ctype.contains("text/html"),
                "console must be HTML, got {ctype}"
            );
            let bytes = axum::body::to_bytes(resp.into_body(), 512 * 1024)
                .await
                .unwrap();
            let html = String::from_utf8_lossy(&bytes);
            assert!(html.contains("API Console"));
        }
    }

    #[tokio::test]
    async fn health_ok_without_auth() {
        let _guard = test_sync::lock();
        let app = app_router(test_state());
        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn unauthed_rejected_for_protected_routes() {
        let _guard = test_sync::lock();
        // Ensure audit ingest without auth is 401.
        audit::clear_audit_store();
        let app = app_router(test_state());
        let req = Request::builder()
            .uri("/v1/audit/ingest")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"redacted_event":"hello","decision":"allow","latency_ms":5}"#,
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        // Policy publish without auth → 401
        let app2 = app_router(test_state());
        let req2 = Request::builder()
            .uri("/v1/policy/publish")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"content":"allow *"}"#))
            .unwrap();
        let resp2 = app2.oneshot(req2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::UNAUTHORIZED);

        // Stats without auth → 401
        let app3 = app_router(test_state());
        let req3 = Request::builder()
            .uri("/v1/stats")
            .body(Body::empty())
            .unwrap();
        let resp3 = app3.oneshot(req3).await.unwrap();
        assert_eq!(resp3.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn auth_signup_login_open_and_session_works() {
        let _guard = test_sync::lock();
        email::clear_user_store();
        let app = app_router(test_state());
        // Signup is open (no auth) and mints a session.
        let req = Request::builder()
            .uri("/v1/auth/signup")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"email":"router@example.com","password":"password-01","name":"Router"}"#,
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let token = v.get("session_token").and_then(|t| t.as_str()).unwrap();
        assert_eq!(
            v.get("email").and_then(|e| e.as_str()),
            Some("router@example.com")
        );
        // Duplicate → 409.
        let app2 = app_router(test_state());
        let req2 = Request::builder()
            .uri("/v1/auth/signup")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"email":"router@example.com","password":"password-02"}"#,
            ))
            .unwrap();
        let resp2 = app2.oneshot(req2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::CONFLICT);
        // Login is open (no auth).
        let app3 = app_router(test_state());
        let req3 = Request::builder()
            .uri("/v1/auth/login")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"email":"router@example.com","password":"password-01"}"#,
            ))
            .unwrap();
        let resp3 = app3.oneshot(req3).await.unwrap();
        assert_eq!(resp3.status(), StatusCode::OK);
        // Wrong password → 401 with no oracle.
        let app4 = app_router(test_state());
        let req4 = Request::builder()
            .uri("/v1/auth/login")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"email":"router@example.com","password":"wrong-password"}"#,
            ))
            .unwrap();
        let resp4 = app4.oneshot(req4).await.unwrap();
        assert_eq!(resp4.status(), StatusCode::UNAUTHORIZED);
        // The minted email session authorizes protected routes (org-scoped).
        let org_id = create_test_org(token, "router-org").await;
        let app5 = app_router(test_state());
        let req5 = Request::builder()
            .uri(format!("/v1/stats?org_id={org_id}"))
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let resp5 = app5.oneshot(req5).await.unwrap();
        assert_eq!(resp5.status(), StatusCode::OK);
        // Stranger sessions cannot read another owner's org (404, no oracle).
        let stranger = test_token("test:stranger-router");
        let app6 = app_router(test_state());
        let req6 = Request::builder()
            .uri(format!("/v1/stats?org_id={org_id}"))
            .header("authorization", format!("Bearer {stranger}"))
            .body(Body::empty())
            .unwrap();
        let resp6 = app6.oneshot(req6).await.unwrap();
        assert_eq!(resp6.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn audit_ingest_drops_source_and_authed() {
        let _guard = test_sync::lock();
        audit::clear_audit_store();
        let token = test_token("test:alice");
        let org_id = create_test_org(&token, "ingest-org").await;
        let app = app_router(test_state());
        let body = serde_json::json!({
            "redacted_event":"redacted ls -la",
            "decision":"allow",
            "latency_ms": 12,
            "source":"should be dropped",
            "trace_id":"trace-1",
            "org_id": org_id,
        });
        let req = Request::builder()
            .uri("/v1/audit/ingest")
            .method("POST")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        // Verify stored record has no source and ingest succeeded.
        let recs = audit::all_records();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].trace_id, "trace-1");
    }

    #[tokio::test]
    async fn audit_ingest_rejects_secret_even_when_authed() {
        let _guard = test_sync::lock();
        audit::clear_audit_store();
        let token = test_token("test:alice");
        let org_id = create_test_org(&token, "secret-org").await;
        let app = app_router(test_state());
        let body = serde_json::json!({
            "redacted_event":"leaked ghp_12345678901234567890",
            "decision":"allow",
            "latency_ms": 5,
            "org_id": org_id,
        });
        let req = Request::builder()
            .uri("/v1/audit/ingest")
            .method("POST")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert!(audit::all_records().is_empty());
    }

    #[tokio::test]
    async fn policy_publish_and_get_with_auth() {
        let _guard = test_sync::lock();
        verify::clear_version_store();
        let state = test_state();
        let token = test_token("test:alice");
        let org_id = create_test_org(&token, "policy-org").await;
        let app = app_router(state.clone());

        // Publish
        let req = Request::builder()
            .uri("/v1/policy/publish")
            .method("POST")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(format!(
                r#"{{"org_id":"{org_id}","content":"allow echo"}}"#
            )))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let version = body.get("version").unwrap().as_str().unwrap().to_string();
        assert_eq!(version, "1");

        // Get
        let app2 = app_router(state);
        let req2 = Request::builder()
            .uri(format!("/v1/policy/{version}?org_id={org_id}"))
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let resp2 = app2.oneshot(req2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::OK);

        // Stranger cannot publish to or read another owner's org (404).
        let stranger = test_token("test:stranger-policy");
        let app3 = app_router(test_state());
        let req3 = Request::builder()
            .uri("/v1/policy/publish")
            .method("POST")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {stranger}"))
            .body(Body::from(format!(
                r#"{{"org_id":"{org_id}","content":"allow evil"}}"#
            )))
            .unwrap();
        let resp3 = app3.oneshot(req3).await.unwrap();
        assert_eq!(resp3.status(), StatusCode::NOT_FOUND);
        let app4 = app_router(test_state());
        let req4 = Request::builder()
            .uri(format!("/v1/policy/{version}?org_id={org_id}"))
            .header("authorization", format!("Bearer {stranger}"))
            .body(Body::empty())
            .unwrap();
        let resp4 = app4.oneshot(req4).await.unwrap();
        assert_eq!(resp4.status(), StatusCode::NOT_FOUND);
        // Missing org_id fails closed with 400 (no default scope).
        let app5 = app_router(test_state());
        let req5 = Request::builder()
            .uri("/v1/policy/publish")
            .method("POST")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(r#"{"content":"allow *"}"#))
            .unwrap();
        let resp5 = app5.oneshot(req5).await.unwrap();
        assert_eq!(resp5.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn stats_after_ingest() {
        let _guard = test_sync::lock();
        audit::clear_audit_store();
        let token = test_token("test:alice");
        let org_id = create_test_org(&token, "stats-org").await;
        // Ingest two records
        for (decision, latency) in [("allow", 10), ("deny", 20)] {
            let app = app_router(test_state());
            let body = serde_json::json!({
                "redacted_event": format!("event {decision}"),
                "decision": decision,
                "latency_ms": latency,
                "org_id": org_id,
            });
            let req = Request::builder()
                .uri("/v1/audit/ingest")
                .method("POST")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
        }
        // Query stats
        let app = app_router(test_state());
        let req = Request::builder()
            .uri(format!("/v1/stats?org_id={org_id}"))
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let stats: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(stats.get("total").unwrap().as_i64().unwrap(), 2);
        // Missing org_id fails closed with 400 (no cross-org aggregation).
        let app2 = app_router(test_state());
        let req2 = Request::builder()
            .uri("/v1/stats")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let resp2 = app2.oneshot(req2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn proves_ask_on_org_create_unauthed() {
        let _guard = test_sync::lock();
        // C2: org creation must require auth (fail-closed → 401/ask).
        let app = app_router(test_state());
        let req = Request::builder()
            .uri("/v1/orgs")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"org_name":"acme"}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn proves_ask_on_loose_token_prefix() {
        let _guard = test_sync::lock();
        audit::clear_audit_store();
        // `valid-` without `token-` must not authenticate.
        let app = app_router(test_state());
        let req = Request::builder()
            .uri("/v1/audit/ingest")
            .method("POST")
            .header("content-type", "application/json")
            .header("authorization", "Bearer valid-xyz")
            .body(Body::from(
                r#"{"redacted_event":"hello","decision":"allow","latency_ms":5}"#,
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        assert!(audit::all_records().is_empty());
    }

    #[tokio::test]
    async fn proves_ask_on_rate_limited() {
        let _guard = test_sync::lock();
        // Drive the in-handler limiter over budget with a unique key.
        let key = format!("test-rl-{}", uuid::Uuid::new_v4());
        let mut limited = false;
        for _ in 0..(RATE_LIMIT_PER_MINUTE + 5) {
            if check_rate_limit(&key).is_err() {
                limited = true;
                break;
            }
        }
        assert!(
            limited,
            "rate limiter must eventually return 429 (fail-closed)"
        );
    }

    #[tokio::test]
    async fn proves_ask_on_empty_org_stats() {
        let _guard = test_sync::lock();
        let token = test_token("test:alice");
        let app = app_router(test_state());
        let req = Request::builder()
            .uri("/v1/stats?org_id=")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        // Empty org filter must not silently leak all-org aggregates.
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn proves_ask_on_tampered_dry_run() {
        let _guard = test_sync::lock();
        verify::clear_version_store();
        subscriptions::clear_subscriptions();
        // dry-run is a pro+ feature: provision the org first (owner = caller).
        let token = test_token("test:dry-run");
        subscriptions::create_subscription("org-dry", "pro", "monthly", 1, "test:dry-run").unwrap();
        // Publish a good bundle first to get valid base64 fields.
        let state = test_state();
        let bundle = state.policy_store.publish("org-dry", "allow echo");
        use base64::{engine::general_purpose::STANDARD as B64, Engine};
        let mut signed = bundle.signed_bytes.clone();
        if !signed.is_empty() {
            signed[0] ^= 0xFF;
        }
        let body = serde_json::json!({
            "org_id": "org-dry",
            "bundle": {
                "version": bundle.version,
                "signed_bytes_b64": B64.encode(&signed),
                "sig_b64": B64.encode(&bundle.sig),
            },
            "history_ids": ["h1", "h2"]
        });
        let app = app_router(test_state());
        let req = Request::builder()
            .uri("/v1/policy/dry-run")
            .method("POST")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn oauth_unconfigured_fails_closed() {
        let _guard = test_sync::lock();
        // Unconfigured provider routes must fail closed (503 not-configured or
        // 502 provider-unreachable) — never 200 with a forged session.
        // Note: github/validate needs no client_id, so it fails at transport (502).
        for (uri, body, ok) in [
            (
                "/v1/auth/github/device",
                "{}",
                vec![StatusCode::SERVICE_UNAVAILABLE],
            ),
            (
                "/v1/auth/github/poll",
                r#"{"device_code":"x"}"#,
                vec![StatusCode::SERVICE_UNAVAILABLE],
            ),
            (
                "/v1/auth/github/validate",
                r#"{"access_token":"x"}"#,
                vec![StatusCode::SERVICE_UNAVAILABLE, StatusCode::BAD_GATEWAY],
            ),
            (
                "/v1/auth/google/url",
                "{}",
                vec![StatusCode::SERVICE_UNAVAILABLE],
            ),
            (
                "/v1/auth/google/callback",
                r#"{"code":"x","state":"y"}"#,
                vec![StatusCode::SERVICE_UNAVAILABLE, StatusCode::BAD_REQUEST],
            ),
            (
                "/v1/auth/google/verify",
                r#"{"id_token":"x","expected_nonce":"n"}"#,
                vec![StatusCode::SERVICE_UNAVAILABLE],
            ),
        ] {
            let app = app_router(test_state());
            let req = Request::builder()
                .uri(uri)
                .method("POST")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            assert!(
                ok.contains(&resp.status()),
                "{uri} must fail closed, got {}",
                resp.status()
            );
        }
    }

    async fn mock_github_user_server() -> String {
        use axum::routing::get as axum_get;
        let app = Router::new().route(
            "/user",
            axum_get(|headers: HeaderMap| async move {
                let ok = headers.get("authorization").and_then(|v| v.to_str().ok())
                    == Some("Bearer ghu_mock");
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
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn github_validate_mints_session_usable_on_protected_routes() {
        let _guard = test_sync::lock();
        audit::clear_audit_store();
        let api = mock_github_user_server().await;
        let cfg = oauth_config::OAuthConfig {
            github_client_id: Some("mock-id".to_string()),
            github_api_base: api,
            ..oauth_config::OAuthConfig::disabled()
        };
        let state = oauth_state(cfg);
        // Validate GitHub token → session JWT.
        let app = app_router(state.clone());
        let req = Request::builder()
            .uri("/v1/auth/github/validate")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"access_token":"ghu_mock"}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.get("valid").unwrap(), true);
        let session = body
            .get("session_token")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(session.split('.').count(), 3);
        register_test_org("org-github", "github:1");
        // Session JWT unlocks a protected route (identity = github:1).
        let app2 = app_router(state);
        let req2 = Request::builder()
            .uri("/v1/audit/ingest")
            .method("POST")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {session}"))
            .body(Body::from(
                r#"{"redacted_event":"hello","decision":"allow","latency_ms":5,"org_id":"org-github"}"#,
            ))
            .unwrap();
        let resp2 = app2.oneshot(req2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::OK);
        // Bogus GitHub token → 401 against the same mock provider.
        let api2 = mock_github_user_server().await;
        let cfg2 = oauth_config::OAuthConfig {
            github_client_id: Some("mock-id".to_string()),
            github_api_base: api2,
            ..oauth_config::OAuthConfig::disabled()
        };
        let app3 = app_router(oauth_state(cfg2));
        let req3 = Request::builder()
            .uri("/v1/auth/github/validate")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"access_token":"ghu_bogus"}"#))
            .unwrap();
        let resp3 = app3.oneshot(req3).await.unwrap();
        assert_eq!(resp3.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn proves_ask_on_expired_session_token() {
        let _guard = test_sync::lock();
        let expired = session::mint_session_with_ttl("github", "github:1", None, None, -5);
        let app = app_router(test_state());
        let req = Request::builder()
            .uri("/v1/stats")
            .header("authorization", format!("Bearer {expired}"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        // Expired credential → not 200 (fail-closed; caller maps to ask).
        assert_ne!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn google_url_issues_state_and_callback_rejects_unknown_state() {
        let _guard = test_sync::lock();
        google::clear_flows();
        let cfg = oauth_config::OAuthConfig {
            google_client_id: Some("mock-google-client-id".to_string()),
            ..oauth_config::OAuthConfig::disabled()
        };
        let state = oauth_state(cfg);
        let app = app_router(state.clone());
        let req = Request::builder()
            .uri("/v1/auth/google/url")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"redirect_uri":"http://127.0.0.1:51004/oauth2redirect"}"#,
            ))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(body
            .get("auth_url")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("code_challenge_method=S256"));
        assert!(!body.get("state").unwrap().as_str().unwrap().is_empty());
        // Unknown state on callback → 400 (no code exchange attempted).
        let app2 = app_router(state.clone());
        let req2 = Request::builder()
            .uri("/v1/auth/google/callback")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"code":"x","state":"unknown-state"}"#))
            .unwrap();
        let resp2 = app2.oneshot(req2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::BAD_REQUEST);
        // Attacker-supplied redirect_uri mismatch → 400 (pinned URI only).
        let app3 = app_router(state.clone());
        let req3 = Request::builder()
            .uri("/v1/auth/google/url")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"redirect_uri":"https://evil.example/cb"}"#))
            .unwrap();
        let resp3 = app3.oneshot(req3).await.unwrap();
        assert_eq!(resp3.status(), StatusCode::BAD_REQUEST);
        // Verify without expected_nonce → 400 (replay binding required).
        let app4 = app_router(state);
        let req4 = Request::builder()
            .uri("/v1/auth/google/verify")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"id_token":"x"}"#))
            .unwrap();
        let resp4 = app4.oneshot(req4).await.unwrap();
        assert_eq!(resp4.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn proves_ask_on_oversize_dry_run() {
        let _guard = test_sync::lock();
        let token = test_token("test:oversize");
        let ids: Vec<String> = (0..(MAX_HISTORY_IDS + 1))
            .map(|i| format!("h{i}"))
            .collect();
        let body = serde_json::json!({"bundle": {"version":"1"}, "history_ids": ids});
        let app = app_router(test_state());
        let req = Request::builder()
            .uri("/v1/policy/dry-run")
            .method("POST")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    // ── Billing handler tests ────────────────────────────────────────

    #[tokio::test]
    async fn account_mode_entitlement_and_refresh_are_caller_scoped() {
        let _guard = test_sync::lock();
        let (state, api) = account_state().await;
        let (status, _) = get_json(app_router(state.clone()), "/v1/entitlement", None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        let (status, body) = get_json(
            app_router(state.clone()),
            "/v1/entitlement?org_id=attacker-controlled",
            Some("test-account"),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["plan"], "team");

        let (status, body) = post_json(
            app_router(state.clone()),
            "/v1/entitlements/refresh?sub=victim",
            Some("test-account"),
            serde_json::json!({"sub":"victim"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["plan"], "team");
        assert!(api
            .subjects
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .all(|subject| subject == "account-subject"));

        let (status, _) = post_json(
            app_router(state),
            "/v1/entitlements/refresh",
            Some("test-account"),
            serde_json::json!({}),
        )
        .await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn account_mode_does_not_expose_local_billing_or_webhook_routes() {
        let _guard = test_sync::lock();
        let (state, _) = account_state().await;
        let (status, _) = get_json(app_router(state.clone()), "/v1/plans", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = post_json(
            app_router(state.clone()),
            "/v1/subscriptions",
            Some("test-account"),
            serde_json::json!({}),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = post_json(
            app_router(state),
            "/v1/billing/webhook",
            None,
            serde_json::json!({}),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn proves_ask_on_account_entitlement_network_failure() {
        let _guard = test_sync::lock();
        let (state, api) = account_state().await;
        register_test_org("owned", "account-subject");
        api.unavailable.store(1, Ordering::SeqCst);
        let (status, body) = post_json(
            app_router(state),
            "/v1/policy/dry-run",
            Some("test-account"),
            serde_json::json!({"org_id":"owned","bundle":{"version":"1"},"history_ids":[]}),
        )
        .await;
        assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
        assert_eq!(body["upgrade"]["feature"], "dry_run");
    }

    fn billing_token() -> String {
        test_token("test:billing")
    }

    async fn post_json(
        app: Router,
        uri: &str,
        token: Option<&str>,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        let mut builder = Request::builder()
            .uri(uri)
            .method("POST")
            .header("content-type", "application/json");
        if let Some(t) = token {
            builder = builder.header("authorization", format!("Bearer {t}"));
        }
        let req = builder
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    async fn get_json(
        app: Router,
        uri: &str,
        token: Option<&str>,
    ) -> (StatusCode, serde_json::Value) {
        let mut builder = Request::builder().uri(uri);
        if let Some(t) = token {
            builder = builder.header("authorization", format!("Bearer {t}"));
        }
        let req = builder.body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    #[tokio::test]
    async fn plans_open_and_billing_requires_auth() {
        let _guard = test_sync::lock();
        subscriptions::clear_subscriptions();
        // Catalog is public.
        let (status, body) = get_json(app_router(test_state()), "/v1/plans", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.get("plans").unwrap().as_array().unwrap().len(), 3);
        // Everything else needs auth.
        for (method_uri, body) in [
            (
                "/v1/subscriptions",
                serde_json::json!({"org_id":"o","tier":"pro","cycle":"monthly","seats":1}),
            ),
            ("/v1/entitlement?org_id=o", serde_json::json!({})),
        ] {
            let (status, _) = if method_uri.starts_with("/v1/entitlement") {
                get_json(app_router(test_state()), method_uri, None).await
            } else {
                post_json(app_router(test_state()), method_uri, None, body).await
            };
            assert_eq!(status, StatusCode::UNAUTHORIZED, "{method_uri}");
        }
    }

    #[tokio::test]
    async fn subscription_lifecycle_via_api() {
        let _guard = test_sync::lock();
        subscriptions::clear_subscriptions();
        let token = billing_token();
        // Create (trialing).
        let (status, body) = post_json(
            app_router(test_state()),
            "/v1/subscriptions",
            Some(&token),
            serde_json::json!({"org_id":"org-bill","tier":"pro","cycle":"monthly","seats":2}),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body.get("status").unwrap(), "trialing");
        assert_eq!(body.get("price_cents").unwrap(), 2400);
        let id = body.get("id").unwrap().as_str().unwrap().to_string();
        // Bad tier → 400.
        let (status, _) = post_json(
            app_router(test_state()),
            "/v1/subscriptions",
            Some(&token),
            serde_json::json!({"org_id":"org-bad","tier":"ultra","cycle":"monthly","seats":1}),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        // Get.
        let (status, body) = get_json(
            app_router(test_state()),
            "/v1/subscriptions?org_id=org-bill",
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.get("id").unwrap().as_str().unwrap(), id);
        // Unknown org → free shape.
        let (status, body) = get_json(
            app_router(test_state()),
            "/v1/subscriptions?org_id=ghost",
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.get("tier").unwrap(), "free");
        // Upgrade immediate.
        let (status, body) = post_json(
            app_router(test_state()),
            &format!("/v1/subscriptions/{id}/change"),
            Some(&token),
            serde_json::json!({"tier":"max"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.get("scheduled").unwrap(), false);
        assert_eq!(body["subscription"].get("tier").unwrap(), "max");
        // Downgrade scheduled.
        let (status, body) = post_json(
            app_router(test_state()),
            &format!("/v1/subscriptions/{id}/change"),
            Some(&token),
            serde_json::json!({"tier":"pro"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.get("scheduled").unwrap(), true);
        // Over-cap seats → 409.
        let (status, _) = post_json(
            app_router(test_state()),
            &format!("/v1/subscriptions/{id}/seats"),
            Some(&token),
            serde_json::json!({"seats": 101}),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        // Cancel at period end keeps status.
        let (status, body) = post_json(
            app_router(test_state()),
            &format!("/v1/subscriptions/{id}/cancel"),
            Some(&token),
            serde_json::json!({"at_period_end": true}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.get("cancel_at_period_end").unwrap(), true);
        // Activate works.
        let (status, body) = post_json(
            app_router(test_state()),
            &format!("/v1/subscriptions/{id}/activate"),
            Some(&token),
            serde_json::json!({}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.get("status").unwrap(), "active");
    }

    #[tokio::test]
    async fn entitlement_gates_dry_run_and_export() {
        let _guard = test_sync::lock();
        subscriptions::clear_subscriptions();
        audit::clear_audit_store();
        let token = billing_token();
        // Free org: entitlement invalid with hint.
        let (status, body) = get_json(
            app_router(test_state()),
            "/v1/entitlement?org_id=org-free&feature=dry_run",
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.get("valid").unwrap(), false);
        assert!(body
            .get("upgrade_hint")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("pro"));
        // Free org dry-run → 402 with upgrade payload.
        let (status, body) = post_json(
            app_router(test_state()),
            "/v1/policy/dry-run",
            Some(&token),
            serde_json::json!({"org_id":"org-free","bundle":{"version":"1"},"history_ids":[]}),
        )
        .await;
        assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
        assert_eq!(body["upgrade"].get("tier").unwrap(), "pro");
        // Pro org dry-run → evaluates (bundle missing sig → 400 proves logic ran).
        subscriptions::create_subscription("org-pro", "pro", "monthly", 1, "test:billing").unwrap();
        let (status, _) = post_json(
            app_router(test_state()),
            "/v1/policy/dry-run",
            Some(&token),
            serde_json::json!({"org_id":"org-pro","bundle":{"version":"bad!!","signed_bytes_b64":"eA==","sig_b64":"eA=="},"history_ids":[]}),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        // Pro org export → 402 (team feature).
        let (status, body) = get_json(
            app_router(test_state()),
            "/v1/audit/export?org_id=org-pro",
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
        assert_eq!(body["upgrade"].get("tier").unwrap(), "team");
        // Team org export → 200 with redacted records only.
        subscriptions::create_subscription("org-team", "team", "monthly", 2, "test:billing")
            .unwrap();
        register_test_org("org-team", "test:billing");
        audit::clear_audit_store();
        let (status, _) = post_json(
            app_router(test_state()),
            "/v1/audit/ingest",
            Some(&token),
            serde_json::json!({"redacted_event":"export me","decision":"allow","latency_ms":3,"org_id":"org-team"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, body) = get_json(
            app_router(test_state()),
            "/v1/audit/export?org_id=org-team",
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.get("total").unwrap(), 1);
        assert_eq!(
            body["records"][0].get("redacted_event").unwrap(),
            "export me"
        );
        // Exported records never carry source fields.
        assert!(body["records"][0].get("source").is_none());
    }

    #[tokio::test]
    async fn proves_no_cross_owner_spend_via_api() {
        let _guard = test_sync::lock();
        subscriptions::clear_subscriptions();
        let victim = test_token("test:victim");
        let attacker = test_token("test:attacker");
        // Victim owns a TEAM subscription on org-victim.
        let (status, body) = post_json(
            app_router(test_state()),
            "/v1/subscriptions",
            Some(&victim),
            serde_json::json!({"org_id":"org-victim","tier":"team","cycle":"monthly","seats":2}),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let id = body.get("id").unwrap().as_str().unwrap().to_string();
        // Attacker reads victim org → free shape (no tier/status leak).
        let (status, body) = get_json(
            app_router(test_state()),
            "/v1/entitlement?org_id=org-victim",
            Some(&attacker),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.get("tier").unwrap(), "free");
        assert_eq!(body.get("valid").unwrap(), false);
        // Attacker spends victim org on dry-run → 402 (not 200).
        let (status, body) = post_json(
            app_router(test_state()),
            "/v1/policy/dry-run",
            Some(&attacker),
            serde_json::json!({"org_id":"org-victim","bundle":{"version":"1"},"history_ids":[]}),
        )
        .await;
        assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
        assert_eq!(body["upgrade"].get("tier").unwrap(), "pro");
        // Attacker spends victim org on export → 402.
        let (status, _) = get_json(
            app_router(test_state()),
            "/v1/audit/export?org_id=org-victim",
            Some(&attacker),
        )
        .await;
        assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
        // Attacker mutates victim sub → 404 (indistinguishable from missing).
        for (uri, body) in [
            (
                format!("/v1/subscriptions/{id}/change"),
                serde_json::json!({"tier": "pro"}),
            ),
            (
                format!("/v1/subscriptions/{id}/cancel"),
                serde_json::json!({"at_period_end": false}),
            ),
            (
                format!("/v1/subscriptions/{id}/seats"),
                serde_json::json!({"seats": 1}),
            ),
        ] {
            let (status, _) =
                post_json(app_router(test_state()), &uri, Some(&attacker), body).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        }
        let (status, _) = post_json(
            app_router(test_state()),
            &format!("/v1/subscriptions/{id}/activate"),
            Some(&attacker),
            serde_json::json!({}),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        // Attacker cannot even re-subscribe over the live org (409 either way,
        // and it reveals nothing about who owns it).
        let (status, _) = post_json(
            app_router(test_state()),
            "/v1/subscriptions",
            Some(&attacker),
            serde_json::json!({"org_id":"org-victim","tier":"pro","cycle":"monthly","seats":1}),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        // Victim still fully entitled.
        let (status, body) = get_json(
            app_router(test_state()),
            "/v1/entitlement?org_id=org-victim&feature=siem_export",
            Some(&victim),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.get("valid").unwrap(), true);
    }

    #[tokio::test]
    async fn billing_webhook_reserved_honest_501() {
        let _guard = test_sync::lock();
        let (status, body) = post_json(
            app_router(test_state()),
            "/v1/billing/webhook",
            None,
            serde_json::json!({"type":"anything"}),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
        assert!(body
            .get("error")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("deferred MoR"));
    }

    // ── Dashboard shims (Phase 2 v1) ─────────────────────────────

    async fn ingest_dashboard_record(
        token: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        post_json(
            app_router(test_state()),
            "/v1/audit/ingest",
            Some(token),
            body,
        )
        .await
    }

    #[tokio::test]
    async fn audit_list_requires_auth_and_valid_org() {
        let _guard = test_sync::lock();
        audit::clear_audit_store();
        let (status, _) = get_json(app_router(test_state()), "/v1/audit", None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        let token = test_token("test:audit-list-validation");
        let (status, _) =
            get_json(app_router(test_state()), "/v1/audit?org_id=", Some(&token)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let (status, _) = get_json(app_router(test_state()), "/v1/audit/stream", None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn audit_list_roundtrip_newest_first_with_pagination() {
        let _guard = test_sync::lock();
        audit::clear_audit_store();
        let token = test_token("test:audit-roundtrip");
        register_test_org("org-dash", "test:audit-roundtrip");
        for (decision, kind) in [("allow", "shell"), ("deny", "edit"), ("ask", "net")] {
            let (status, _) = ingest_dashboard_record(
                &token,
                serde_json::json!({
                    "redacted_event": format!("redacted {decision} {kind}"),
                    "decision": decision,
                    "latency_ms": 7,
                    "org_id": "org-dash",
                    "tool_kind": kind,
                    "user_id": "u-1",
                    "reason": format!("{decision} by rule"),
                    "confidence_0_1": 0.9,
                    "source_level": "rule",
                    "policy_version": "3",
                }),
            )
            .await;
            assert_eq!(status, StatusCode::OK);
        }
        // Newest first: last ingested (ask) leads page 1 with limit=2.
        let (status, body) = get_json(
            app_router(test_state()),
            "/v1/audit?org_id=org-dash&limit=2",
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.get("total").unwrap(), 3);
        assert_eq!(body.get("truncated").unwrap(), true);
        let records = body.get("records").unwrap().as_array().unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(
            records[0].get("decision").unwrap().get("action").unwrap(),
            3 // ask
        );
        // Proto-shaped view fields.
        assert_eq!(records[0].get("tool_kind").unwrap(), 5); // net
        assert_eq!(
            records[0]
                .get("decision")
                .unwrap()
                .get("source_level")
                .unwrap(),
            1 // rule
        );
        assert_eq!(
            records[0]
                .get("decision")
                .unwrap()
                .get("confidence_0_1")
                .unwrap()
                .as_f64()
                .unwrap(),
            0.9
        );
        assert_eq!(records[0].get("privacy_mode").unwrap(), 2); // redacted
        let cursor = body
            .get("next_cursor")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        assert!(!cursor.is_empty());
        // Follow the cursor to the tail.
        let (status, body2) = get_json(
            app_router(test_state()),
            &format!("/v1/audit?org_id=org-dash&limit=2&cursor={cursor}"),
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let records2 = body2.get("records").unwrap().as_array().unwrap();
        assert_eq!(records2.len(), 1);
        assert_eq!(body2.get("truncated").unwrap(), false);
        assert_eq!(body2.get("next_cursor").unwrap().as_str().unwrap(), "");
    }

    #[tokio::test]
    async fn audit_list_filters_and_fail_closed_params() {
        let _guard = test_sync::lock();
        audit::clear_audit_store();
        let token = test_token("test:audit-filters");
        register_test_org("org-a", "test:audit-filters");
        register_test_org("org-b", "test:audit-filters");
        for (decision, org) in [("allow", "org-a"), ("deny", "org-a"), ("allow", "org-b")] {
            let (status, _) = ingest_dashboard_record(
                &token,
                serde_json::json!({
                    "redacted_event": format!("redacted {decision}"),
                    "decision": decision,
                    "latency_ms": 5,
                    "org_id": org,
                }),
            )
            .await;
            assert_eq!(status, StatusCode::OK);
        }
        // Decision filter.
        let (status, body) = get_json(
            app_router(test_state()),
            "/v1/audit?org_id=org-a&decision=deny",
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.get("total").unwrap(), 1);
        // Org isolation: org-b sees only its own.
        let (status, body) = get_json(
            app_router(test_state()),
            "/v1/audit?org_id=org-b",
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.get("total").unwrap(), 1);
        // Unknown decision matches nothing (200 empty per contract, not 400).
        let (status, body) = get_json(
            app_router(test_state()),
            "/v1/audit?org_id=org-a&decision=bogus",
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.get("total").unwrap(), 0);
        // Invalid cursor / from fail closed with 400.
        let (status, _) = get_json(
            app_router(test_state()),
            "/v1/audit?org_id=org-a&cursor=nope",
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let (status, _) = get_json(
            app_router(test_state()),
            "/v1/audit?org_id=org-a&from=not-a-time",
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        // Valid from/to window round-trips (wide bounds keep everything).
        let (status, body) = get_json(
            app_router(test_state()),
            "/v1/audit?org_id=org-a&from=2000-01-01T00:00:00Z&to=2100-01-01T00:00:00Z",
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.get("total").unwrap(), 2);
    }

    #[tokio::test]
    async fn ingest_rejects_invalid_dashboard_fields() {
        let _guard = test_sync::lock();
        audit::clear_audit_store();
        let token = test_token("test:invalid-fields");
        let base = serde_json::json!({
            "redacted_event": "redacted ls",
            "decision": "allow",
            "latency_ms": 5,
        });
        // Bad tool_kind / source / confidence / reason all 400, nothing stored.
        for patch in [
            serde_json::json!({"tool_kind": "teleport"}),
            serde_json::json!({"source_level": "oracle"}),
            serde_json::json!({"confidence_0_1": 1.5}),
            serde_json::json!({"confidence_0_1": "high"}),
            serde_json::json!({"reason": "has\nnewline"}),
        ] {
            let mut body = base.clone();
            for (k, v) in patch.as_object().unwrap() {
                body[k] = v.clone();
            }
            let (status, _) = ingest_dashboard_record(&token, body).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{patch}");
        }
        assert!(audit::all_records().is_empty());
    }

    #[tokio::test]
    async fn stats_series_buckets_and_per_user() {
        let _guard = test_sync::lock();
        audit::clear_audit_store();
        let token = test_token("test:stats-series");
        register_test_org("org-series", "test:stats-series");
        for (decision, user) in [("allow", "u-1"), ("deny", "u-1"), ("ask", "u-2")] {
            let (status, _) = ingest_dashboard_record(
                &token,
                serde_json::json!({
                    "redacted_event": format!("redacted {decision}"),
                    "decision": decision,
                    "latency_ms": 10,
                    "org_id": "org-series",
                    "user_id": user,
                }),
            )
            .await;
            assert_eq!(status, StatusCode::OK);
        }
        let (status, body) = get_json(
            app_router(test_state()),
            "/v1/stats?org_id=org-series&granularity=day",
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.get("total").unwrap(), 3);
        assert_eq!(body.get("allow").unwrap(), 1);
        let buckets = body.get("buckets").unwrap().as_array().unwrap();
        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].get("total").unwrap(), 3);
        assert!(buckets[0].get("bucket_start").unwrap().is_string());
        let per_user = body.get("per_user").unwrap().as_array().unwrap();
        assert_eq!(per_user.len(), 2);
        assert_eq!(per_user[0].get("user_id").unwrap(), "u-1");
        assert_eq!(per_user[0].get("total").unwrap(), 2);
        // Unknown granularity falls back to day (200, not 400).
        let (status, _) = get_json(
            app_router(test_state()),
            "/v1/stats?org_id=org-series&granularity=fortnight",
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        // Invalid from is 400.
        let (status, _) = get_json(
            app_router(test_state()),
            "/v1/stats?org_id=org-series&from=soon",
            Some(&token),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn audit_stream_burst_shape() {
        let _guard = test_sync::lock();
        audit::clear_audit_store();
        let token = test_token("test:audit-stream");
        register_test_org("org-stream", "test:audit-stream");
        let (status, _) = ingest_dashboard_record(
            &token,
            serde_json::json!({
                "redacted_event": "redacted whoami",
                "decision": "allow",
                "latency_ms": 3,
                "org_id": "org-stream",
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let req = Request::builder()
            .uri("/v1/audit/stream?org_id=org-stream")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let resp = app_router(test_state()).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let ctype = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        assert!(ctype.contains("text/event-stream"), "got {ctype}");
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("retry: 5000"), "{text}");
        assert!(text.contains("event: record"), "{text}");
        assert!(text.contains("event: ready"), "{text}");
        assert!(text.contains("redacted whoami"), "{text}");
    }

    #[tokio::test]
    async fn org_reads_are_owner_scoped_no_oracle() {
        let _guard = test_sync::lock();
        clear_org_store();
        let owner = test_token("test:owner1");
        let stranger = test_token("test:stranger1");
        let (status, body) = post_json(
            app_router(test_state()),
            "/v1/orgs",
            Some(&owner),
            serde_json::json!({"org_name": "dash-org"}),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let org_id = body.get("org_id").unwrap().as_str().unwrap().to_string();
        // Owner reads fine.
        let (status, body) = get_json(
            app_router(test_state()),
            &format!("/v1/orgs/{org_id}"),
            Some(&owner),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body.get("org").unwrap().get("org_name").unwrap(),
            "dash-org"
        );
        // Stranger gets the same 404 as an unknown id (no oracle).
        let (status, _) = get_json(
            app_router(test_state()),
            &format!("/v1/orgs/{org_id}"),
            Some(&stranger),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) =
            get_json(app_router(test_state()), "/v1/orgs/org_nope", Some(&owner)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        // Owner list contains it; stranger list is empty.
        let (status, body) = get_json(app_router(test_state()), "/v1/orgs", Some(&owner)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.get("total").unwrap(), 1);
        let (status, body) = get_json(app_router(test_state()), "/v1/orgs", Some(&stranger)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.get("total").unwrap(), 0);
        // Unauthed reads rejected.
        let (status, _) = get_json(
            app_router(test_state()),
            &format!("/v1/orgs/{org_id}"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}
