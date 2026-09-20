//! `algo-adapter-claude` – shell-only adapter (P1-07, `P1-AGENT-2`).
//!
//! - `parse`: Claude PreToolUse JSON for `Bash` (`tool_input.command`, `cwd`, `session_id`)
//!   → `CanonicalEvent { tool_kind: shell }`. Unsupported tools (Edit/Write/Read) in P1
//!   return `Err(SkippedUnsupportedTool)` and the caller maps to ask passthrough with
//!   `skipped:unsupported_tool`. Pure function, no policy logic. Redacts before logging.
//! - `render`: `Decision` → Claude hook JSON strings (`approve`/`block`/`ask`). Version-gated
//!   with `adapter_version`. Unknown schema version → `ask` + `unsupported_schema`.
//!
//! [VERIFY] Exact decision strings (`approve`/`block`/`ask`) and hook payload shapes must be
//! verified against current Claude docs on each Claude release – see `render.rs` header.
//! See `plans/phase-1-07-agent-claude-adapter-shell-only.md` for spec.

pub mod parse;
pub mod render;

use algo_types::ToolKind;

/// Adapter version – baked from `CARGO_PKG_VERSION`, emitted in every `render` output
/// for version-gating / debugging.
pub const ADAPTER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Alias satisfying spec naming `AdapterVersion = env!("CARGO_PKG_VERSION")`.
#[allow(non_upper_case_globals)]
pub const AdapterVersion: &str = ADAPTER_VERSION;

/// Canonical event for shell tools only in P1.
///
/// `tool_kind` is always `ToolKind::Shell` in P1. Other tool kinds are not emitted;
/// the parser returns `SkippedUnsupportedTool` for them so the caller can ask-passthrough.
#[derive(Debug, Clone)]
pub struct CanonicalEvent {
    /// Always `ToolKind::Shell` in P1.
    pub tool_kind: ToolKind,
    /// Raw shell command string (as received, before redaction for storage).
    pub command: String,
    /// Working directory from hook payload (`cwd`), empty if absent.
    pub cwd: String,
    /// Session identifier from hook payload (`session_id`), empty if absent.
    pub session_id: String,
    /// Full raw JSON value for audit/debug (never logged with secrets – redacted before logging).
    pub raw: serde_json::Value,
}

impl CanonicalEvent {
    /// Convenience: is this a shell event (always true in P1, but keeps call sites explicit).
    pub fn is_shell(&self) -> bool {
        self.tool_kind == ToolKind::Shell
    }
}
