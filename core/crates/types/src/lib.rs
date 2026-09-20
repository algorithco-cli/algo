//! Canonical types for `algorithco_guard.v0` — generated at build, never hand-edit.
//!
//! Pin: proto tag `v0.0.1-alpha` (see `proto/README.md` + `proto/VERSIONING.md`).
//! Consumers generate at build from the tagged release; this crate re-exports.

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/algorithco_guard.v0.rs"));
}

pub use prost_types::Timestamp;
pub use proto::{
    Action, AgentIdentity, AgentQuestion, AgentStop, Decision, PrivacyMode, QuestionKind,
    SourceLevel, ToolAfter, ToolBefore, ToolKind,
};

// Re-export dataset types (they import events/decision, so they live in the same proto package).
pub use proto::{DatasetRecord, RedactionCert};

/// Helpers for `Decision` — fail-safe and deny checks (AGENTS.md:6 + `decision.proto:3` invariant).
pub trait DecisionExt {
    fn is_deny(&self) -> bool;
    fn is_allow(&self) -> bool;
    fn is_ask(&self) -> bool;
    /// Map any error/timeout/unknown to `ASK` — never `ALLOW` (fail-safe).
    fn to_ask_on_error(self) -> Self;
}

impl DecisionExt for Decision {
    fn is_deny(&self) -> bool {
        self.action == Action::Deny as i32
    }
    fn is_allow(&self) -> bool {
        self.action == Action::Allow as i32
    }
    fn is_ask(&self) -> bool {
        // Unspecified (0) is also ASK per decision.proto:42.
        !self.is_allow() && !self.is_deny()
    }

    fn to_ask_on_error(self) -> Self {
        Self {
            action: Action::Ask as i32,
            reason: if self.reason.is_empty() {
                "error/timeout/parse-fail → ask (fail-safe)".to_string()
            } else {
                self.reason
            },
            confidence_0_1: 0.0,
            source_level: SourceLevel::Fallback as i32,
            latency_ms: self.latency_ms,
            policy_version: self.policy_version,
            trace_id: self.trace_id,
        }
    }
}

/// Helper to create a fail-safe `ASK` decision from any error.
pub fn ask_on_error(reason: impl Into<String>, trace_id: impl Into<String>) -> Decision {
    Decision {
        action: Action::Ask as i32,
        reason: reason.into(),
        confidence_0_1: 0.0,
        source_level: SourceLevel::Fallback as i32,
        latency_ms: 0,
        policy_version: env!("CARGO_PKG_VERSION").to_string(),
        trace_id: trace_id.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_deny_helpers() {
        let deny = Decision {
            action: Action::Deny as i32,
            reason: "hard deny".into(),
            confidence_0_1: 0.92,
            source_level: SourceLevel::Rule as i32,
            latency_ms: 1,
            policy_version: "test".into(),
            trace_id: "t1".into(),
        };
        assert!(deny.is_deny());
        assert!(!deny.is_allow());
        assert!(!deny.is_ask());
    }

    #[test]
    fn unspecified_maps_to_ask() {
        let unspecified = Decision {
            action: Action::Unspecified as i32,
            ..Default::default()
        };
        assert!(unspecified.is_ask());
        assert!(!unspecified.is_allow());
        assert!(!unspecified.is_deny());
    }

    #[test]
    fn to_ask_on_error_never_allow() {
        let d = Decision {
            action: Action::Allow as i32,
            reason: "allow".into(),
            confidence_0_1: 0.9,
            source_level: SourceLevel::Jev as i32,
            latency_ms: 10,
            policy_version: "v".into(),
            trace_id: "t".into(),
        };
        let ask = d.to_ask_on_error();
        assert!(ask.is_ask());
        assert_eq!(ask.source_level, SourceLevel::Fallback as i32);
    }

    #[test]
    fn proves_ask_on_error() {
        let ask = ask_on_error("timeout", "trace-1");
        assert!(ask.is_ask());
        assert_eq!(ask.source_level, SourceLevel::Fallback as i32);
    }

    #[test]
    fn round_trip_event_json() {
        // Simple JSON round-trip via prost JSON? We just check that ToolBefore can be constructed and debug-printed.
        let ev = ToolBefore {
            event_id: "evt-1".into(),
            timestamp: Some(Timestamp {
                seconds: 0,
                nanos: 0,
            }),
            agent: Some(AgentIdentity {
                agent_type: Some("claude-code".into()),
                agent_version: Some("2.0.0".into()),
                session_id: "sess-1".into(),
                working_dir: "/tmp".into(),
            }),
            tool_kind: ToolKind::Shell as i32,
            redacted_payload: "ls -la".into(),
            privacy_mode: PrivacyMode::Redacted as i32,
            shell_argv: vec!["ls".into(), "-la".into()],
            file_path: None,
        };
        assert_eq!(ev.tool_kind, ToolKind::Shell as i32);
        assert!(ev.shell_argv.contains(&"ls".to_string()));
    }
}
