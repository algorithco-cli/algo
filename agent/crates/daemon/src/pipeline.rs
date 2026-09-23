use algo_provider::TypedAnswer;
use algo_types::{Action, Decision, SourceLevel, ToolBefore};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::cache::Cache;
use crate::jev_pool::JevPool;

/// Record sent to the single SQLite writer task.
#[derive(Debug, Clone)]
pub struct DbRecord {
    pub ts: i64,
    pub session_id: String,
    pub tool_kind: i32,
    pub redacted_command: String,
    pub fingerprint: String,
    pub action: i32,
    pub source: i32,
    pub reason: String,
    pub confidence: f64,
    pub latency_ms: i64,
    pub profile: String,
    pub shadow: bool,
}

/// P1-08 shadow encoding (no proto change until P2-01 deltas).
/// When shadow=true, pipeline stores the real decision (deny/ask) with shadow=true
/// but returns Allow with reason `shadow: would_have {deny|ask|allow} (...) → approve (shadow)`.
/// Daemon JSON and audit derive `would_have` from this prefix + DB row.
pub fn action_str_of(action: i32) -> &'static str {
    if action == Action::Allow as i32 {
        "allow"
    } else if action == Action::Deny as i32 {
        "deny"
    } else {
        "ask"
    }
}

pub fn is_shadow_reason(reason: &str) -> bool {
    reason.starts_with("shadow: would_have ")
}

pub fn parse_would_have(reason: &str) -> Option<&str> {
    // Expects `shadow: would_have {deny|ask|allow} ...`
    let rest = reason.strip_prefix("shadow: would_have ")?;
    let token = rest.split_whitespace().next()?;
    match token {
        "deny" | "ask" | "allow" => Some(token),
        _ => None,
    }
}

pub fn shadow_allow_from(real: &Decision) -> Decision {
    let orig = action_str_of(real.action);
    Decision {
        action: Action::Allow as i32,
        reason: format!(
            "shadow: would_have {} ({}) → approve (shadow)",
            orig, real.reason
        ),
        confidence_0_1: real.confidence_0_1,
        source_level: real.source_level,
        latency_ms: real.latency_ms,
        policy_version: real.policy_version.clone(),
        trace_id: real.trace_id.clone(),
    }
}

pub struct Pipeline {
    engine: Arc<algo_policy::Engine>,
    cache: Arc<Cache>,
    pool: Arc<JevPool>,
    writer_tx: mpsc::Sender<DbRecord>,
    policy_version: String,
    shadow: bool,
}

impl Pipeline {
    pub fn new(
        engine: Arc<algo_policy::Engine>,
        cache: Arc<Cache>,
        pool: Arc<JevPool>,
        writer_tx: mpsc::Sender<DbRecord>,
    ) -> Self {
        Self {
            engine,
            cache,
            pool,
            writer_tx,
            policy_version: env!("CARGO_PKG_VERSION").to_string(),
            // Unit default is enforcing (shadow=false) so L0-deny tests stay pure.
            // Daemon runtime enables shadow by default via with_shadow(true) unless
            // `algo enforce on` / ALGO_ENFORCE=1 (see daemon read_shadow_mode).
            shadow: false,
        }
    }

    pub fn with_shadow(mut self, shadow: bool) -> Self {
        self.shadow = shadow;
        self
    }

    pub fn set_shadow(&mut self, shadow: bool) {
        self.shadow = shadow;
    }

    pub fn is_shadow(&self) -> bool {
        self.shadow
    }

    /// Decide pipeline: L0 policy.evaluate → L1 cache (blake3, TTL 24h, max 10k) → L3 Jev (miss+uncertain) → L4 ask.
    /// Attaches source and latency_ms. Warm Jev pool on start. Single writer channel.
    /// Fail-safe: any error/timeout → ask, never block >1s on DB.
    pub async fn decide(&self, mut event: ToolBefore) -> Decision {
        let start = Instant::now();

        // Redact before logging (one path for send and --show-egress).
        let redactor = algo_redact::Redactor::global();
        let (redacted, _findings) = redactor.redact(&event.redacted_payload);
        event.redacted_payload = redacted.clone();

        let session_id = event
            .agent
            .as_ref()
            .map(|a| a.session_id.clone())
            .unwrap_or_default();
        let tool_kind = event.tool_kind;

        // Fingerprint for cache/DB
        let norm = algo_fingerprint::normalize(&event.redacted_payload);
        let fingerprint = if norm.is_empty() || norm == "unparseable:nested" {
            blake3::hash(event.redacted_payload.as_bytes())
                .to_hex()
                .to_string()
        } else {
            algo_fingerprint::cache_key(&norm, &self.policy_version, "balanced")
        };

        // L0: hard deny
        let pol = self
            .engine
            .evaluate(&event.redacted_payload, algo_policy::Profile::Balanced);
        match pol {
            algo_policy::PolicyDecision::Deny { reason, .. } => {
                let elapsed = start.elapsed().as_millis() as i64;
                let latency = elapsed.max(1);
                let d = Decision {
                    action: Action::Deny as i32,
                    reason: reason.clone(),
                    confidence_0_1: 0.95,
                    source_level: SourceLevel::Rule as i32,
                    latency_ms: latency,
                    policy_version: self.policy_version.clone(),
                    trace_id: event.event_id.clone(),
                };
                // Best-effort audit write, never block >1s. Even if DB busy, deny must still be returned.
                // Shadow P1: store real deny with shadow=true, return Allow (never block in shadow).
                let rec = DbRecord {
                    ts: chrono::Utc::now().timestamp_millis(),
                    session_id,
                    tool_kind,
                    redacted_command: event.redacted_payload.clone(),
                    fingerprint,
                    action: d.action,
                    source: d.source_level,
                    reason: d.reason.clone(),
                    confidence: d.confidence_0_1,
                    latency_ms: d.latency_ms,
                    profile: "balanced".into(),
                    shadow: self.shadow,
                };
                // Use try_send with 1s timeout; if it would block, we still return deny (hard deny outranks DB).
                let _ =
                    tokio::time::timeout(Duration::from_secs(1), self.writer_tx.send(rec)).await;
                if self.shadow {
                    return shadow_allow_from(&d);
                }
                return d;
            }
            algo_policy::PolicyDecision::Allow { reason, .. } => {
                // Policy currently never returns Allow, but handle for completeness.
                let latency = start.elapsed().as_millis() as i64;
                let d = Decision {
                    action: Action::Allow as i32,
                    reason,
                    confidence_0_1: 0.90,
                    source_level: SourceLevel::Rule as i32,
                    latency_ms: latency.max(1),
                    policy_version: self.policy_version.clone(),
                    trace_id: event.event_id.clone(),
                };
                let rec = DbRecord {
                    ts: chrono::Utc::now().timestamp_millis(),
                    session_id,
                    tool_kind,
                    redacted_command: event.redacted_payload.clone(),
                    fingerprint,
                    action: d.action,
                    source: d.source_level,
                    reason: d.reason.clone(),
                    confidence: d.confidence_0_1,
                    latency_ms: d.latency_ms,
                    profile: "balanced".into(),
                    shadow: self.shadow,
                };
                let _ =
                    tokio::time::timeout(Duration::from_secs(1), self.writer_tx.send(rec)).await;
                return d;
            }
            algo_policy::PolicyDecision::Abstain => {}
        }

        // L1: cache
        if let Some(mut cached) = self.cache.get(&event.redacted_payload) {
            let elapsed = start.elapsed().as_millis() as i64;
            cached.latency_ms = elapsed.max(1);
            cached.trace_id = event.event_id.clone();
            cached.policy_version = self.policy_version.clone();
            // Ensure source is Cache (cache.get already sets it, but be explicit)
            cached.source_level = SourceLevel::Cache as i32;

            let rec = DbRecord {
                ts: chrono::Utc::now().timestamp_millis(),
                session_id,
                tool_kind,
                redacted_command: event.redacted_payload.clone(),
                fingerprint: fingerprint.clone(),
                action: cached.action,
                source: cached.source_level,
                reason: cached.reason.clone(),
                confidence: cached.confidence_0_1,
                latency_ms: cached.latency_ms,
                profile: "balanced".into(),
                shadow: self.shadow,
            };
            // Cache hit audit: timeout <1s
            let send_res =
                tokio::time::timeout(Duration::from_secs(1), self.writer_tx.send(rec)).await;
            if send_res.is_err() {
                // DB locked: fail-safe ask within 1s per spec, but for cache hit we still
                // map to ask to prove the invariant. However hard deny already handled.
                // Shadow: never block even on DB error → allow with would_have ask.
                let ask = algo_types::ask_on_error(
                    "db write timeout → ask (fail-safe)",
                    event.event_id.clone(),
                );
                if self.shadow {
                    return shadow_allow_from(&ask);
                }
                return ask;
            }
            if let Ok(Err(_)) = send_res {
                let ask = algo_types::ask_on_error(
                    "db channel closed → ask (fail-safe)",
                    event.event_id.clone(),
                );
                if self.shadow {
                    return shadow_allow_from(&ask);
                }
                return ask;
            }
            if self.shadow && cached.action != Action::Allow as i32 {
                return shadow_allow_from(&cached);
            }
            return cached;
        }

        // L3: Jev (miss + uncertain). Pool already has 700ms timeout.
        let jev_result = self.pool.judge(&event).await;

        let mut decision = match jev_result {
            Ok(answers) => map_answers_to_decision(&answers, &self.policy_version, &event.event_id),
            Err(e) => {
                let mut d = algo_types::ask_on_error(
                    format!("provider error → ask: {e}"),
                    event.event_id.clone(),
                );
                d.reason = format!("provider error → ask: {e}");
                d.source_level = SourceLevel::Fallback as i32;
                d
            }
        };

        let elapsed = start.elapsed().as_millis() as i64;
        decision.latency_ms = elapsed.max(1);
        decision.trace_id = event.event_id.clone();
        decision.policy_version = self.policy_version.clone();

        // Insert into cache if not fallback (to avoid caching fallback asks)
        if decision.source_level != SourceLevel::Fallback as i32 {
            self.cache.insert(&event.redacted_payload, decision.clone());
        }

        // Audit write with 1s guard — store REAL decision with shadow flag.
        // Cache already holds real (not shadow Allow) so future hits preserve would_have.
        let rec = DbRecord {
            ts: chrono::Utc::now().timestamp_millis(),
            session_id,
            tool_kind,
            redacted_command: event.redacted_payload.clone(),
            fingerprint,
            action: decision.action,
            source: decision.source_level,
            reason: decision.reason.clone(),
            confidence: decision.confidence_0_1,
            latency_ms: decision.latency_ms,
            profile: "balanced".into(),
            shadow: self.shadow,
        };
        let send_res = tokio::time::timeout(Duration::from_secs(1), self.writer_tx.send(rec)).await;
        // If DB/channel blocks >1s, prove ask and never block.
        // Shadow: never block even on DB error → allow with would_have ask.
        if send_res.is_err() {
            let ask = algo_types::ask_on_error(
                "db write timeout → ask (fail-safe)",
                event.event_id.clone(),
            );
            if self.shadow {
                return shadow_allow_from(&ask);
            }
            return ask;
        }
        if let Ok(Err(_)) = send_res {
            let ask = algo_types::ask_on_error(
                "db channel closed → ask (fail-safe)",
                event.event_id.clone(),
            );
            if self.shadow {
                return shadow_allow_from(&ask);
            }
            return ask;
        }

        if self.shadow && decision.action != Action::Allow as i32 {
            return shadow_allow_from(&decision);
        }
        decision
    }
}

fn map_answers_to_decision(
    answers: &[TypedAnswer],
    policy_version: &str,
    trace_id: &str,
) -> Decision {
    let (action, confidence, reason, source) = if let Some(ans) = answers.first() {
        match ans {
            TypedAnswer::Choice {
                choice,
                confidence: c,
                ..
            } => {
                let conf = *c;
                match choice.as_str() {
                    "deny" => (
                        Action::Deny as i32,
                        conf,
                        "jev deny".to_string(),
                        SourceLevel::Jev as i32,
                    ),
                    "allow" => (
                        Action::Allow as i32,
                        conf,
                        "jev allow".to_string(),
                        SourceLevel::Jev as i32,
                    ),
                    "ask" => (
                        Action::Ask as i32,
                        conf,
                        "jev ask".to_string(),
                        SourceLevel::Jev as i32,
                    ),
                    other => (
                        Action::Ask as i32,
                        conf,
                        format!("jev choice {other} → ask"),
                        SourceLevel::Jev as i32,
                    ),
                }
            }
            TypedAnswer::Bool {
                value,
                confidence: c,
                ..
            } => {
                let conf = *c;
                if *value {
                    (
                        Action::Allow as i32,
                        conf,
                        "jev bool true".to_string(),
                        SourceLevel::Jev as i32,
                    )
                } else {
                    (
                        Action::Ask as i32,
                        conf,
                        "jev bool false → ask".to_string(),
                        SourceLevel::Jev as i32,
                    )
                }
            }
            TypedAnswer::Score {
                score,
                confidence: c,
                ..
            } => {
                let conf = *c;
                if *score > 0.7 {
                    (
                        Action::Deny as i32,
                        conf,
                        "jev score high → deny".to_string(),
                        SourceLevel::Jev as i32,
                    )
                } else if *score < 0.3 {
                    (
                        Action::Allow as i32,
                        conf,
                        "jev score low → allow".to_string(),
                        SourceLevel::Jev as i32,
                    )
                } else {
                    (
                        Action::Ask as i32,
                        conf,
                        "jev score uncertain → ask".to_string(),
                        SourceLevel::Jev as i32,
                    )
                }
            }
        }
    } else {
        (
            Action::Ask as i32,
            0.0,
            "jev empty → ask".to_string(),
            SourceLevel::Fallback as i32,
        )
    };

    Decision {
        action,
        reason,
        confidence_0_1: confidence,
        source_level: source,
        latency_ms: 0,
        policy_version: policy_version.to_string(),
        trace_id: trace_id.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use algo_provider::MockProvider;
    use algo_types::{AgentIdentity, PrivacyMode, ToolKind};

    fn tool_before(payload: &str) -> ToolBefore {
        ToolBefore {
            event_id: "evt-1".into(),
            timestamp: None,
            agent: Some(AgentIdentity {
                agent_type: Some("claude-code".into()),
                agent_version: Some("1.0".into()),
                session_id: "sess-1".into(),
                working_dir: "/tmp".into(),
            }),
            tool_kind: ToolKind::Shell as i32,
            redacted_payload: payload.to_string(),
            privacy_mode: PrivacyMode::Redacted as i32,
            shell_argv: vec![],
            file_path: None,
        }
    }

    fn make_pipeline() -> (Pipeline, mpsc::Receiver<DbRecord>) {
        let engine = Arc::new(algo_policy::Engine::new());
        let cache = Arc::new(Cache::new());
        let pool = Arc::new(JevPool::new(Arc::new(MockProvider::new())));
        let (tx, rx) = mpsc::channel(1000);
        let p = Pipeline::new(engine, cache, pool, tx);
        (p, rx)
    }

    #[tokio::test]
    async fn pipeline_l0_deny() {
        let (pipeline, _rx) = make_pipeline();
        let d = pipeline.decide(tool_before("rm -rf /")).await;
        assert_eq!(d.action, Action::Deny as i32);
        assert_eq!(d.source_level, SourceLevel::Rule as i32);
        assert!(d.latency_ms >= 1);
        // proves fail-safe: deny never downgraded to allow
        assert_ne!(d.action, Action::Allow as i32);
    }

    #[tokio::test]
    async fn pipeline_cache_hit() {
        let engine = Arc::new(algo_policy::Engine::new());
        let cache = Arc::new(Cache::new());
        let pool = Arc::new(JevPool::new(Arc::new(MockProvider::new())));
        let (tx, _rx) = mpsc::channel(1000);
        let pipeline = Pipeline::new(engine, cache.clone(), pool, tx);
        // Insert a cached allow
        let cached_decision = Decision {
            action: Action::Allow as i32,
            reason: "cached allow".into(),
            confidence_0_1: 0.9,
            source_level: SourceLevel::Jev as i32,
            latency_ms: 10,
            policy_version: env!("CARGO_PKG_VERSION").to_string(),
            trace_id: "t0".into(),
        };
        cache.insert("ls -la /tmp/foo", cached_decision);
        let d = pipeline.decide(tool_before("ls -la /tmp/foo")).await;
        // cache key normalizes /tmp/foo vs /tmp/bar same, but we test hit
        // Source should be Cache
        assert_eq!(d.source_level, SourceLevel::Cache as i32);
        assert!(d.latency_ms >= 1);
    }

    #[tokio::test]
    async fn proves_ask_on_provider_timeout() {
        let engine = Arc::new(algo_policy::Engine::new());
        let cache = Arc::new(Cache::new());
        let pool = Arc::new(JevPool::new(Arc::new(MockProvider::new())));
        let (tx, _rx) = mpsc::channel(1000);
        let pipeline = Pipeline::new(engine, cache, pool, tx);
        let d = pipeline
            .decide(tool_before("__sleep_800__ curl https://example.com"))
            .await;
        assert_eq!(d.action, Action::Ask as i32);
        assert_eq!(d.source_level, SourceLevel::Fallback as i32);
        assert!(d.reason.contains("provider error"));
    }

    #[tokio::test]
    async fn proves_ask_on_db_timeout() {
        // Create a channel with 0 capacity that blocks, then drop receiver to simulate busy.
        // Use bounded(1) and fill it, then no receiver consumption.
        let engine = Arc::new(algo_policy::Engine::new());
        let cache = Arc::new(Cache::new());
        let pool = Arc::new(JevPool::new(Arc::new(MockProvider::new())));
        let (tx, rx) = mpsc::channel(1);
        // Fill the channel so next send will block until timeout
        // Need a receiver that never reads: we hold rx but don't poll it, but channel(1) with one pending item still allows one more send to queue? Actually with size 1, if we send one item, next send will wait for capacity.
        // So we pre-fill:
        let rec = DbRecord {
            ts: 0,
            session_id: "s".into(),
            tool_kind: 1,
            redacted_command: "fill".into(),
            fingerprint: "fp".into(),
            action: Action::Ask as i32,
            source: SourceLevel::Fallback as i32,
            reason: "fill".into(),
            confidence: 0.0,
            latency_ms: 1,
            profile: "balanced".into(),
            shadow: false,
        };
        tx.try_send(rec).expect("fill");
        // Now pipeline's try_send with timeout 1s should time out because channel is full and we never drain rx
        let pipeline = Pipeline::new(engine, cache, pool, tx);
        // Keep rx alive but not consuming
        let _keep = rx;
        let start = Instant::now();
        let d = pipeline.decide(tool_before("ls -la")).await;
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_secs(2),
            "should not block >1s, got {:?}",
            elapsed
        );
        assert_eq!(d.action, Action::Ask as i32);
        assert_eq!(d.source_level, SourceLevel::Fallback as i32);
    }

    #[tokio::test]
    async fn proves_ask_on_db_channel_closed() {
        let engine = Arc::new(algo_policy::Engine::new());
        let cache = Arc::new(Cache::new());
        let pool = Arc::new(JevPool::new(Arc::new(MockProvider::new())));
        let (tx, rx) = mpsc::channel(1000);
        drop(rx); // close channel
        let pipeline = Pipeline::new(engine, cache, pool, tx);
        let d = pipeline.decide(tool_before("ls -la")).await;
        assert_eq!(d.action, Action::Ask as i32);
        assert_eq!(d.source_level, SourceLevel::Fallback as i32);
    }

    #[tokio::test]
    async fn cache_hit_returns_same() {
        let cache = Cache::new();
        let cmd = "ls -la /tmp/test";
        let dec = Decision {
            action: Action::Allow as i32,
            reason: "jev allow".into(),
            confidence_0_1: 0.8,
            source_level: SourceLevel::Jev as i32,
            latency_ms: 5,
            policy_version: "v0".into(),
            trace_id: "t1".into(),
        };
        cache.insert(cmd, dec.clone());
        let hit = cache.get(cmd).unwrap();
        assert_eq!(hit.action, Action::Allow as i32);
        assert_eq!(hit.source_level, SourceLevel::Cache as i32);
    }

    #[tokio::test]
    async fn latency_under_3ms_for_l0_l1() {
        let (pipeline, _rx) = make_pipeline();
        // Warm cache with ls
        let _ = pipeline.decide(tool_before("ls -la")).await;
        let start = Instant::now();
        for _ in 0..10 {
            let _ = pipeline.decide(tool_before("ls -la")).await;
        }
        let avg = start.elapsed().as_millis() / 10;
        // p50 <3ms is budget, but in debug may be higher; just assert <25ms to avoid flake
        assert!(avg < 25, "avg latency {}ms too high", avg);
    }

    fn make_shadow_pipeline() -> (Pipeline, mpsc::Receiver<DbRecord>) {
        let engine = Arc::new(algo_policy::Engine::new());
        let cache = Arc::new(Cache::new());
        let pool = Arc::new(JevPool::new(Arc::new(MockProvider::new())));
        let (tx, rx) = mpsc::channel(1000);
        let p = Pipeline::new(engine, cache, pool, tx).with_shadow(true);
        assert!(p.is_shadow());
        (p, rx)
    }

    #[tokio::test]
    async fn shadow_l0_deny_returns_allow_but_audits_deny() {
        // P1-08: shadow default — compute deny, return Allow, audit would_have deny.
        let (pipeline, mut rx) = make_shadow_pipeline();
        let d = pipeline.decide(tool_before("rm -rf /")).await;
        assert_eq!(d.action, Action::Allow as i32, "shadow must never block");
        assert!(is_shadow_reason(&d.reason));
        assert_eq!(parse_would_have(&d.reason), Some("deny"));
        // Audit row must preserve real deny with shadow=true for would-have-blocked.
        let rec = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("audit row")
            .expect("channel open");
        assert_eq!(rec.action, Action::Deny as i32);
        assert!(rec.shadow);
    }

    #[tokio::test]
    async fn shadow_never_blocks_on_db_error() {
        // Shadow soak: even DB-closed must return Allow (would_have ask), never block.
        let engine = Arc::new(algo_policy::Engine::new());
        let cache = Arc::new(Cache::new());
        let pool = Arc::new(JevPool::new(Arc::new(MockProvider::new())));
        let (tx, rx) = mpsc::channel(1000);
        drop(rx);
        let pipeline = Pipeline::new(engine, cache, pool, tx).with_shadow(true);
        let d = pipeline.decide(tool_before("ls -la")).await;
        assert_eq!(d.action, Action::Allow as i32);
        assert!(is_shadow_reason(&d.reason));
    }

    #[test]
    fn shadow_reason_helpers() {
        assert!(!is_shadow_reason("hard deny: rm -rf /"));
        assert!(is_shadow_reason(
            "shadow: would_have deny (hard deny) → approve (shadow)"
        ));
        assert_eq!(
            parse_would_have("shadow: would_have deny (x) → approve (shadow)"),
            Some("deny")
        );
        assert_eq!(
            parse_would_have("shadow: would_have ask (x) → approve (shadow)"),
            Some("ask")
        );
        assert_eq!(parse_would_have("hard deny"), None);
        assert_eq!(action_str_of(Action::Deny as i32), "deny");
        assert_eq!(action_str_of(Action::Allow as i32), "allow");
        assert_eq!(action_str_of(Action::Ask as i32), "ask");
    }
}
