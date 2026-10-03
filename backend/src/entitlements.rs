//! Account-service entitlement client and bounded in-memory cache.
//!
//! The account API is authoritative. Successful responses are cached for at
//! most the documented 60 seconds; an outage may use a last-known-good entry
//! only while its own `validUntil` remains valid. No billing data is written
//! to disk.

use crate::{account_auth::CompactEntitlement, auth::AuthenticatedUser, plans::FeatureSet};
use chrono::{DateTime, Utc};
use reqwest::{
    header::{CACHE_CONTROL, ETAG, IF_NONE_MATCH},
    Client, StatusCode, Url,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

const PRODUCT: &str = "guard";
const DOCUMENTED_MAX_AGE_SECS: u64 = 60;
const OWNER_DEFAULT_MAX_TTL_SECS: u64 = 300;
const OWNER_MAX_CONFIG_TTL_SECS: u64 = 300;
const DEFAULT_REFRESH_MIN_INTERVAL_SECS: u64 = 30;
const MAX_REFRESH_MIN_INTERVAL_SECS: u64 = 3600;
const DEFAULT_HTTP_TIMEOUT_SECS: u64 = 5;
const MAX_RESPONSE_BYTES: usize = 256 * 1024;
const MAX_CACHE_ENTRIES: usize = 1024;

#[derive(Clone)]
pub struct EntitlementConfig {
    endpoint_base: Url,
    service_key: String,
    max_ttl: Duration,
    refresh_min_interval: Duration,
    cache_capacity: usize,
}

impl EntitlementConfig {
    pub fn from_env() -> Result<Self, String> {
        let issuer = std::env::var("GUARD_ACCOUNT_ISSUER")
            .map_err(|_| "GUARD_ACCOUNT_ISSUER is required in account mode".to_string())?;
        let service_key = std::env::var("GUARD_ACCOUNT_SERVICE_KEY")
            .map_err(|_| "GUARD_ACCOUNT_SERVICE_KEY is required in account mode".to_string())?;
        let ttl = std::env::var("ENTITLEMENT_CACHE_MAX_TTL_SECS")
            .unwrap_or_else(|_| OWNER_DEFAULT_MAX_TTL_SECS.to_string());
        let refresh = std::env::var("ENTITLEMENT_REFRESH_MIN_INTERVAL_SECS")
            .unwrap_or_else(|_| DEFAULT_REFRESH_MIN_INTERVAL_SECS.to_string());
        Self::parse(&issuer, &service_key, &ttl, &refresh)
    }

    fn parse(issuer: &str, service_key: &str, ttl: &str, refresh: &str) -> Result<Self, String> {
        let mut endpoint_base = Url::parse(issuer)
            .map_err(|_| "GUARD_ACCOUNT_ISSUER must be a valid URL".to_string())?;
        if endpoint_base.scheme() != "https" && !endpoint_base.host_str().is_some_and(is_loopback) {
            return Err(
                "GUARD_ACCOUNT_ISSUER must use HTTPS except for a loopback development host"
                    .to_string(),
            );
        }
        if endpoint_base.query().is_some() || endpoint_base.fragment().is_some() {
            return Err("GUARD_ACCOUNT_ISSUER must not contain a query or fragment".to_string());
        }
        endpoint_base.set_path("/");
        if service_key.trim() != service_key
            || !service_key.starts_with("alg_sk_")
            || service_key.len() <= "alg_sk_".len()
        {
            return Err(
                "GUARD_ACCOUNT_SERVICE_KEY must be a non-empty alg_sk_ service credential"
                    .to_string(),
            );
        }
        let configured_ttl = parse_bounded(
            ttl,
            1,
            OWNER_MAX_CONFIG_TTL_SECS,
            "ENTITLEMENT_CACHE_MAX_TTL_SECS",
        )?;
        let refresh = parse_bounded(
            refresh,
            1,
            MAX_REFRESH_MIN_INTERVAL_SECS,
            "ENTITLEMENT_REFRESH_MIN_INTERVAL_SECS",
        )?;
        Ok(Self {
            endpoint_base,
            service_key: service_key.to_string(),
            // INTEGRATION_BILLING.md's max-age=60 is stricter than D7's
            // owner-approved configurable ceiling.
            max_ttl: Duration::from_secs(configured_ttl.min(DOCUMENTED_MAX_AGE_SECS)),
            refresh_min_interval: Duration::from_secs(refresh),
            cache_capacity: MAX_CACHE_ENTRIES,
        })
    }

    #[cfg(test)]
    fn test(endpoint_base: Url, max_ttl: Duration, capacity: usize) -> Self {
        Self {
            endpoint_base,
            service_key: "alg_sk_obviously-fake-test-key".to_string(),
            max_ttl,
            refresh_min_interval: Duration::from_millis(50),
            cache_capacity: capacity,
        }
    }
}

fn is_loopback(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "[::1]" | "::1")
}

fn parse_bounded(value: &str, min: u64, max: u64, name: &str) -> Result<u64, String> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| format!("{name} must be an integer from {min} through {max}"))?;
    if !(min..=max).contains(&parsed) {
        return Err(format!(
            "{name} must be an integer from {min} through {max}"
        ));
    }
    Ok(parsed)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccountEntitlement {
    pub product: String,
    pub plan: String,
    #[serde(default)]
    pub limits: serde_json::Map<String, serde_json::Value>,
    pub status: String,
    #[serde(rename = "validUntil")]
    pub valid_until: Option<DateTime<Utc>>,
    pub source: String,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    pub owner: EntitlementOwner,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntitlementOwner {
    #[serde(rename = "type")]
    pub owner_type: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EffectiveEntitlement {
    pub product: String,
    pub plan: String,
    pub status: String,
    pub valid_until: Option<DateTime<Utc>>,
    pub limits: serde_json::Map<String, serde_json::Value>,
    pub features: FeatureSet,
    pub source: String,
}

impl EffectiveEntitlement {
    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.get(feature) == Some(true)
    }

    pub fn rate_multiplier(&self) -> i64 {
        self.limits
            .get("rate_multiplier")
            .and_then(serde_json::Value::as_i64)
            .filter(|value| *value > 0)
            .unwrap_or(1)
    }
}

#[derive(Debug, Deserialize)]
struct EntitlementResponse {
    entitlements: Vec<AccountEntitlement>,
}

#[derive(Debug, Deserialize)]
struct PlanFeatureConfig {
    default_plan: String,
    upgrade_order: Vec<String>,
    plans: HashMap<String, FeatureSet>,
}

fn plan_config() -> &'static PlanFeatureConfig {
    static CONFIG: OnceLock<PlanFeatureConfig> = OnceLock::new();
    CONFIG.get_or_init(|| {
        serde_json::from_str(include_str!("../guard-plan-features.json"))
            .expect("guard-plan-features.json must be valid")
    })
}

pub fn min_plan_for_feature(feature: &str) -> Option<&'static str> {
    let config = plan_config();
    config
        .upgrade_order
        .iter()
        .find(|plan| {
            config
                .plans
                .get(plan.as_str())
                .and_then(|features| features.get(feature))
                == Some(true)
        })
        .map(String::as_str)
}

fn free_entitlement(source: &str) -> EffectiveEntitlement {
    let config = plan_config();
    EffectiveEntitlement {
        product: PRODUCT.to_string(),
        plan: config.default_plan.clone(),
        status: "active".to_string(),
        valid_until: None,
        limits: serde_json::Map::new(),
        features: config
            .plans
            .get(&config.default_plan)
            .cloned()
            .unwrap_or_else(FeatureSet::free),
        source: source.to_string(),
    }
}

fn effective(item: AccountEntitlement) -> EffectiveEntitlement {
    let Some(features) = plan_config().plans.get(&item.plan).cloned() else {
        tracing::warn!("account returned an unknown Guard plan; using free feature mapping");
        return free_entitlement("unknown-plan-fallback");
    };
    EffectiveEntitlement {
        product: item.product,
        plan: item.plan,
        status: item.status,
        valid_until: item.valid_until,
        limits: item.limits,
        features,
        source: item.source,
    }
}

fn from_compact(
    item: &CompactEntitlement,
    token_iat: i64,
    token_exp: i64,
    max_ttl: Duration,
) -> Option<EffectiveEntitlement> {
    let now = Utc::now().timestamp();
    let claim_exp = item.exp.unwrap_or(token_exp).min(token_exp);
    let freshness_exp = token_iat.saturating_add(max_ttl.as_secs() as i64);
    let expires = claim_exp.min(freshness_exp);
    if item.p != PRODUCT || expires <= now {
        return None;
    }
    let features = plan_config().plans.get(&item.plan)?.clone();
    Some(EffectiveEntitlement {
        product: PRODUCT.to_string(),
        plan: item.plan.clone(),
        status: "active".to_string(),
        valid_until: DateTime::from_timestamp(expires, 0),
        limits: serde_json::Map::new(),
        features,
        source: "token-claim".to_string(),
    })
}

#[derive(Clone)]
struct CacheEntry {
    entitlement: EffectiveEntitlement,
    etag: Option<String>,
    fresh_until: Instant,
    last_access: u64,
}

#[derive(Default)]
struct CacheState {
    entries: HashMap<String, CacheEntry>,
    sequence: u64,
    refreshes: HashMap<String, Instant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshError {
    RateLimited { retry_after_secs: u64 },
}

#[derive(Clone)]
pub struct AccountEntitlementService {
    config: EntitlementConfig,
    client: Client,
    cache: Arc<Mutex<CacheState>>,
    flights: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
}

impl AccountEntitlementService {
    pub fn from_env(client: Client) -> Result<Self, String> {
        Ok(Self::new(EntitlementConfig::from_env()?, client))
    }

    fn new(config: EntitlementConfig, client: Client) -> Self {
        Self {
            config,
            client,
            cache: Arc::new(Mutex::new(CacheState::default())),
            flights: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[cfg(test)]
    pub fn test(
        endpoint_base: Url,
        max_ttl: Duration,
        refresh_min_interval: Duration,
        capacity: usize,
    ) -> Self {
        let mut config = EntitlementConfig::test(endpoint_base, max_ttl, capacity);
        config.refresh_min_interval = refresh_min_interval;
        Self::new(config, Client::new())
    }

    pub async fn resolve(&self, user: &AuthenticatedUser) -> EffectiveEntitlement {
        if let (Some(items), Some(token_iat), Some(token_exp)) =
            (&user.entitlements, user.token_iat, user.token_exp)
        {
            if let Some(entitlement) = items
                .iter()
                .find_map(|item| from_compact(item, token_iat, token_exp, self.config.max_ttl))
            {
                return entitlement;
            }
        }
        self.resolve_api(&user.sub, false).await
    }

    pub async fn refresh(
        &self,
        user: &AuthenticatedUser,
    ) -> Result<EffectiveEntitlement, RefreshError> {
        {
            let mut cache = self.cache.lock().await;
            let now = Instant::now();
            if let Some(elapsed) = cache
                .refreshes
                .get(&user.sub)
                .map(|last| now.duration_since(*last))
                .filter(|elapsed| *elapsed < self.config.refresh_min_interval)
            {
                let retry_after_secs = self
                    .config
                    .refresh_min_interval
                    .saturating_sub(elapsed)
                    .as_secs()
                    .max(1);
                return Err(RefreshError::RateLimited { retry_after_secs });
            }
            if !cache.refreshes.contains_key(&user.sub)
                && cache.refreshes.len() >= self.config.cache_capacity
            {
                if let Some(oldest) = cache
                    .refreshes
                    .iter()
                    .min_by_key(|(_, at)| **at)
                    .map(|(sub, _)| sub.clone())
                {
                    cache.refreshes.remove(&oldest);
                }
            }
            cache.refreshes.insert(user.sub.clone(), now);
        }
        Ok(self.resolve_api(&user.sub, true).await)
    }

    async fn resolve_api(&self, sub: &str, bypass_fresh: bool) -> EffectiveEntitlement {
        if !bypass_fresh {
            if let Some(hit) = self.fresh_cache(sub).await {
                return hit;
            }
        }
        let flight = {
            let mut flights = self.flights.lock().await;
            flights
                .entry(sub.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        let _singleflight = flight.lock().await;
        if !bypass_fresh {
            if let Some(hit) = self.fresh_cache(sub).await {
                return hit;
            }
        }
        let previous = self.cache_entry(sub).await;
        let result = match self.fetch(sub, previous.as_ref()).await {
            Ok(FetchResult::Modified(entitlement, etag, ttl)) => {
                self.insert(sub, entitlement.clone(), etag, ttl).await;
                entitlement
            }
            Ok(FetchResult::NotModified(ttl)) => {
                if let Some(mut entry) = previous {
                    entry.fresh_until = Instant::now() + ttl;
                    let entitlement = entry.entitlement.clone();
                    self.insert_entry(sub, entry).await;
                    entitlement
                } else {
                    tracing::warn!("account entitlement API returned 304 without a cached entry");
                    free_entitlement("cold-cache-fallback")
                }
            }
            Err(()) => {
                tracing::warn!("account entitlement lookup failed; applying bounded fallback");
                previous
                    .filter(|entry| last_known_good_valid(&entry.entitlement))
                    .map(|entry| entry.entitlement)
                    .unwrap_or_else(|| free_entitlement("outage-free-fallback"))
            }
        };
        drop(_singleflight);
        let mut flights = self.flights.lock().await;
        if Arc::strong_count(&flight) <= 2 {
            flights.remove(sub);
        }
        result
    }

    async fn fresh_cache(&self, sub: &str) -> Option<EffectiveEntitlement> {
        let mut cache = self.cache.lock().await;
        let now = Instant::now();
        let next = cache.sequence.saturating_add(1);
        cache.sequence = next;
        let entry = cache.entries.get_mut(sub)?;
        entry.last_access = next;
        (entry.fresh_until > now).then(|| entry.entitlement.clone())
    }

    async fn cache_entry(&self, sub: &str) -> Option<CacheEntry> {
        self.cache.lock().await.entries.get(sub).cloned()
    }

    async fn insert(
        &self,
        sub: &str,
        entitlement: EffectiveEntitlement,
        etag: Option<String>,
        ttl: Duration,
    ) {
        self.insert_entry(
            sub,
            CacheEntry {
                entitlement,
                etag,
                fresh_until: Instant::now() + ttl,
                last_access: 0,
            },
        )
        .await;
    }

    async fn insert_entry(&self, sub: &str, mut entry: CacheEntry) {
        let mut cache = self.cache.lock().await;
        cache.sequence = cache.sequence.saturating_add(1);
        entry.last_access = cache.sequence;
        if !cache.entries.contains_key(sub) && cache.entries.len() >= self.config.cache_capacity {
            if let Some(oldest) = cache
                .entries
                .iter()
                .min_by_key(|(_, value)| value.last_access)
                .map(|(key, _)| key.clone())
            {
                cache.entries.remove(&oldest);
            }
        }
        cache.entries.insert(sub.to_string(), entry);
    }

    async fn fetch(&self, sub: &str, previous: Option<&CacheEntry>) -> Result<FetchResult, ()> {
        let mut url = self.config.endpoint_base.clone();
        url.path_segments_mut()
            .map_err(|_| ())?
            .extend(["v1", "users", sub, "entitlements"]);
        let mut request = self
            .client
            .get(url)
            .bearer_auth(&self.config.service_key)
            .timeout(Duration::from_secs(DEFAULT_HTTP_TIMEOUT_SECS));
        if let Some(etag) = previous.and_then(|entry| entry.etag.as_ref()) {
            request = request.header(IF_NONE_MATCH, etag);
        }
        let mut response = request.send().await.map_err(|_| ())?;
        let ttl = cache_ttl(response.headers(), self.config.max_ttl);
        if response.status() == StatusCode::NOT_MODIFIED {
            return Ok(FetchResult::NotModified(ttl));
        }
        if !response.status().is_success() {
            return Err(());
        }
        let etag = response
            .headers()
            .get(ETAG)
            .and_then(|value| value.to_str().ok())
            .filter(|value| !value.is_empty() && value.len() <= 512)
            .map(str::to_string);
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| ())? {
            if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                return Err(());
            }
            body.extend_from_slice(&chunk);
        }
        let parsed: EntitlementResponse = serde_json::from_slice(&body).map_err(|_| ())?;
        if parsed.entitlements.len() > 64 {
            return Err(());
        }
        let mut guard = parsed
            .entitlements
            .into_iter()
            .filter(|item| item.product == PRODUCT);
        let selected = guard.next();
        if guard.next().is_some() {
            return Err(());
        }
        if selected.as_ref().is_some_and(|item| {
            item.plan.is_empty()
                || !matches!(item.status.as_str(), "active" | "grace")
                || !matches!(item.source.as_str(), "subscription" | "manual" | "default")
                || !matches!(item.owner.owner_type.as_str(), "user" | "org")
                || item.owner.id.is_empty()
        }) {
            return Err(());
        }
        let entitlement = selected
            .map(effective)
            .unwrap_or_else(|| free_entitlement("api"));
        Ok(FetchResult::Modified(entitlement, etag, ttl))
    }

    #[cfg(test)]
    async fn cache_len(&self) -> usize {
        self.cache.lock().await.entries.len()
    }
}

enum FetchResult {
    Modified(EffectiveEntitlement, Option<String>, Duration),
    NotModified(Duration),
}

fn last_known_good_valid(entitlement: &EffectiveEntitlement) -> bool {
    match entitlement.valid_until {
        Some(until) => until > Utc::now(),
        None => false,
    }
}

fn cache_ttl(headers: &reqwest::header::HeaderMap, hard_max: Duration) -> Duration {
    let max_age = headers
        .get(CACHE_CONTROL)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value.split(',').find_map(|directive| {
                directive
                    .trim()
                    .strip_prefix("max-age=")
                    .and_then(|seconds| seconds.parse::<u64>().ok())
            })
        })
        .map(Duration::from_secs)
        .unwrap_or(Duration::ZERO);
    max_age.min(hard_max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        extract::State,
        http::{HeaderMap, StatusCode as AxumStatus},
        response::IntoResponse,
        routing::get,
        Router,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Clone)]
    struct MockState {
        calls: Arc<AtomicUsize>,
        mode: Arc<AtomicUsize>,
    }

    async fn mock_handler(State(state): State<MockState>, headers: HeaderMap) -> impl IntoResponse {
        state.calls.fetch_add(1, Ordering::SeqCst);
        match state.mode.load(Ordering::SeqCst) {
            1 => return (AxumStatus::SERVICE_UNAVAILABLE, "down").into_response(),
            2 => return (AxumStatus::OK, "not json").into_response(),
            3 => {
                return (
                    AxumStatus::OK,
                    "x".repeat(MAX_RESPONSE_BYTES.saturating_add(1)),
                )
                    .into_response()
            }
            _ => {}
        }
        if headers
            .get(IF_NONE_MATCH)
            .and_then(|value| value.to_str().ok())
            == Some("\"v1\"")
        {
            return (
                AxumStatus::NOT_MODIFIED,
                [(CACHE_CONTROL, "private, max-age=60")],
                "",
            )
                .into_response();
        }
        (
            AxumStatus::OK,
            [
                (CACHE_CONTROL, "private, max-age=60, stale-if-error=300"),
                (ETAG, "\"v1\""),
            ],
            axum::Json(serde_json::json!({"entitlements":[{
                "product":"guard","plan":"pro","limits":{},"status":"active",
                "validUntil":"2099-01-01T00:00:00Z","source":"default","sourceId":"free",
                "owner":{"type":"user","id":"subject"}
            }]})),
        )
            .into_response()
    }

    async fn service(ttl: Duration, capacity: usize) -> (AccountEntitlementService, MockState) {
        let state = MockState {
            calls: Arc::new(AtomicUsize::new(0)),
            mode: Arc::new(AtomicUsize::new(0)),
        };
        let app = Router::new()
            .route("/v1/users/:sub/entitlements", get(mock_handler))
            .with_state(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let url = Url::parse(&format!("http://{address}/")).unwrap();
        (
            AccountEntitlementService::new(
                EntitlementConfig::test(url, ttl, capacity),
                Client::new(),
            ),
            state,
        )
    }

    fn user(sub: &str) -> AuthenticatedUser {
        AuthenticatedUser {
            sub: sub.to_string(),
            email: None,
            email_verified: None,
            token_iat: Some(Utc::now().timestamp()),
            token_exp: Some(Utc::now().timestamp() + 300),
            entitlements: None,
        }
    }

    #[test]
    fn configuration_is_strict_and_docs_ttl_wins() {
        let config =
            EntitlementConfig::parse("https://auth.algorithco.com", "alg_sk_fake", "300", "30")
                .unwrap();
        assert_eq!(config.max_ttl, Duration::from_secs(60));
        assert!(EntitlementConfig::parse("http://example.com", "alg_sk_x", "1", "30").is_err());
        assert!(EntitlementConfig::parse("https://example.com", "secret", "1", "30").is_err());
        assert!(EntitlementConfig::parse("https://example.com", "alg_sk_x", "0", "30").is_err());
        assert_eq!(min_plan_for_feature("dry_run"), Some("pro"));
        assert_eq!(min_plan_for_feature("siem_export"), Some("team"));
        assert_eq!(min_plan_for_feature("unknown"), None);
    }

    #[tokio::test]
    async fn cache_hit_expiry_and_etag_304_revalidation() {
        let (service, state) = service(Duration::from_millis(30), 4).await;
        assert_eq!(service.resolve(&user("one")).await.plan, "pro");
        assert_eq!(service.resolve(&user("one")).await.plan, "pro");
        assert_eq!(state.calls.load(Ordering::SeqCst), 1);
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert_eq!(service.resolve(&user("one")).await.plan, "pro");
        assert_eq!(state.calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn compact_claim_precedes_api_and_expired_claim_falls_back() {
        let (service, state) = service(Duration::from_secs(1), 4).await;
        let mut claimed = user("claim");
        claimed.entitlements = Some(vec![CompactEntitlement {
            p: "guard".to_string(),
            plan: "team".to_string(),
            exp: Some(Utc::now().timestamp() + 60),
        }]);
        assert_eq!(service.resolve(&claimed).await.plan, "team");
        assert_eq!(state.calls.load(Ordering::SeqCst), 0);
        claimed.entitlements.as_mut().unwrap()[0].exp = Some(Utc::now().timestamp() - 1);
        assert_eq!(service.resolve(&claimed).await.plan, "pro");
        assert_eq!(state.calls.load(Ordering::SeqCst), 1);

        let mut old_claim = user("old-claim");
        old_claim.token_iat = Some(Utc::now().timestamp() - 120);
        old_claim.entitlements = Some(vec![CompactEntitlement {
            p: "guard".to_string(),
            plan: "team".to_string(),
            exp: Some(Utc::now().timestamp() + 60),
        }]);
        assert_eq!(service.resolve(&old_claim).await.plan, "pro");
        assert_eq!(state.calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn outage_uses_valid_warm_cache_and_cold_cache_is_free() {
        let (service, state) = service(Duration::ZERO, 4).await;
        assert_eq!(service.resolve(&user("warm")).await.plan, "pro");
        state.mode.store(1, Ordering::SeqCst);
        assert_eq!(service.resolve(&user("warm")).await.plan, "pro");
        assert_eq!(service.resolve(&user("cold")).await.plan, "free");
    }

    #[tokio::test]
    async fn outage_never_serves_last_known_good_past_or_without_valid_until() {
        let (service, state) = service(Duration::ZERO, 4).await;
        service
            .insert(
                "expired",
                EffectiveEntitlement {
                    product: "guard".to_string(),
                    plan: "team".to_string(),
                    status: "active".to_string(),
                    valid_until: DateTime::from_timestamp(Utc::now().timestamp() - 1, 0),
                    limits: serde_json::Map::new(),
                    features: plan_config().plans["team"].clone(),
                    source: "test".to_string(),
                },
                Some("\"expired\"".to_string()),
                Duration::ZERO,
            )
            .await;
        state.mode.store(1, Ordering::SeqCst);
        assert_eq!(service.resolve(&user("expired")).await.plan, "free");

        service
            .insert(
                "missing-valid-until",
                EffectiveEntitlement {
                    product: "guard".to_string(),
                    plan: "team".to_string(),
                    status: "active".to_string(),
                    valid_until: None,
                    limits: serde_json::Map::new(),
                    features: plan_config().plans["team"].clone(),
                    source: "test".to_string(),
                },
                Some("\"missing-valid-until\"".to_string()),
                Duration::ZERO,
            )
            .await;
        assert_eq!(
            service.resolve(&user("missing-valid-until")).await.plan,
            "free"
        );
    }

    #[tokio::test]
    async fn singleflight_and_bounded_eviction() {
        let (service, state) = service(Duration::from_secs(1), 2).await;
        let same_a = user("same");
        let same_b = user("same");
        let first = service.resolve(&same_a);
        let second = service.resolve(&same_b);
        let (a, b) = tokio::join!(first, second);
        assert_eq!((a.plan.as_str(), b.plan.as_str()), ("pro", "pro"));
        assert_eq!(state.calls.load(Ordering::SeqCst), 1);
        service.resolve(&user("two")).await;
        service.resolve(&user("three")).await;
        assert_eq!(service.cache_len().await, 2);
    }

    #[tokio::test]
    async fn refresh_is_per_subject_and_rate_limited() {
        let (service, state) = service(Duration::from_secs(1), 4).await;
        assert_eq!(service.refresh(&user("one")).await.unwrap().plan, "pro");
        assert_eq!(
            service.refresh(&user("one")).await,
            Err(RefreshError::RateLimited {
                retry_after_secs: 1
            })
        );
        assert_eq!(service.refresh(&user("two")).await.unwrap().plan, "pro");
        assert_eq!(state.calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn malformed_and_oversized_responses_fail_to_free() {
        let (service, state) = service(Duration::ZERO, 4).await;
        state.mode.store(2, Ordering::SeqCst);
        assert_eq!(service.resolve(&user("bad-json")).await.plan, "free");
        state.mode.store(3, Ordering::SeqCst);
        assert_eq!(service.resolve(&user("oversized")).await.plan, "free");
    }

    #[tokio::test]
    async fn simulated_restart_is_cold_and_refetches() {
        let (service, state) = service(Duration::from_secs(1), 4).await;
        service.resolve(&user("subject")).await;
        let fresh = AccountEntitlementService::new(service.config.clone(), Client::new());
        fresh.resolve(&user("subject")).await;
        assert_eq!(state.calls.load(Ordering::SeqCst), 2);
    }
}
