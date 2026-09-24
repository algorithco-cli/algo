//! `algo-adapter-opencode` – OpenCode adapter (P4-03: shell + edit/write/read + stop).
//!
//! - `parse`: OpenCode plugin/config event JSON for shell/file tools → `CanonicalEvent`.
//!   Unknown tools still map to `SkippedUnsupportedTool` → ask. Pure, redacts.
//! - `render`: `Decision` → OpenCode permission JSON. Version-gated
//!   with `adapter_version`. Unknown schema version → `ask` + `unsupported_schema`.
//!
//! [VERIFY-OPEN] OpenCode plugin events and blockable-vs-observe-only mapping must
//! be verified against current OpenCode docs (P4-03 spike) – the current
//! parse/render logic is Claude-shaped and must not be trusted for OpenCode until
//! the spike lands. If interception cannot block, degrade to observe/advise via
//! ADR, never guess-and-allow. See `plans/phase-4-adapters.md`.

pub mod parse;
pub mod render;

use algo_types::ToolKind;

/// Adapter version – baked from `CARGO_PKG_VERSION`, emitted in every `render` output
/// for version-gating / debugging.
pub const ADAPTER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Alias satisfying spec naming `AdapterVersion = env!("CARGO_PKG_VERSION")`.
#[allow(non_upper_case_globals)]
pub const AdapterVersion: &str = ADAPTER_VERSION;

/// Canonical event for all tools in P2 (shell + edit/write/read + stop).
///
/// In P1 only Shell was emitted; P2 adds Edit/Write/Read (file_path + redacted preview)
/// and AgentStop passthrough. All events carry redactions; never raw secrets.
#[derive(Debug, Clone)]
pub struct CanonicalEvent {
    /// Tool kind (Shell/Edit/Write/Read/Other; Stop handled separately).
    pub tool_kind: ToolKind,
    /// Raw shell command string (for Shell) or file preview (for Edit/Write).
    pub command: String,
    /// Working directory from hook payload (`cwd`), empty if absent.
    pub cwd: String,
    /// Session identifier from hook payload (`session_id`), empty if absent.
    pub session_id: String,
    /// File path for Edit/Write/Read (None for Shell).
    pub file_path: Option<String>,
    /// Full raw JSON value for audit/debug (never logged with secrets – redacted before logging).
    pub raw: serde_json::Value,
}

impl CanonicalEvent {
    /// Convenience: is this a shell event.
    pub fn is_shell(&self) -> bool {
        self.tool_kind == ToolKind::Shell
    }

    /// Convenience: is this a file-edit event.
    pub fn is_edit(&self) -> bool {
        self.tool_kind == ToolKind::Edit
    }

    /// Convenience: is this a file-write event.
    pub fn is_write(&self) -> bool {
        self.tool_kind == ToolKind::Write
    }
}
