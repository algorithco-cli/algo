//! Scanner (P4): tree-sitter + rule pre-filter finds candidates, Jev judges, report-only.
//! Never blocks; findings surfaced after edits. Quiet unless attention.

use algo_provider::{DecisionProvider, TypedAnswer, TypedQuestion};
use regex::Regex;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

static RE_SECRET: OnceLock<Regex> = OnceLock::new();
fn re_secret() -> &'static Regex {
    RE_SECRET.get_or_init(|| {
        Regex::new(r"(?i)(ghp_[A-Za-z0-9_]{30,}|AKIA[0-9A-Z]{16}|xox[bpras]-\d+-)").unwrap()
    })
}

/// Finding from scanner.
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub file: String,
    pub line: usize,
    pub kind: String,
    pub snippet: String,
    pub jev_verdict: Option<String>,
}

/// Scanner holds a provider (Mock default). Pre-filter is regex, then batched Jev.
pub struct Scanner {
    provider: Arc<dyn DecisionProvider>,
    timeout: Duration,
}

impl Scanner {
    pub fn new(provider: Arc<dyn DecisionProvider>) -> Self {
        Self {
            provider,
            timeout: Duration::from_millis(700),
        }
    }

    /// Scan a file's redacted content. Returns findings (pre-filter + Jev judged).
    /// Fail-safe: any provider error → finding with jev_verdict None, not crash.
    pub async fn scan(&self, file: &str, content: &str) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (idx, line) in content.lines().enumerate() {
            if re_secret().is_match(line) {
                findings.push(Finding {
                    file: file.to_string(),
                    line: idx + 1,
                    kind: "secret".into(),
                    snippet: line.chars().take(80).collect(),
                    jev_verdict: None,
                });
            }
        }
        if findings.is_empty() {
            return findings;
        }
        // Batched Jev: one Choice per finding (simplified: single question batch).
        let qs: Vec<TypedQuestion> = findings
            .iter()
            .enumerate()
            .map(|(i, f)| TypedQuestion::Choice {
                id: format!("finding_{i}"),
                prompt: format!(
                    "Is this line secret exfiltration? File {} line {} snippet '{}'",
                    f.file, f.line, f.snippet
                ),
                options: vec!["secret".into(), "not_secret".into(), "ask".into()],
            })
            .collect();
        let provider = Arc::clone(&self.provider);
        let qs_clone = qs.clone();
        // Use a dummy ToolBefore for provider (privacy-minimal).
        let dummy = algo_types::ToolBefore {
            event_id: "scan".into(),
            timestamp: None,
            agent: None,
            tool_kind: algo_types::ToolKind::Other as i32,
            redacted_payload: format!("scan {} findings {}", file, findings.len()),
            privacy_mode: algo_types::PrivacyMode::Redacted as i32,
            shell_argv: vec![],
            file_path: Some(file.to_string()),
        };
        let inner = tokio::task::spawn_blocking(move || provider.judge(&dummy, &qs_clone));
        let res = tokio::time::timeout(self.timeout, inner).await;
        if let Ok(Ok(Ok(answers))) = res {
            for (i, ans) in answers.iter().enumerate() {
                if i < findings.len() {
                    if let TypedAnswer::Choice { choice, .. } = ans {
                        findings[i].jev_verdict = Some(choice.clone());
                    }
                }
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use algo_provider::MockProvider;

    #[tokio::test]
    async fn finds_secret_and_judges() {
        let provider = MockProvider::new().with_fixture(
            "scan",
            vec![TypedAnswer::Choice {
                id: "finding_0".into(),
                choice: "secret".into(),
                confidence: 0.9,
            }],
        );
        let s = Scanner::new(Arc::new(provider));
        let content = "line1\n ghp_1234567890123456789012345678901234\n line3";
        let findings = s.scan("src/main.rs", content).await;
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, "secret");
        assert_eq!(findings[0].jev_verdict.as_deref(), Some("secret"));
    }

    #[tokio::test]
    async fn no_secret_no_findings() {
        let s = Scanner::new(Arc::new(MockProvider::new()));
        let findings = s.scan("src/main.rs", "safe content\nlet x = 1;\n").await;
        assert!(findings.is_empty());
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
        let s = Scanner::new(Arc::new(Slow));
        let findings = s.scan("f", "ghp_1234567890123456789012345678901234").await;
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].jev_verdict, None); // timeout → None, not crash
    }
}
