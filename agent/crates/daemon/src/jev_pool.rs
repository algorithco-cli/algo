use algo_provider::{DecisionProvider, ProviderError, TypedAnswers, TypedQuestion};
use algo_types::ToolBefore;
use std::sync::Arc;
use std::time::Duration;

/// L3 provider pool: holds any `DecisionProvider` (Mock by default; the real
/// `JevProvider` with `--features jev`). Construction warms the pool — for the
/// real client that means building the pooled HTTP client; no request is sent.
///
/// The trait is synchronous (the real client uses blocking HTTP so `core` stays
/// runtime-free), therefore `judge` runs the provider on a blocking thread and
/// enforces the 700ms budget with `tokio::time::timeout`. Every outcome maps to
/// a typed error the pipeline turns into `ask` — never `allow`.
///
/// NOTE: the question set is currently empty (`&[]`). The Mock answers from
/// fixtures; the real client rejects empty batches (`Parse` → ask, fail-safe).
/// The pinned L3 question set (`questions-v0.1`, EVAL-6) lands here when tuned.
pub struct JevPool {
    provider: Arc<dyn DecisionProvider>,
}

impl JevPool {
    pub fn new(provider: Arc<dyn DecisionProvider>) -> Self {
        Self { provider }
    }

    /// Warm hook (kept call site stable). Real warming happens in
    /// `JevProvider::new` (pool construction); nothing is sent here.
    pub fn warm(&self) {
        // No-op by design: no network on warm, no traffic without an explicit judge.
    }

    /// Judge with 700ms timeout. Slow/blocked providers (and the timeout itself)
    /// surface as `ProviderError::Timeout`. Questions ride along for the real
    /// client; callers pass the pinned set (currently `&[]` — see struct NOTE).
    pub async fn judge(
        &self,
        event: &ToolBefore,
        qs: &[TypedQuestion],
    ) -> Result<TypedAnswers, ProviderError> {
        let provider = Arc::clone(&self.provider);
        let evt = event.clone();
        let qs = qs.to_vec();
        let inner = tokio::task::spawn_blocking(move || provider.judge(&evt, &qs));
        match tokio::time::timeout(Duration::from_millis(700), inner).await {
            Ok(Ok(r)) => r,
            Ok(Err(join_err)) => Err(ProviderError::Net(format!("judge task: {join_err}"))),
            Err(_) => Err(ProviderError::Timeout),
        }
    }

    /// Expose provider for tests that want direct access.
    pub fn provider(&self) -> &Arc<dyn DecisionProvider> {
        &self.provider
    }
}

/// Slow synchronous test double (shared by pool + pipeline timeout tests).
/// Production code never sniffs payloads for test markers.
#[cfg(test)]
pub(crate) struct SlowMock {
    pub ms: u64,
}

#[cfg(test)]
impl DecisionProvider for SlowMock {
    fn judge(
        &self,
        _event: &ToolBefore,
        _qs: &[algo_provider::TypedQuestion],
    ) -> Result<TypedAnswers, ProviderError> {
        std::thread::sleep(Duration::from_millis(self.ms));
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use algo_provider::MockProvider;
    use algo_types::{AgentIdentity, PrivacyMode, ToolKind};

    fn tool_before(payload: &str) -> ToolBefore {
        ToolBefore {
            event_id: "evt-test".into(),
            timestamp: None,
            agent: Some(AgentIdentity {
                agent_type: Some("claude-code".into()),
                agent_version: Some("test".into()),
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

    /// Shared slow double lives at module scope (`super::SlowMock`) so the
    /// pipeline timeout test can reuse it — no payload-marker sniffing anywhere.
    #[tokio::test]
    async fn warm_is_noop() {
        let pool = JevPool::new(Arc::new(MockProvider::new()));
        pool.warm(); // should not panic, and must not send anything
    }

    #[tokio::test]
    async fn judge_ok_within_timeout() {
        let pool = JevPool::new(Arc::new(MockProvider::new()));
        let res = pool.judge(&tool_before("ls -la"), &[]).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn proves_ask_on_provider_timeout() {
        let pool = JevPool::new(Arc::new(SlowMock { ms: 1500 }));
        let res = pool.judge(&tool_before("ls -la"), &[]).await;
        assert_eq!(res.unwrap_err(), ProviderError::Timeout);
    }

    #[tokio::test]
    async fn pool_accepts_any_provider_impl() {
        // dyn dispatch: Mock and SlowMock both slot into the same pool.
        let pool = JevPool::new(Arc::new(SlowMock { ms: 1 }));
        assert!(pool.judge(&tool_before("ls"), &[]).await.is_ok());
        assert!(pool.provider().judge(&tool_before("ls"), &[]).is_ok());
    }

    /// Real-client plumbing (feature-gated): the stub server accepts then drops
    /// the connection, so the client deterministically surfaces `Net` — no
    /// dependence on refused-connection timing (which hangs ~2s on some hosts).
    ///
    /// Runtime hygiene (see `JevProvider::new` docs): the blocking client builds
    /// and drops a helper runtime, so construction AND drop happen on blocking
    /// threads — never in this async context.
    #[cfg(feature = "jev")]
    #[tokio::test]
    async fn real_client_plumbs_through_pool() {
        use algo_provider::JevProvider;
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub");
        let url = format!("http://{}", listener.local_addr().expect("stub addr"));
        std::thread::spawn(move || {
            // Accept one connection and drop it: client sees EOF → Net.
            let _ = listener.accept();
        });
        let real = tokio::task::spawn_blocking(move || {
            JevProvider::new("k".into(), url, "m".into()).expect("build")
        })
        .await
        .expect("spawn");
        let pool = JevPool::new(Arc::new(real));
        // A real question (not empty) so the client reaches the network path.
        let qs = vec![algo_provider::TypedQuestion::Choice {
            id: "decision".into(),
            prompt: "Choose.".into(),
            options: vec!["allow".into(), "ask".into(), "deny".into()],
        }];
        let res = pool.judge(&tool_before("ls"), &qs).await;
        assert!(
            matches!(res, Err(ProviderError::Net(_))),
            "dropped connection must surface as Net, got {res:?}"
        );
        tokio::task::spawn_blocking(move || drop(pool))
            .await
            .expect("drop pool");
    }
}
