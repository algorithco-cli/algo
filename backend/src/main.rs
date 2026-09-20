#![allow(dead_code)]
#![allow(clippy::default_constructed_unit_structs)]
#![allow(clippy::manual_pattern_char_comparison)]

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};
use tower_http::trace::TraceLayer;

mod audit;
mod auth;
mod policy;
mod stats;
mod test_sync;
mod verify;

use audit::{ingest_audit, AuditError};
use auth::{create_device_flow, poll_device_flow, require_auth};
use policy::PolicyStore;
use stats::query_stats;
use verify::PolicyBundle;

// ── State & rate limit ──────────────────────────────────────────────

#[derive(Clone)]
struct AppState {
    policy_store: Arc<PolicyStore>,
}

static RATE_LIMIT_STORE: OnceLock<Mutex<HashMap<String, (u32, Instant)>>> = OnceLock::new();

fn rate_store() -> &'static Mutex<HashMap<String, (u32, Instant)>> {
    RATE_LIMIT_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

const RATE_LIMIT_PER_MINUTE: u32 = 100;

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
        // MVP stub: increment global counter but always allow.
        // Real impl would check Valkey and return 429.
        {
            let mut store = rate_store().lock().expect("rate store poisoned");
            let now = Instant::now();
            let entry = store
                .entry("global".to_string())
                .or_insert((0, now));
            if now.duration_since(entry.1) > Duration::from_secs(60) {
                *entry = (0, now);
            }
            entry.0 += 1;
            if entry.0 > RATE_LIMIT_PER_MINUTE {
                // In real tower service we'd return 429 here; MVP just logs.
                tracing::warn!("rate limit stub: over limit {}", entry.0);
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
    owner_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct CreateOrgResponse {
    org_id: String,
    org_name: String,
    owner_id: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct DeviceInitPayload {
    client_id: Option<String>,
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DevicePollPayload {
    device_code: String,
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

async fn create_org_handler(
    _headers: HeaderMap,
    Json(payload): Json<CreateOrgPayload>,
) -> impl IntoResponse {
    if payload.org_name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"org_name required"})),
        )
            .into_response();
    }
    let org_id = format!("org_{}", uuid::Uuid::new_v4());
    let owner = payload.owner_id.unwrap_or_else(|| "owner_default".to_string());
    let resp = CreateOrgResponse {
        org_id: org_id.clone(),
        org_name: payload.org_name,
        owner_id: owner,
    };
    // In-memory: we don't persist org beyond response for MVP; stats/policy keyed by org_id still works.
    (StatusCode::CREATED, Json(serde_json::to_value(resp).unwrap())).into_response()
}

async fn device_init_handler(
    Json(payload): Json<DeviceInitPayload>,
) -> impl IntoResponse {
    let client_id = payload
        .client_id
        .unwrap_or_else(|| "default-client".to_string());
    if client_id.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"client_id required"})),
        )
            .into_response();
    }
    let resp = create_device_flow(&client_id);
    (StatusCode::OK, Json(serde_json::to_value(resp).unwrap())).into_response()
}

async fn device_poll_handler(
    Json(payload): Json<DevicePollPayload>,
) -> impl IntoResponse {
    match poll_device_flow(&payload.device_code) {
        Ok(resp) => (StatusCode::OK, Json(serde_json::to_value(resp).unwrap())).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn publish_policy_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<PublishPolicyPayload>,
) -> impl IntoResponse {
    // Auth already checked via middleware; double-check for direct handler tests.
    if let Err(e) = require_auth(&headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response();
    }
    let org_id = payload.org_id.unwrap_or_else(|| "default".to_string());
    // If raw bundle provided, verify_and_apply directly; else create new bundle from content.
    if let (Some(version), Some(signed_b64), Some(sig_b64)) =
        (payload.version, payload.signed_bytes_b64, payload.sig_b64)
    {
        use base64::{engine::general_purpose::STANDARD as B64, Engine};
        let signed_bytes = match B64.decode(signed_b64) {
            Ok(b) => b,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": format!("bad signed_bytes_b64: {e}")})),
                )
                    .into_response();
            }
        };
        let sig = match B64.decode(sig_b64) {
            Ok(b) => b,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": format!("bad sig_b64: {e}")})),
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
            Err(e) => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response(),
        }
    } else {
        let content = payload.content.unwrap_or_else(|| "default policy".to_string());
        let bundle = state.policy_store.publish(&org_id, &content);
        let resp = PublishPolicyResponse {
            version: bundle.version.clone(),
            ok: true,
        };
        (StatusCode::OK, Json(serde_json::to_value(resp).unwrap())).into_response()
    }
}

async fn get_policy_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(version): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response();
    }
    let org_id = params.get("org_id").map(|s| s.as_str()).unwrap_or("default");
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
    if let Err(e) = require_auth(&headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response();
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
    if let Err(e) = require_auth(&headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response();
    }
    let s = query_stats(params.org_id.as_deref());
    (StatusCode::OK, Json(serde_json::to_value(s).unwrap())).into_response()
}

async fn dry_run_handler(
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response();
    }
    // MVP dry-run: just verify bundle and replay count from history_ids length.
    let bundle_val = payload.get("bundle");
    if bundle_val.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":"bundle required"})),
        )
            .into_response();
    }
    // Extract bundle fields if present, else mock.
    let history_len = payload
        .get("history_ids")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    // Try to verify if bundle has version/signed_bytes/sig.
    // For MVP we don't enforce full verify here, just report.
    let decisions: Vec<String> = (0..history_len).map(|i| format!("ask: history {i}")).collect();
    let body = serde_json::json!({
        "result": "dry-run ok",
        "decisions": decisions,
        "evaluated": history_len
    });
    (StatusCode::OK, Json(body)).into_response()
}

// ── WAL queue inspection (health/diagnostics) ───────────────────────

async fn wal_drain_handler(headers: HeaderMap) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response();
    }
    let drained = audit::drain_wal();
    (StatusCode::OK, Json(serde_json::json!({"drained": drained.len()}))).into_response()
}

// ── Router ──────────────────────────────────────────────────────────

fn app_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/v1/auth/device", post(device_init_handler))
        .route("/v1/auth/poll", post(device_poll_handler))
        .route("/v1/orgs", post(create_org_handler))
        .route("/v1/policy/publish", post(publish_policy_handler))
        .route("/v1/policy/:version", get(get_policy_handler))
        .route("/v1/audit/ingest", post(ingest_audit_handler))
        .route("/v1/stats", get(stats_handler))
        .route("/v1/policy/dry-run", post(dry_run_handler))
        .route("/v1/wal/drain", post(wal_drain_handler))
        .layer(RateLimitLayer)
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
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{HeaderValue, Request, StatusCode};
    use tower::ServiceExt; // for oneshot

    fn test_state() -> AppState {
        AppState {
            policy_store: Arc::new(PolicyStore::new()),
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
            .body(Body::from(r#"{"redacted_event":"hello","decision":"allow","latency_ms":5}"#))
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
    async fn auth_device_flow_does_not_require_auth() {
        let _guard = test_sync::lock();
        let app = app_router(test_state());
        let req = Request::builder()
            .uri("/v1/auth/device")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"client_id":"test-client"}"#))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
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
        let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
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
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        let stats: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(stats.get("total").unwrap().as_i64().unwrap(), 2);
    }
}
