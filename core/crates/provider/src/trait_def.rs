use algo_types::Decision;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub enum TypedQuestion {
    Bool { id: String, prompt: String },
    Choice { id: String, prompt: String, options: Vec<String> },
    Score { id: String, prompt: String, levels: Vec<String> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedAnswer {
    Bool { id: String, value: bool, confidence: f64 },
    Choice { id: String, choice: String, confidence: f64 },
    Score { id: String, score: f64, confidence: f64 },
}

pub type TypedAnswers = Vec<TypedAnswer>;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum ProviderError {
    #[error("timeout")]
    Timeout,
    #[error("auth error")]
    Auth,
    #[error("network error: {0}")]
    Net(String),
    #[error("parse error: {0}")]
    Parse(String),
}

/// Zero business logic — transport + serde only (per spec).
pub trait DecisionProvider: Send + Sync {
    fn judge(
        &self,
        event: &algo_types::ToolBefore,
        qs: &[TypedQuestion],
    ) -> Result<TypedAnswers, ProviderError>;
}

/// Helper to map ProviderError to fail-safe ASK Decision (never ALLOW).
pub fn map_to_ask(err: ProviderError, trace_id: &str) -> Decision {
    use algo_types::{Action, SourceLevel};
    Decision {
        action: Action::Ask as i32,
        reason: format!("provider error → ask: {}", err),
        confidence_0_1: 0.0,
        source_level: SourceLevel::Fallback as i32,
        latency_ms: 0,
        policy_version: env!("CARGO_PKG_VERSION").to_string(),
        trace_id: trace_id.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proves_ask_on_timeout() {
        let err = ProviderError::Timeout;
        let d = map_to_ask(err, "trace-1");
        assert_eq!(d.action, algo_types::Action::Ask as i32);
        assert_eq!(d.source_level, algo_types::SourceLevel::Fallback as i32);
    }

    #[test]
    fn proves_ask_on_parse() {
        let err = ProviderError::Parse("bad json".into());
        let d = map_to_ask(err, "t");
        assert_eq!(d.action, algo_types::Action::Ask as i32);
    }
}
