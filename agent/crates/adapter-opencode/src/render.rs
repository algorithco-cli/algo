//! Render `Decision` → OpenCode permission JSON.
//!
//! [VERIFY-OPEN P4-03] OpenCode mapping is Claude-derived until the spike verifies it
//! against current OpenCode docs (plugin/config system, blockable events).
//! Unknown schema version → always `ask` + `reason:unsupported_schema` (fail-safe).
//! All outputs are version-gated with `adapter_version`. If interception cannot
//! block, degrade to observe/advise via ADR, never guess-and-allow.
//! The mapping below is behind the `is_supported_schema` gate so a future schema bump fails safe.

use algo_types::{Action, Decision};
use serde_json::Value;

/// Render a `Decision` to Claude hook JSON with the given `adapter_version`.
///
/// - `Allow` → `{"decision":"approve","adapter_version": version}`
/// - `Deny`  → `{"decision":"block","reason": reason,"adapter_version": version}`
/// - `Ask` (and `Unspecified`) → `{"decision":"ask","reason": reason,"adapter_version": version}`
///
/// `reason` is `decision.reason` when non-empty, otherwise a fail-safe default (`ask (fail-safe)`
/// for ask, `deny` for deny). `adapter_version` should be `crate::ADAPTER_VERSION` in call sites.
pub fn render(decision: &Decision, adapter_version: &str) -> Value {
    if decision.action == Action::Allow as i32 {
        serde_json::json!({
            "decision": "approve",
            "adapter_version": adapter_version
        })
    } else if decision.action == Action::Deny as i32 {
        let reason = if decision.reason.is_empty() {
            "deny".to_string()
        } else {
            decision.reason.clone()
        };
        serde_json::json!({
            "decision": "block",
            "reason": reason,
            "adapter_version": adapter_version
        })
    } else {
        // Ask and Unspecified (fail-safe) both map to ask.
        let reason = if decision.reason.is_empty() {
            "ask (fail-safe)".to_string()
        } else {
            decision.reason.clone()
        };
        serde_json::json!({
            "decision": "ask",
            "reason": reason,
            "adapter_version": adapter_version
        })
    }
}

/// Whether a `schema_version` (from `hook_event_name` or `version` field) is known/supported.
///
/// Supported versions are those verified against the current Claude docs. Any other
/// `Some(version)` is treated as unknown and forces `ask` with `unsupported_schema`.
///
/// Currently supported: `"1"`, `"1.0"`, `"v1"`, `"v0"`, `"PreToolUse"`, `"PostToolUse"`,
/// `"0"`, `"0.1.0"`, `"1.0.0"`. Empty/whitespace is treated as "no version" (supported for
/// backwards compat – caller should pass `None` instead).
///
/// [VERIFY 2026-09-20] Re-check on each Claude release bump; update this list and add a
/// `docs/adr` note if the hook schema adds a new version field or renames `hook_event_name`.
fn is_supported_schema(version: &str) -> bool {
    let v = version.trim();
    if v.is_empty() {
        return true; // treat empty as no version → backward compat
    }
    matches!(
        v,
        "1" | "1.0" | "v1" | "v0" | "PreToolUse" | "PostToolUse" | "0" | "0.1.0" | "1.0.0"
    )
}

/// Version-gated render: if `schema_version` is `Some` and not supported, always return
/// `ask` with `reason:unsupported_schema`, regardless of the underlying `Decision`.
///
/// This enforces fail-safe on schema bumps: an adapter that does not understand a new
/// Claude hook schema will not silently `allow`.
pub fn render_with_schema(
    decision: &Decision,
    adapter_version: &str,
    schema_version: Option<&str>,
) -> Value {
    if let Some(v) = schema_version {
        let trimmed = v.trim();
        if !trimmed.is_empty() && !is_supported_schema(trimmed) {
            return serde_json::json!({
                "decision": "ask",
                "reason": "unsupported_schema",
                "adapter_version": adapter_version
            });
        }
    }
    render(decision, adapter_version)
}

/// Spec shorthand `render_with_schema(decision, schema_version)` – uses `crate::ADAPTER_VERSION`.
///
/// This wrapper exists so the spec's 2-arg form (`render_with_schema(decision, schema_version: Option<&str>)`)
/// is directly available while the 3-arg form above retains explicit version-gating for callers that
/// inject a version (tests, daemon). Both enforce the same `unsupported_schema` fail-safe.
pub fn render_with_schema_default_version(
    decision: &Decision,
    schema_version: Option<&str>,
) -> Value {
    render_with_schema(decision, crate::ADAPTER_VERSION, schema_version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use algo_types::{Action, SourceLevel};

    fn decision(action: Action, reason: &str) -> Decision {
        Decision {
            action: action as i32,
            reason: reason.to_string(),
            confidence_0_1: 0.9,
            source_level: SourceLevel::Rule as i32,
            latency_ms: 5,
            policy_version: "test".into(),
            trace_id: "t-1".into(),
        }
    }

    #[test]
    fn allow_renders_approve() {
        let d = decision(Action::Allow, "");
        let v = render(&d, "0.1.0");
        assert_eq!(v["decision"], "approve");
        assert_eq!(v["adapter_version"], "0.1.0");
        // allow must not include reason in current spec (or empty)
        assert!(v.get("reason").is_none(), "allow should not have reason");
    }

    #[test]
    fn deny_renders_block_with_reason() {
        let d = decision(Action::Deny, "hard deny: rm -rf /");
        let v = render(&d, "0.1.0");
        assert_eq!(v["decision"], "block");
        assert_eq!(v["reason"], "hard deny: rm -rf /");
        assert_eq!(v["adapter_version"], "0.1.0");
    }

    #[test]
    fn deny_empty_reason_defaults() {
        let d = decision(Action::Deny, "");
        let v = render(&d, "0.1.0");
        assert_eq!(v["decision"], "block");
        assert_eq!(v["reason"], "deny");
    }

    #[test]
    fn ask_renders_ask_with_reason() {
        let d = decision(Action::Ask, "need human: ambiguous");
        let v = render(&d, "0.1.0");
        assert_eq!(v["decision"], "ask");
        assert_eq!(v["reason"], "need human: ambiguous");
        assert_eq!(v["adapter_version"], "0.1.0");
    }

    #[test]
    fn ask_empty_reason_defaults_to_failsafe() {
        let d = decision(Action::Ask, "");
        let v = render(&d, "0.1.0");
        assert_eq!(v["decision"], "ask");
        assert_eq!(v["reason"], "ask (fail-safe)");
    }

    #[test]
    fn unspecified_maps_to_ask() {
        let mut d = decision(Action::Ask, "");
        d.action = Action::Unspecified as i32;
        let v = render(&d, "0.1.0");
        assert_eq!(v["decision"], "ask");
    }

    #[test]
    fn adapter_version_always_present() {
        for action in [Action::Allow, Action::Deny, Action::Ask] {
            let d = decision(action, "r");
            let v = render(&d, "9.9.9");
            assert_eq!(
                v["adapter_version"], "9.9.9",
                "adapter_version must be present for {action:?}"
            );
        }
        // also for unknown schema path
        let d = decision(Action::Allow, "");
        let v = render_with_schema(&d, "9.9.9", Some("999"));
        assert_eq!(v["adapter_version"], "9.9.9");
    }

    #[test]
    fn unknown_schema_always_ask_unsupported() {
        let allow = decision(Action::Allow, "would be allow");
        let deny = decision(Action::Deny, "would be deny");
        let ask = decision(Action::Ask, "would be ask");
        for d in [&allow, &deny, &ask] {
            let v = render_with_schema(d, "0.1.0", Some("999"));
            assert_eq!(
                v["decision"], "ask",
                "unknown schema must force ask even for allow/deny"
            );
            assert_eq!(v["reason"], "unsupported_schema");
        }
    }

    #[test]
    fn unknown_schema_variants() {
        let d = decision(Action::Allow, "");
        for unknown in ["2", "2.0", "999", "v999", "unknown", "3.0.0", "future"] {
            let v = render_with_schema(&d, "0.1.0", Some(unknown));
            assert_eq!(v["decision"], "ask", "unknown {unknown} should be ask");
            assert_eq!(v["reason"], "unsupported_schema");
        }
    }

    #[test]
    fn known_schema_passes_through() {
        let allow = decision(Action::Allow, "");
        for known in [
            "1",
            "1.0",
            "v1",
            "v0",
            "PreToolUse",
            "PostToolUse",
            "0",
            "0.1.0",
            "1.0.0",
        ] {
            let v = render_with_schema(&allow, "0.1.0", Some(known));
            assert_eq!(
                v["decision"], "approve",
                "known schema {known} should pass through"
            );
        }
        // None also passes through
        let v_none = render_with_schema(&allow, "0.1.0", None);
        assert_eq!(v_none["decision"], "approve");

        let deny = decision(Action::Deny, "x");
        let v_deny_known = render_with_schema(&deny, "0.1.0", Some("1"));
        assert_eq!(v_deny_known["decision"], "block");

        let ask = decision(Action::Ask, "y");
        let v_ask_known = render_with_schema(&ask, "0.1.0", Some("PreToolUse"));
        assert_eq!(v_ask_known["decision"], "ask");
        assert_eq!(v_ask_known["reason"], "y");
    }

    #[test]
    fn empty_schema_version_is_not_unsupported() {
        let d = decision(Action::Allow, "");
        let v = render_with_schema(&d, "0.1.0", Some(""));
        assert_eq!(v["decision"], "approve");
        let v_ws = render_with_schema(&d, "0.1.0", Some("   "));
        assert_eq!(v_ws["decision"], "approve");
    }

    #[test]
    fn render_is_deterministic() {
        let d = decision(Action::Deny, "deny reason");
        let v1 = render(&d, "0.1.0");
        let v2 = render(&d, "0.1.0");
        assert_eq!(v1, v2);
    }

    #[test]
    fn version_gate_adapter_version_in_output() {
        // Spec: version-gate adapter_version in output – every render includes it
        let d = decision(Action::Allow, "");
        for ver in ["0.1.0", "1.2.3", env!("CARGO_PKG_VERSION")] {
            let v = render(&d, ver);
            assert_eq!(v["adapter_version"], ver);
        }
    }

    #[test]
    fn unknown_schema_even_with_skipped_reason_still_unsupported() {
        // Even if decision reason is skipped:unsupported_tool, unknown schema must win with unsupported_schema
        let mut d = decision(Action::Ask, "skipped:unsupported_tool");
        d.action = Action::Ask as i32;
        let v = render_with_schema(&d, "0.1.0", Some("999"));
        assert_eq!(v["reason"], "unsupported_schema");
        assert_eq!(v["decision"], "ask");
    }
}
