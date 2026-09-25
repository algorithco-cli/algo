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

mod audit;
mod auth;
mod email;
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

use audit::{ingest_audit, AuditError};
use auth::require_auth;
use policy::PolicyStore;
use stats::query_stats;
use verify::PolicyBundle;

// ── State & rate limit ──────────────────────────────────────────────

#[derive(Clone)]
struct AppState {
    policy_store: Arc<PolicyStore>,
    oauth: Arc<oauth_config::OAuthConfig>,
    http: reqwest::Client,
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

// WAL-like queue is audit::WAL_QUEUE (VecDeque) — append on ingest, drain via /v1/wal/drain.

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

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct StatsQueryParams {
    org_id: Option<String>,
    from: Option<String>,
    to: Option<String>,
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
    headers: HeaderMap,
    Json(payload): Json<CreateOrgPayload>,
) -> impl IntoResponse {
    // P3-03: org creation requires auth; owner is derived from caller, never
    // trusted from the client payload.
    let caller = match require_auth(&headers) {
        Ok(c) => c,
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
        org_name: name,
        owner_id: caller,
    };
    // In-memory: we don't persist org beyond response for MVP; stats/policy keyed by org_id still works.
    (StatusCode::CREATED, Json(resp)).into_response()
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
fn require_feature(
    org_id: &str,
    caller: &str,
    feature: &str,
) -> Result<subscriptions::Entitlement, (StatusCode, serde_json::Value)> {
    if org_id.trim().is_empty() || org_id.len() > MAX_ORG_ID_LEN {
        return Err((
            StatusCode::PAYMENT_REQUIRED,
            serde_json::json!({"error": "org_id required", "upgrade": {"tier": "pro", "feature": feature}}),
        ));
    }
    let ent = subscriptions::resolve_entitlement(org_id, caller, Some(feature));
    if ent.valid && ent.features.get(feature) == Some(true) {
        return Ok(ent);
    }
    let tier = plans::FeatureSet::min_tier_for(feature).unwrap_or(plans::TIER_PRO);
    Err((
        StatusCode::PAYMENT_REQUIRED,
        serde_json::json!({"error": ent.upgrade_hint.clone().unwrap_or_else(|| "subscription required".to_string()), "upgrade": {"tier": tier, "feature": feature}}),
    ))
}

/// Authenticate and return the caller identity (token or session sub).
/// Every billing handler binds records to this identity (owner).
fn authed_caller(headers: &HeaderMap) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    require_auth(headers).map_err(|_| {
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
    headers: HeaderMap,
    Json(payload): Json<CreateSubPayload>,
) -> impl IntoResponse {
    let caller = match authed_caller(&headers) {
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

async fn get_sub_handler(headers: HeaderMap, Query(params): Query<SubQuery>) -> impl IntoResponse {
    let caller = match authed_caller(&headers) {
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
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<ChangeSubPayload>,
) -> impl IntoResponse {
    let caller = match authed_caller(&headers) {
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
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<CancelSubPayload>,
) -> impl IntoResponse {
    let caller = match authed_caller(&headers) {
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

async fn activate_sub_handler(headers: HeaderMap, Path(id): Path<String>) -> impl IntoResponse {
    let caller = match authed_caller(&headers) {
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
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<SeatsPayload>,
) -> impl IntoResponse {
    let caller = match authed_caller(&headers) {
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
    headers: HeaderMap,
    Query(params): Query<EntitlementQuery>,
) -> impl IntoResponse {
    let caller = match authed_caller(&headers) {
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
    if org_id.len() > MAX_ORG_ID_LEN {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "org_id invalid"})),
        )
            .into_response();
    }
    // Owner-gated inside resolve: strangers see the free shape.
    let ent = subscriptions::resolve_entitlement(&org_id, &caller, params.feature.as_deref());
    (StatusCode::OK, Json(ent)).into_response()
}

async fn billing_webhook_handler() -> impl IntoResponse {
    // Reserved for the ADR-0006 MoR vendor. No fake processing: an explicit
    // 501 beats silently dropping provider events (fail-closed, honest).
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(
            serde_json::json!({"error": "billing provider not configured (ADR-0006 MoR deferred)"}),
        ),
    )
        .into_response()
}

async fn export_handler(
    headers: HeaderMap,
    Query(params): Query<ExportQuery>,
) -> impl IntoResponse {
    let caller = match authed_caller(&headers) {
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
    let ent = match require_feature(&org_id, &caller, "siem_export") {
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
    if require_auth(&headers).is_err() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "unauthorized"})),
        )
            .into_response();
    }
    if let Err((s, b)) = check_rate_limit(&rate_key(&headers, "policy_publish")) {
        return (s, Json(b)).into_response();
    }
    let org_id = payload.org_id.unwrap_or_else(|| "default".to_string());
    if org_id.len() > MAX_ORG_ID_LEN || org_id.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"org_id invalid"})),
        )
            .into_response();
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
    if require_auth(&headers).is_err() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "unauthorized"})),
        )
            .into_response();
    }
    if version.len() > MAX_VERSION_LEN {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"version too long"})),
        )
            .into_response();
    }
    let org_id = params
        .get("org_id")
        .map(|s| s.as_str())
        .unwrap_or("default");
    if org_id.len() > MAX_ORG_ID_LEN {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"org_id invalid"})),
        )
            .into_response();
    }
    match state.policy_store.get(org_id, &version) {
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
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    if require_auth(&headers).is_err() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "unauthorized"})),
        )
            .into_response();
    }
    if let Err((s, b)) = check_rate_limit(&rate_key(&headers, "audit_ingest")) {
        return (s, Json(b)).into_response();
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

async fn stats_handler(
    headers: HeaderMap,
    Query(params): Query<StatsQueryParams>,
) -> impl IntoResponse {
    if require_auth(&headers).is_err() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "unauthorized"})),
        )
            .into_response();
    }
    // Empty org_id fails closed with 400 (no silent all-org leak).
    if let Some(org) = params.org_id.as_deref() {
        if org.is_empty() || org.len() > MAX_ORG_ID_LEN {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error":"org_id invalid"})),
            )
                .into_response();
        }
    }
    let s = query_stats(params.org_id.as_deref());
    (StatusCode::OK, Json(s)).into_response()
}

async fn dry_run_handler(
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let caller = match authed_caller(&headers) {
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
    let ent = match require_feature(scope_org, &caller, "dry_run") {
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

// ── WAL queue inspection (health/diagnostics) ───────────────────────

async fn wal_drain_handler(headers: HeaderMap) -> impl IntoResponse {
    if require_auth(&headers).is_err() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "unauthorized"})),
        )
            .into_response();
    }
    if let Err((s, b)) = check_rate_limit(&rate_key(&headers, "wal_drain")) {
        return (s, Json(b)).into_response();
    }
    let drained = audit::drain_wal();
    tracing::warn!("wal drain called: {} records", drained.len());
    (
        StatusCode::OK,
        Json(serde_json::json!({"drained": drained.len()})),
    )
        .into_response()
}

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
    Router::new()
        .route("/", get(api_console_handler))
        .route("/ui", get(api_console_handler))
        .route("/health", get(health_handler))
        .route("/v1/auth/signup", post(email_signup_handler))
        .route("/v1/auth/login", post(email_login_handler))
        .route("/v1/auth/github/device", post(github_device_handler))
        .route("/v1/auth/github/poll", post(github_poll_handler))
        .route("/v1/auth/github/validate", post(github_validate_handler))
        .route("/v1/auth/google/url", post(google_url_handler))
        .route("/v1/auth/google/callback", post(google_callback_handler))
        .route("/v1/auth/google/verify", post(google_verify_handler))
        .route("/v1/orgs", post(create_org_handler))
        .route("/v1/policy/publish", post(publish_policy_handler))
        .route("/v1/policy/:version", get(get_policy_handler))
        .route("/v1/plans", get(plans_handler))
        .route("/v1/subscriptions", post(create_sub_handler))
        .route("/v1/subscriptions", get(get_sub_handler))
        .route("/v1/subscriptions/:id/change", post(change_sub_handler))
        .route("/v1/subscriptions/:id/cancel", post(cancel_sub_handler))
        .route("/v1/subscriptions/:id/activate", post(activate_sub_handler))
        .route("/v1/subscriptions/:id/seats", post(seats_handler))
        .route("/v1/entitlement", get(entitlement_handler))
        .route("/v1/audit/export", get(export_handler))
        .route("/v1/billing/webhook", post(billing_webhook_handler))
        .route("/v1/audit/ingest", post(ingest_audit_handler))
        .route("/v1/stats", get(stats_handler))
        .route("/v1/policy/dry-run", post(dry_run_handler))
        .route("/v1/wal/drain", post(wal_drain_handler))
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
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let state = AppState {
        policy_store: Arc::new(PolicyStore::new()),
        oauth: Arc::new(oauth_config::OAuthConfig::from_env()),
        http: http_client(),
    };
    let app = app_router(state);

    let addr = std::env::var("ALGO_BACKEND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("bind failed");
    tracing::info!("algo-backend listening on {}", addr);
    axum::serve(listener, app).await.expect("serve failed");
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
    use tower::ServiceExt; // for oneshot

    fn test_state() -> AppState {
        AppState {
            policy_store: Arc::new(PolicyStore::new()),
            oauth: Arc::new(oauth_config::OAuthConfig::disabled()),
            http: http_client(),
        }
    }

    fn oauth_state(cfg: oauth_config::OAuthConfig) -> AppState {
        AppState {
            policy_store: Arc::new(PolicyStore::new()),
            oauth: Arc::new(cfg),
            http: http_client(),
        }
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
        // The minted email session authorizes protected routes.
        let app5 = app_router(test_state());
        let req5 = Request::builder()
            .uri("/v1/stats?org_id=org_router")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let resp5 = app5.oneshot(req5).await.unwrap();
        assert_eq!(resp5.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn audit_ingest_drops_source_and_authed() {
        let _guard = test_sync::lock();
        audit::clear_audit_store();
        let token = "valid-token-test123";
        let app = app_router(test_state());
        let body = serde_json::json!({
            "redacted_event":"redacted ls -la",
            "decision":"allow",
            "latency_ms": 12,
            "source":"should be dropped",
            "trace_id":"trace-1"
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
        let token = "valid-token-test123";
        let app = app_router(test_state());
        let body = serde_json::json!({
            "redacted_event":"leaked ghp_12345678901234567890",
            "decision":"allow",
            "latency_ms": 5
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
        let token = "valid-token-test123";
        let app = app_router(state.clone());

        // Publish
        let req = Request::builder()
            .uri("/v1/policy/publish")
            .method("POST")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(r#"{"org_id":"org-1","content":"allow echo"}"#))
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
            .uri(format!("/v1/policy/{version}?org_id=org-1"))
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let resp2 = app2.oneshot(req2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn stats_after_ingest() {
        let _guard = test_sync::lock();
        audit::clear_audit_store();
        let token = "valid-token-test123";
        // Ingest two records
        for (decision, latency) in [("allow", 10), ("deny", 20)] {
            let app = app_router(test_state());
            let body = serde_json::json!({
                "redacted_event": format!("event {decision}"),
                "decision": decision,
                "latency_ms": latency,
                "org_id":"org-stats"
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
            .uri("/v1/stats?org_id=org-stats")
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
        let token = "valid-token-test123";
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
        subscriptions::create_subscription("org-dry", "pro", "monthly", 1, "valid-token-test123")
            .unwrap();
        let token = "valid-token-test123";
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
        // Session JWT unlocks a protected route (identity = github:1).
        let app2 = app_router(state);
        let req2 = Request::builder()
            .uri("/v1/audit/ingest")
            .method("POST")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {session}"))
            .body(Body::from(
                r#"{"redacted_event":"hello","decision":"allow","latency_ms":5}"#,
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
        let token = "valid-token-test123";
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

    fn billing_token() -> String {
        "valid-token-billing".to_string()
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
        subscriptions::create_subscription("org-pro", "pro", "monthly", 1, &token).unwrap();
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
        subscriptions::create_subscription("org-team", "team", "monthly", 2, &token).unwrap();
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
        let victim = "valid-token-victim";
        let attacker = "valid-token-attacker";
        // Victim owns a TEAM subscription on org-victim.
        let (status, body) = post_json(
            app_router(test_state()),
            "/v1/subscriptions",
            Some(victim),
            serde_json::json!({"org_id":"org-victim","tier":"team","cycle":"monthly","seats":2}),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let id = body.get("id").unwrap().as_str().unwrap().to_string();
        // Attacker reads victim org → free shape (no tier/status leak).
        let (status, body) = get_json(
            app_router(test_state()),
            "/v1/entitlement?org_id=org-victim",
            Some(attacker),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.get("tier").unwrap(), "free");
        assert_eq!(body.get("valid").unwrap(), false);
        // Attacker spends victim org on dry-run → 402 (not 200).
        let (status, body) = post_json(
            app_router(test_state()),
            "/v1/policy/dry-run",
            Some(attacker),
            serde_json::json!({"org_id":"org-victim","bundle":{"version":"1"},"history_ids":[]}),
        )
        .await;
        assert_eq!(status, StatusCode::PAYMENT_REQUIRED);
        assert_eq!(body["upgrade"].get("tier").unwrap(), "pro");
        // Attacker spends victim org on export → 402.
        let (status, _) = get_json(
            app_router(test_state()),
            "/v1/audit/export?org_id=org-victim",
            Some(attacker),
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
            let (status, _) = post_json(app_router(test_state()), &uri, Some(attacker), body).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        }
        let (status, _) = post_json(
            app_router(test_state()),
            &format!("/v1/subscriptions/{id}/activate"),
            Some(attacker),
            serde_json::json!({}),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        // Attacker cannot even re-subscribe over the live org (409 either way,
        // and it reveals nothing about who owns it).
        let (status, _) = post_json(
            app_router(test_state()),
            "/v1/subscriptions",
            Some(attacker),
            serde_json::json!({"org_id":"org-victim","tier":"pro","cycle":"monthly","seats":1}),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        // Victim still fully entitled.
        let (status, body) = get_json(
            app_router(test_state()),
            "/v1/entitlement?org_id=org-victim&feature=siem_export",
            Some(victim),
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
            .contains("ADR-0006"));
    }
}
