//! Verifier (P2-06): on `AgentStop`, check (a) tests run? (b) diff matches task?
//! Jev Choice/Score batched, timeout -> ask never allow. Quiet unless fail.
//! Eval: fake-done vs real-done; false-pass ≤ threshold.

use algo_provider::{DecisionProvider, TypedAnswer, TypedQuestion};
use algo_types::{AgentStop, VerifierVerdict};
use std::sync::Arc;
use std::time::Duration;

/// Verifier holds a provider (Mock default, Jev behind feature). Construction
/// does not send; verify runs a batched Jev call with 700ms budget per spec.
pub struct Verifier {
    provider: Arc<dyn DecisionProvider>,
    timeout: Duration,
}

impl Verifier {
    pub fn new(provider: Arc<dyn DecisionProvider>) -> Self {
        Self {
            provider,
            timeout: Duration::from_millis(700),
        }
    }

    /// Verify an AgentStop. Fail-safe: any missing field, timeout, parse, or
    /// ambiguous provider answer maps to `ASK` (never PASS on error).
    pub async fn verify(&self, stop: &AgentStop) -> VerifierVerdict {
        // Basic presence checks: if the stop carries no task/diff, we cannot
        // claim PASS — ask. This is not a model call, just deterministic guard.
        if stop.task_ref.is_none() || stop.diff_ref.is_none() {
            return VerifierVerdict::Ask;
        }
        // Build batched questions (Choice + Score per spec). Provider trait is
        // sync (blocking HTTP), so run on blocking thread with timeout.
        let task = stop.task_ref.clone().unwrap_or_default();
        let diff = stop.diff_ref.clone().unwrap_or_default();
        let qs = vec![
            TypedQuestion::Choice {
                id: "tests_run".into(),
                prompt: format!("Did tests run for task '{}'? Answer pass/fail/ask.", task),
                options: vec!["pass".into(), "fail".into(), "ask".into()],
            },
            TypedQuestion::Score {
                id: "diff_match".into(),
                prompt: format!("Does diff '{}' match task '{}'? Score 0..1.", diff, task),
                levels: vec!["no".into(), "partial".into(), "yes".into()],
            },
        ];
        let provider = Arc::clone(&self.provider);
        let stop_clone = stop.clone();
        let inner = tokio::task::spawn_blocking(move || {
            provider.judge(&stop_clone_to_tool(&stop_clone), &qs)
        });
        let res = tokio::time::timeout(self.timeout, inner).await;
        match res {
            Ok(Ok(Ok(answers))) => map_answers(&answers),
            _ => VerifierVerdict::Ask,
        }
    }
}

fn stop_clone_to_tool(stop: &AgentStop) -> algo_types::ToolBefore {
    // Privacy-minimal ToolBefore for provider: no raw diff, just redacted refs.
    algo_types::ToolBefore {
        event_id: stop.event_id.clone(),
        timestamp: stop.timestamp,
        agent: stop.agent.clone(),
        tool_kind: algo_types::ToolKind::Other as i32,
        redacted_payload: format!(
            "agent_stop task={} diff={} tests={}",
            stop.task_ref.clone().unwrap_or_default(),
            stop.diff_ref.clone().unwrap_or_default(),
            stop.test_report_ref.clone().unwrap_or_default()
        ),
        privacy_mode: algo_types::PrivacyMode::Redacted as i32,
        shell_argv: vec![],
        file_path: None,
    }
}

fn map_answers(answers: &[TypedAnswer]) -> VerifierVerdict {
    // Expect tests_run choice + diff_match score. Any missing/unknown -> ASK.
    let mut tests_pass = false;
    let mut score_ok = false;
    for a in answers {
        match a {
            TypedAnswer::Choice { id, choice, .. } if id == "tests_run" => {
                tests_pass = choice == "pass";
                if choice != "pass" && choice != "fail" && choice != "ask" {
                    return VerifierVerdict::Ask;
                }
            }
            TypedAnswer::Score { id, score, .. } if id == "diff_match" => {
                score_ok = *score >= 0.6;
                if !score.is_finite() {
                    return VerifierVerdict::Ask;
                }
            }
            _ => {}
        }
    }
    if tests_pass && score_ok {
        VerifierVerdict::Pass
    } else if !tests_pass {
        VerifierVerdict::FailBackToWork
    } else {
        VerifierVerdict::Ask
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use algo_provider::{MockProvider, TypedAnswer};
    use algo_types::{AgentIdentity, VerifierVerdict};

    fn stop_with(task: Option<&str>, diff: Option<&str>) -> AgentStop {
        AgentStop {
            event_id: "evt-stop-1".into(),
            timestamp: None,
            agent: Some(AgentIdentity {
                agent_type: Some("claude-code".into()),
                agent_version: Some("1.0".into()),
                session_id: "sess-1".into(),
                working_dir: "/tmp".into(),
            }),
            reason: "done".into(),
            task_ref: task.map(|s| s.to_string()),
            diff_ref: diff.map(|s| s.to_string()),
            test_report_ref: Some("tests: pass".into()),
        }
    }

    #[tokio::test]
    async fn missing_fields_maps_to_ask() {
        let v = Verifier::new(Arc::new(MockProvider::new()));
        let s = stop_with(None, Some("diff"));
        assert_eq!(v.verify(&s).await, VerifierVerdict::Ask);
        let s2 = stop_with(Some("task"), None);
        assert_eq!(v.verify(&s2).await, VerifierVerdict::Ask);
    }

    #[tokio::test]
    async fn pass_when_mock_says_pass_and_high_score() {
        let provider = MockProvider::new().with_fixture(
            "agent_stop",
            vec![
                TypedAnswer::Choice {
                    id: "tests_run".into(),
                    choice: "pass".into(),
                    confidence: 0.9,
                },
                TypedAnswer::Score {
                    id: "diff_match".into(),
                    score: 0.8,
                    confidence: 0.7,
                },
            ],
        );
        // Our Mock matches substring in redacted_payload; stop payload contains "agent_stop"
        // but our mapping uses task/diff literal, so we use a fixture that matches "task1"
        let provider2 = MockProvider::new().with_fixture(
            "task1",
            vec![
                TypedAnswer::Choice {
                    id: "tests_run".into(),
                    choice: "pass".into(),
                    confidence: 0.9,
                },
                TypedAnswer::Score {
                    id: "diff_match".into(),
                    score: 0.8,
                    confidence: 0.7,
                },
            ],
        );
        let v = Verifier::new(Arc::new(provider2));
        let s = stop_with(Some("task1"), Some("diff1"));
        let verdict = v.verify(&s).await;
        assert_eq!(verdict, VerifierVerdict::Pass);
        let _ = provider; // keep first provider unused warning quiet
    }

    #[tokio::test]
    async fn fail_back_when_tests_fail() {
        let provider = MockProvider::new().with_fixture(
            "task1",
            vec![
                TypedAnswer::Choice {
                    id: "tests_run".into(),
                    choice: "fail".into(),
                    confidence: 0.9,
                },
                TypedAnswer::Score {
                    id: "diff_match".into(),
                    score: 0.9,
                    confidence: 0.7,
                },
            ],
        );
        let v = Verifier::new(Arc::new(provider));
        let s = stop_with(Some("task1"), Some("diff1"));
        assert_eq!(v.verify(&s).await, VerifierVerdict::FailBackToWork);
    }

    #[tokio::test]
    async fn proves_ask_on_provider_timeout() {
        use algo_provider::{DecisionProvider, ProviderError, TypedAnswers, TypedQuestion};
        use algo_types::ToolBefore;
        struct Slow;
        impl DecisionProvider for Slow {
            fn judge(
                &self,
                _: &ToolBefore,
                _: &[TypedQuestion],
            ) -> Result<TypedAnswers, ProviderError> {
                std::thread::sleep(Duration::from_millis(2000));
                Ok(vec![])
            }
        }
        let v = Verifier::new(Arc::new(Slow));
        let s = stop_with(Some("t"), Some("d"));
        assert_eq!(v.verify(&s).await, VerifierVerdict::Ask);
    }
}
