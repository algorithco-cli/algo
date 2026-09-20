use algo_provider::{DecisionProvider, MockProvider, ProviderError, TypedAnswers};
use algo_types::ToolBefore;
use std::sync::Arc;
use std::time::Duration;

/// Dummy warm pool that holds a MockProvider.
/// In production this would warm an h2 pool to Jev; here it is a no-op but keeps the shape.
pub struct JevPool {
    provider: Arc<MockProvider>,
}

impl JevPool {
    pub fn new(provider: Arc<MockProvider>) -> Self {
        Self { provider }
    }

    /// Warm the Jev h2 pool on start. No-op for mock but keeps the call site.
    pub fn warm(&self) {
        // In real Jev provider, this would establish h2 connections.
        // Mock: no-op.
    }

    /// Judge with 700ms timeout via `tokio::time::timeout`.
    /// Returns `ProviderError::Timeout` if the provider does not respond in time.
    pub async fn judge(&self, event: &ToolBefore) -> Result<TypedAnswers, ProviderError> {
        let provider = self.provider.clone();
        let evt = event.clone();
        let fut = async move {
            // Simulate slow provider if payload contains marker __sleep_800__ .
            // This allows `proves_ask_on_provider_timeout` without a custom provider.
            if evt.redacted_payload.contains("__sleep_800__") {
                tokio::time::sleep(Duration::from_millis(800)).await;
            }
            // Also support generic SLEEP marker for integration tests: e.g. __sleep_1500__
            if evt.redacted_payload.contains("__sleep_1500__") {
                tokio::time::sleep(Duration::from_millis(1500)).await;
            }
            provider.judge(&evt, &[])
        };
        match tokio::time::timeout(Duration::from_millis(700), fut).await {
            Ok(r) => r,
            Err(_) => Err(ProviderError::Timeout),
        }
    }

    /// Expose provider for tests that want direct access.
    pub fn provider(&self) -> &Arc<MockProvider> {
        &self.provider
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[tokio::test]
    async fn warm_is_noop() {
        let pool = JevPool::new(Arc::new(MockProvider::new()));
        pool.warm(); // should not panic
    }

    #[tokio::test]
    async fn judge_ok_within_timeout() {
        let pool = JevPool::new(Arc::new(MockProvider::new()));
        let res = pool.judge(&tool_before("ls -la")).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn proves_ask_on_provider_timeout() {
        let pool = JevPool::new(Arc::new(MockProvider::new()));
        let res = pool.judge(&tool_before("__sleep_800__")).await;
        assert_eq!(res.unwrap_err(), ProviderError::Timeout);
    }
}
