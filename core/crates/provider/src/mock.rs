use crate::trait_def::{DecisionProvider, ProviderError, TypedAnswer, TypedAnswers, TypedQuestion};
use algo_types::ToolBefore;
use std::collections::HashMap;

/// Deterministic mock from a fixture map — for daemon/adapter tests + offline eval harness.
/// No network, no Jev, no secrets in logs.
pub struct MockProvider {
    // Map from redacted_payload substring → answer
    fixtures: HashMap<String, TypedAnswers>,
    default: TypedAnswers,
}

impl MockProvider {
    pub fn new() -> Self {
        Self {
            fixtures: HashMap::new(),
            default: vec![TypedAnswer::Choice {
                id: "decision".to_string(),
                choice: "ask".to_string(),
                confidence: 0.5,
            }],
        }
    }

    pub fn with_fixture(mut self, substr: impl Into<String>, answers: TypedAnswers) -> Self {
        self.fixtures.insert(substr.into(), answers);
        self
    }
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DecisionProvider for MockProvider {
    fn judge(
        &self,
        event: &ToolBefore,
        _qs: &[TypedQuestion],
    ) -> Result<TypedAnswers, ProviderError> {
        let payload = &event.redacted_payload;
        for (k, v) in &self.fixtures {
            if payload.contains(k) {
                return Ok(v.clone());
            }
        }
        Ok(self.default.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn mock_matches_fixture() {
        let mock = MockProvider::new().with_fixture(
            "rm -rf /",
            vec![TypedAnswer::Choice {
                id: "decision".into(),
                choice: "deny".into(),
                confidence: 0.99,
            }],
        );
        let ans = mock.judge(&tool_before("rm -rf /"), &[]).unwrap();
        assert_eq!(
            ans[0],
            TypedAnswer::Choice {
                id: "decision".into(),
                choice: "deny".into(),
                confidence: 0.99
            }
        );
    }

    #[test]
    fn mock_default_is_ask() {
        let mock = MockProvider::new();
        let ans = mock.judge(&tool_before("ls -la"), &[]).unwrap();
        assert_eq!(
            ans[0],
            TypedAnswer::Choice {
                id: "decision".into(),
                choice: "ask".into(),
                confidence: 0.5
            }
        );
    }

    #[test]
    fn mock_never_returns_allow_on_error() {
        // Mock never errors, but the trait's error mapping is tested in trait_def
        let mock = MockProvider::new();
        assert!(mock.judge(&tool_before("anything"), &[]).is_ok());
    }
}
