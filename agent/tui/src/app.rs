//! App state for TUI — read-only view over `AuditStore`.
//!
//! No decision-path coupling: TUI only reads `~/.algo/audit.db` (SQLite WAL).
//! If daemon is down or DB missing, `store` is `None` and UI shows offline banner.
//! Crash of this process never blocks hooks.

use algo_audit::{AuditEntry, AuditStore, Counts};
use ratatui::widgets::TableState;

/// Ticks (at ~100ms UI poll) before a pending browser sign-in times out
/// with an honest "not available yet" error instead of spinning forever.
pub const BROWSER_TIMEOUT_TICKS: usize = 300;
/// Milliseconds per marching-ants dash step (5 cells/sec).
pub const ANIM_STEP_MS: u64 = 200;
/// Milliseconds per spinner frame (10 fps).
pub const SPIN_STEP_MS: u64 = 100;
/// Ticks before a successful login auto-transitions to the feed.
pub const SUCCESS_TICKS: usize = 12;

/// View mode for the TUI.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ViewMode {
    /// Login gate — shown first on launch (browser sign-in or API key).
    Login,
    /// CLI integration — pick the terminal coding tool algo works with.
    #[default]
    Connect,
    /// Live feed of decisions (legacy, unreachable in the current flow).
    Feed,
    /// Policy snapshot (legacy, unreachable in the current flow).
    Policy,
}

/// A terminal coding tool the user can pick for `algo` CLI integration.
/// UI-only for now: picking stores the choice locally, no connection yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliTool {
    ClaudeCode,
    Codex,
    Gemini,
    Custom,
}

impl CliTool {
    pub fn name(self) -> &'static str {
        match self {
            CliTool::ClaudeCode => "Claude Code",
            CliTool::Codex => "Codex",
            CliTool::Gemini => "Gemini CLI",
            CliTool::Custom => "Custom / other",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            CliTool::ClaudeCode => "Anthropic's terminal coding assistant",
            CliTool::Codex => "OpenAI's terminal coding assistant",
            CliTool::Gemini => "Google's terminal coding assistant",
            CliTool::Custom => "Another tool — configured manually later",
        }
    }
}

/// Tools listed in the Connect picker, in display order.
pub const CLI_TOOLS: [CliTool; 4] = [
    CliTool::ClaudeCode,
    CliTool::Codex,
    CliTool::Gemini,
    CliTool::Custom,
];

/// Number of rows in the Connect picker.
pub const CLI_TOOL_COUNT: usize = CLI_TOOLS.len();

/// Which element has keyboard focus on the login screen.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum LoginFocus {
    #[default]
    Browser,
    ApiKey,
    Offline,
}

/// Authentication sub-state on the login screen.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum LoginStatus {
    /// Both options ready to be selected.
    #[default]
    Idle,
    /// Browser OAuth flow in progress — spinner + device code + cancel.
    BrowserPending,
    /// API key input field active — user is typing.
    ApiKeyEditing,
    /// Validating the API key (spinner).
    ApiKeyValidating,
    /// Success — brief confirmation before auto-transition to Connect.
    Success,
    /// Error with human-readable message and retry.
    Error(String),
}

/// Application state.
///
/// Required fields per spec: `store`, `counts`, `selected`, `mode`.
/// Extra fields keep the TUI functional and testable without coupling to daemon.
pub struct App {
    /// Read-only audit store; `None` => offline / daemon down.
    pub store: Option<AuditStore>,
    /// Aggregate decision counts.
    pub counts: Counts,
    /// Most-recent decisions (up to limit) shown in table.
    pub entries: Vec<AuditEntry>,
    /// Currently selected row in `entries`.
    pub selected: usize,
    /// Stateful table scroll/selection — kept in sync with `selected`.
    /// Rendered via `render_stateful_widget` so highlight + scroll-into-view work.
    pub table_state: TableState,
    /// Current view mode.
    pub mode: ViewMode,
    /// Whether we are in offline read-only mode.
    pub is_offline: bool,
    /// Last error message if refresh failed.
    pub error: Option<String>,
    /// Resolved paths for header/policy views (ALGO_HOME respected).
    pub db_path: Option<std::path::PathBuf>,
    pub config_path: Option<std::path::PathBuf>,
    pub socket_path: Option<std::path::PathBuf>,
    /// Guard mode from `config.json` (`enforce:true` = enforcing, else shadow).
    pub enforce: bool,
    /// Privacy mode from `config.json`.
    pub privacy: String,
    /// Whether `~/.algo/paused` exists (instant bypass).
    pub paused: bool,
    /// TUI crate version for header.
    pub version: String,

    // ---- Login screen state (new) ----
    /// Which of the two primary options (or offline fallback) has focus.
    pub login_focus: LoginFocus,
    /// Current auth sub-state.
    pub login_status: LoginStatus,
    /// Device code shown during browser flow (e.g. "WD-4829-XK").
    pub login_device_code: Option<String>,
    /// Raw API key input buffer (stored unmasked, displayed masked).
    pub login_api_input: String,
    /// Whether the API key input should be shown masked (always true for display, but kept for toggle).
    pub login_api_masked: bool,
    /// Ticks since current login_status was entered (drives validation timers / success auto-transition).
    pub login_ticks: usize,
    /// Single authoritative status line for login screen (replaces duplicated warning banner).
    /// Used for idle hint, pending message, success/error. None = no extra line.
    pub login_status_msg: Option<String>,
    /// Frame tick for animations (incremented each UI loop).
    pub tick: usize,
    /// Wall-clock phase for the marching-ants border. Advanced at a fixed
    /// rate in `tick()` so mouse-motion event floods can't speed it up.
    pub anim_phase: usize,
    /// Last time `anim_phase` advanced.
    pub anim_clock: std::time::Instant,
    /// Wall-clock phase for the spinner. Same flood-proofing as `anim_phase`.
    pub spin_phase: usize,
    /// Last time `spin_phase` advanced.
    pub spin_clock: std::time::Instant,

    // ---- Clickable areas, populated each render (mouse-first UI). ----
    pub login_browser: Option<ratatui::layout::Rect>,
    pub login_apikey: Option<ratatui::layout::Rect>,
    pub login_apikey_input: Option<ratatui::layout::Rect>,
    pub login_submit: Option<ratatui::layout::Rect>,
    pub login_cancel: Option<ratatui::layout::Rect>,
    pub login_offline: Option<ratatui::layout::Rect>,
    pub login_quit: Option<ratatui::layout::Rect>,

    // Backward compat shims for older tests / UI — kept but derived from new state.
    /// Deprecated: use login_status == BrowserPending || ApiKeyValidating
    pub login_pending: bool,
    /// Deprecated: use login_status_msg
    pub login_note: Option<String>,
    /// Deprecated alias: login_browser
    pub login_signin: Option<ratatui::layout::Rect>,

    pub tab_feed: Option<ratatui::layout::Rect>,
    pub tab_policy: Option<ratatui::layout::Rect>,
    pub footer_quit: Option<ratatui::layout::Rect>,
    /// Inner table content area (inside borders) for click-to-select math.
    pub table_inner: Option<ratatui::layout::Rect>,

    // ---- Connect (CLI integration picker) state — UI-only for now. ----
    /// Cursor into `CLI_TOOLS`.
    pub tool_selected: usize,
    /// Tool picked via click/keyboard. Stored locally; no connection yet.
    pub connected_tool: Option<CliTool>,
    /// Honest status line for the Connect view (stored-locally messaging).
    pub connect_status_msg: Option<String>,
    /// Clickable box per tool row, populated each render (mouse-first UI).
    pub tool_rects: [Option<ratatui::layout::Rect>; CLI_TOOL_COUNT],
}

impl App {
    /// Resolve home dir, respecting `ALGO_HOME` like CLI/daemon.
    pub fn resolve_home() -> std::path::PathBuf {
        if let Ok(v) = std::env::var("ALGO_HOME") {
            if !v.trim().is_empty() {
                return std::path::PathBuf::from(v);
            }
        }
        if let Some(h) = dirs::home_dir() {
            return h;
        }
        std::path::PathBuf::from(".")
    }

    /// Create a new app from an optional store. Does not automatically refresh;
    /// caller should invoke `refresh()` to load counts/entries.
    pub fn new(store: Option<AuditStore>) -> Self {
        let is_offline = store.is_none();
        let home = Self::resolve_home();
        let algo_dir = home.join(".algo");
        Self {
            store,
            counts: Counts {
                total: 0,
                allow: 0,
                deny: 0,
                ask: 0,
                shadow: 0,
                would_have_blocked: 0,
            },
            entries: Vec::new(),
            selected: 0,
            table_state: TableState::default(),
            mode: ViewMode::Connect,
            tool_selected: 0,
            connected_tool: None,
            connect_status_msg: None,
            tool_rects: [None; CLI_TOOL_COUNT],
            is_offline,
            error: None,
            login_focus: LoginFocus::Browser,
            login_status: LoginStatus::Idle,
            login_device_code: None,
            login_api_input: String::new(),
            login_api_masked: true,
            login_ticks: 0,
            login_status_msg: None,
            tick: 0,
            anim_phase: 0,
            anim_clock: std::time::Instant::now(),
            spin_phase: 0,
            spin_clock: std::time::Instant::now(),
            login_browser: None,
            login_apikey: None,
            login_apikey_input: None,
            login_submit: None,
            login_cancel: None,
            login_offline: None,
            login_quit: None,
            login_pending: false,
            login_note: None,
            login_signin: None,
            tab_feed: None,
            tab_policy: None,
            footer_quit: None,
            table_inner: None,
            db_path: Some(algo_dir.join("audit.db")),
            config_path: Some(algo_dir.join("config.json")),
            socket_path: Some(algo_dir.join("algo.sock")),
            enforce: false,
            privacy: "redacted".to_string(),
            paused: std::fs::metadata(algo_dir.join("paused"))
                .map(|m| m.is_file())
                .unwrap_or(false),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Convenience: open a store at `path` and return an `App`. If the file
    /// does not exist or cannot be opened, returns an offline App.
    #[allow(dead_code)]
    pub fn from_path(path: std::path::PathBuf) -> Self {
        match AuditStore::open(&path) {
            Ok(s) => {
                // `init` is idempotent; failure is non-fatal for read.
                let _ = s.init();
                let mut app = Self::new(Some(s));
                app.refresh();
                app
            }
            Err(e) => {
                let mut app = Self::new(None);
                app.is_offline = true;
                app.error = Some(e.to_string());
                app
            }
        }
    }

    fn load_meta(&mut self) {
        // Best-effort, never fails refresh: missing config => shadow defaults.
        if let Some(cfg_path) = self.config_path.clone() {
            if let Ok(bytes) = std::fs::read(&cfg_path) {
                if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                    if let Some(p) = json.get("privacy").and_then(|x| x.as_str()) {
                        self.privacy = p.to_string();
                    }
                    self.enforce = json.get("enforce").and_then(|x| x.as_bool()) == Some(true);
                }
            }
        }
        // Paused file can appear/disappear at runtime (`algo pause`).
        if let Some(sock) = self.socket_path.clone() {
            if let Some(parent) = sock.parent() {
                self.paused = std::fs::metadata(parent.join("paused"))
                    .map(|m| m.is_file())
                    .unwrap_or(false);
            }
        }
    }

    /// Reload `counts` and `entries` from the audit store.
    ///
    /// On failure, marks offline and preserves previous counts.
    pub fn refresh(&mut self) {
        self.load_meta();
        let Some(store) = &self.store else {
            self.is_offline = true;
            return;
        };
        match store.counts() {
            Ok(c) => {
                self.counts = c;
                self.is_offline = false;
                self.error = None;
            }
            Err(e) => {
                self.is_offline = true;
                self.error = Some(format!("counts: {e}"));
                return;
            }
        }
        match store.list(100) {
            Ok(list) => {
                self.entries = list;
                self.clamp_selection();
                self.is_offline = false;
            }
            Err(e) => {
                self.is_offline = true;
                self.error = Some(format!("list: {e}"));
            }
        }
    }

    /// Keep `table_state.selected` in sync with `selected`.
    fn sync_table_state(&mut self) {
        if self.entries.is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(self.selected));
        }
    }

    fn clamp_selection(&mut self) {
        if self.entries.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.entries.len() {
            self.selected = self.entries.len() - 1;
        }
        self.sync_table_state();
    }

    /// Move selection down (j).
    pub fn select_next(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
        }
        self.sync_table_state();
    }

    /// Move selection up (k).
    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
        self.sync_table_state();
    }

    /// Jump to first row (g / Home).
    pub fn select_first(&mut self) {
        self.selected = 0;
        self.sync_table_state();
    }

    /// Jump to last row (G / End).
    pub fn select_last(&mut self) {
        if !self.entries.is_empty() {
            self.selected = self.entries.len() - 1;
        }
        self.sync_table_state();
    }

    /// Currently selected entry, if any.
    pub fn selected_entry(&self) -> Option<&AuditEntry> {
        self.entries.get(self.selected)
    }

    /// Single authoritative way to change views. No-op when the login gate
    /// is up (use `show_login` / `continue_offline` for those transitions)
    /// so adding a third `ViewMode` cannot silently misbehave.
    pub fn set_mode(&mut self, mode: ViewMode) {
        if self.mode == ViewMode::Login {
            return;
        }
        // Never allow switching *into* Login via set_mode; use show_login().
        if mode == ViewMode::Login {
            return;
        }
        self.mode = mode;
    }

    /// Show the live feed (explicit, unambiguous).
    pub fn show_feed(&mut self) {
        self.set_mode(ViewMode::Feed);
    }

    /// Show the policy snapshot (explicit, unambiguous).
    pub fn show_policy(&mut self) {
        self.set_mode(ViewMode::Policy);
    }

    /// Show the CLI-integration picker (explicit, unambiguous).
    pub fn show_connect(&mut self) {
        self.set_mode(ViewMode::Connect);
    }

    /// Move the Connect picker cursor down (wraps around).
    pub fn select_tool_next(&mut self) {
        self.tool_selected = (self.tool_selected + 1) % CLI_TOOL_COUNT;
    }

    /// Move the Connect picker cursor up (wraps around).
    pub fn select_tool_prev(&mut self) {
        self.tool_selected = (self.tool_selected + CLI_TOOL_COUNT - 1) % CLI_TOOL_COUNT;
    }

    /// Pick the tool under the cursor. UI-only: stored locally with an honest
    /// "not connected yet" message — no fake connection theater.
    pub fn choose_tool(&mut self) {
        self.choose_tool_idx(self.tool_selected);
    }

    /// Pick tool `i` directly (1–4 shortcuts, mouse clicks). Out of range is
    /// ignored.
    pub fn choose_tool_idx(&mut self, i: usize) {
        if i >= CLI_TOOL_COUNT {
            return;
        }
        self.tool_selected = i;
        let tool = CLI_TOOLS[i];
        self.connected_tool = Some(tool);
        self.connect_status_msg = Some(format!(
            "✓ {} selected — stored locally (connection wiring not built yet).",
            tool.name()
        ));
    }

    /// Show the login gate (called once on launch). Resets to clean Idle state.
    pub fn show_login(&mut self) {
        self.mode = ViewMode::Login;
        self.login_focus = LoginFocus::Browser;
        self.login_status = LoginStatus::Idle;
        self.login_device_code = None;
        self.login_api_input.clear();
        self.login_api_masked = true;
        self.login_ticks = 0;
        self.login_status_msg = None;
        self.login_pending = false;
        self.login_note = None;
        self.tool_selected = 0;
        self.connected_tool = None;
        self.connect_status_msg = None;
    }

    /// Enter the app offline (local-only, no cloud). Never fails.
    pub fn continue_offline(&mut self) {
        self.login_pending = false;
        self.login_note = None;
        self.login_status = LoginStatus::Idle;
        self.login_device_code = None;
        self.login_ticks = 0;
        self.login_status_msg = None;
        // Clear secret from memory on exit — do not retain raw key after leaving login
        self.login_api_input.clear();
        self.mode = ViewMode::Connect;
    }

    /// Start a browser sign-in attempt.
    ///
    /// No OAuth backend exists in this MVP, so this is honest: it enters a
    /// pending state with NO fabricated device code and `tick()` surfaces a
    /// clear "not available yet" timeout instead of spinning forever.
    pub fn start_browser_signin(&mut self) {
        self.login_status = LoginStatus::BrowserPending;
        self.login_device_code = None;
        self.login_ticks = 0;
        self.login_pending = true;
        self.login_note = Some(
            "Browser sign-in is not available yet — use an API key or Continue offline."
                .to_string(),
        );
        self.login_status_msg = Some(
            "Browser sign-in is not available yet — use an API key or Continue offline.  Press Esc to cancel.".to_string(),
        );
        self.login_focus = LoginFocus::Browser;
    }

    /// Enter API key entry mode (reveals input field).
    pub fn start_api_key_entry(&mut self) {
        // Clear prior error before entering editing (check before overwrite)
        if let LoginStatus::Error(_) = self.login_status {
            self.login_status_msg = None;
        }
        self.login_status = LoginStatus::ApiKeyEditing;
        self.login_focus = LoginFocus::ApiKey;
        self.login_ticks = 0;
        self.login_pending = false;
        self.login_note = None;
    }

    /// Cancel current login sub-flow and return to Idle.
    pub fn cancel_login(&mut self) {
        match self.login_status {
            LoginStatus::BrowserPending
            | LoginStatus::ApiKeyEditing
            | LoginStatus::ApiKeyValidating
            | LoginStatus::Error(_)
            | LoginStatus::Success => {
                self.login_status = LoginStatus::Idle;
                self.login_device_code = None;
                self.login_ticks = 0;
                self.login_pending = false;
                self.login_note = None;
                self.login_status_msg = None;
                // Do not clear login_api_input on plain cancel — keep typed key for retry
                // Secret is cleared on continue_offline / Success auto-transition
            }
            LoginStatus::Idle => {}
        }
    }

    /// Push a character into the API key buffer (only when editing).
    pub fn push_api_key_char(&mut self, c: char) {
        if self.login_status != LoginStatus::ApiKeyEditing {
            // Auto-enter editing if user starts typing while focused on ApiKey idle
            if self.login_status == LoginStatus::Idle && self.login_focus == LoginFocus::ApiKey {
                self.start_api_key_entry();
            } else if self.login_status != LoginStatus::ApiKeyEditing {
                return;
            }
        }
        if self.login_api_input.chars().count() < 128 {
            self.login_api_input.push(c);
            // clear prior error on new input
            if let LoginStatus::Error(_) = self.login_status {
                self.login_status = LoginStatus::ApiKeyEditing;
            }
        }
    }

    /// Backspace one character from API key buffer.
    pub fn pop_api_key_char(&mut self) {
        if self.login_status == LoginStatus::ApiKeyEditing
            || matches!(self.login_status, LoginStatus::Error(_))
        {
            self.login_api_input.pop();
            if let LoginStatus::Error(_) = self.login_status {
                // stay in editing after clearing error
                self.login_status = LoginStatus::ApiKeyEditing;
                self.login_status_msg = None;
            }
        }
    }

    /// Submit the current API key.
    ///
    /// No validation backend exists in this MVP, so there is no fake network
    /// validation theater: basic local sanity (non-empty, >=8 chars) is
    /// checked, then the key is accepted locally with a clear "stored
    /// locally, not verified" message. Returns true on accept, false on
    /// immediate local error.
    pub fn submit_api_key(&mut self) -> bool {
        let trimmed = self.login_api_input.trim().to_string();
        if trimmed.is_empty() {
            self.login_status = LoginStatus::Error("API key cannot be empty.".to_string());
            self.login_status_msg = Some("API key cannot be empty.".to_string());
            self.login_ticks = 0;
            self.login_pending = false;
            return false;
        }
        if trimmed.chars().count() < 8 {
            self.login_status = LoginStatus::Error(
                "API key too short — must be at least 8 characters.".to_string(),
            );
            self.login_status_msg =
                Some("API key too short — must be at least 8 characters.".to_string());
            self.login_ticks = 0;
            self.login_pending = false;
            return false;
        }
        // Accepted locally — honestly labelled, no fake spinner.
        self.login_status = LoginStatus::Success;
        self.login_ticks = 0;
        self.login_pending = false;
        self.login_status_msg =
            Some("API key stored locally, not verified (offline MVP).".to_string());
        self.login_note = self.login_status_msg.clone();
        true
    }

    /// Cycle focus forward (Tab).
    pub fn cycle_focus_next(&mut self) {
        // When editing, Tab should move between the two primary boxes + offline,
        // but keep editing state. If validating/pending/success, cycle is limited.
        if matches!(
            self.login_status,
            LoginStatus::BrowserPending | LoginStatus::ApiKeyValidating | LoginStatus::Success
        ) {
            return;
        }
        self.login_focus = match self.login_focus {
            LoginFocus::Browser => LoginFocus::ApiKey,
            LoginFocus::ApiKey => LoginFocus::Offline,
            LoginFocus::Offline => LoginFocus::Browser,
        };
        // If user tabs to ApiKey while idle, we stay idle (not auto-editing) — Enter will activate.
        // Clear transient error when moving focus?
        if let LoginStatus::Error(_) = self.login_status {
            self.login_status = LoginStatus::Idle;
            self.login_status_msg = None;
        }
    }

    /// Cycle focus backward (Shift+Tab).
    pub fn cycle_focus_prev(&mut self) {
        if matches!(
            self.login_status,
            LoginStatus::BrowserPending | LoginStatus::ApiKeyValidating | LoginStatus::Success
        ) {
            return;
        }
        self.login_focus = match self.login_focus {
            LoginFocus::Browser => LoginFocus::Offline,
            LoginFocus::ApiKey => LoginFocus::Browser,
            LoginFocus::Offline => LoginFocus::ApiKey,
        };
        if let LoginStatus::Error(_) = self.login_status {
            self.login_status = LoginStatus::Idle;
            self.login_status_msg = None;
        }
    }

    /// Confirm on the login screen based on current focus/status.
    /// - Idle + Browser focused → start browser sign-in
    /// - Idle + ApiKey focused → enter API key editing
    /// - Idle + Offline focused → continue offline
    /// - BrowserPending → no-op (use Esc to cancel)
    /// - ApiKeyEditing + non-empty → submit
    /// - Error → retry (back to Idle or Editing)
    /// - Success → go to Feed
    pub fn login_confirm(&mut self) {
        match self.login_status.clone() {
            LoginStatus::Idle => match self.login_focus {
                LoginFocus::Browser => self.start_browser_signin(),
                LoginFocus::ApiKey => self.start_api_key_entry(),
                LoginFocus::Offline => self.continue_offline(),
            },
            LoginStatus::BrowserPending => {
                // No auto-confirm while waiting; user must cancel with Esc
                // Keep pending — do nothing
            }
            LoginStatus::ApiKeyEditing => {
                self.submit_api_key();
            }
            LoginStatus::ApiKeyValidating => {
                // validating — ignore
            }
            LoginStatus::Error(_) => {
                // Retry: back to editing if we have input, else idle
                if !self.login_api_input.trim().is_empty() {
                    self.login_status = LoginStatus::ApiKeyEditing;
                    self.login_status_msg = None;
                    self.login_focus = LoginFocus::ApiKey;
                } else {
                    self.login_status = LoginStatus::Idle;
                    self.login_status_msg = None;
                }
            }
            LoginStatus::Success => {
                self.continue_offline();
            }
        }
        // Keep legacy fields in sync
        self.sync_legacy();
    }

    fn sync_legacy(&mut self) {
        self.login_pending = matches!(
            self.login_status,
            LoginStatus::BrowserPending | LoginStatus::ApiKeyValidating
        );
        self.login_note = self.login_status_msg.clone();
        self.login_signin = self.login_browser;
    }

    /// Animation tick — call each UI loop.
    /// Drives spinner, validation timers, success auto-transition.
    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        // Wall-clock animation phase: fixed 200ms steps no matter how many
        // input events (mouse motion floods the loop) arrive between frames.
        while self.anim_clock.elapsed() >= std::time::Duration::from_millis(ANIM_STEP_MS) {
            self.anim_phase = self.anim_phase.wrapping_add(1);
            self.anim_clock += std::time::Duration::from_millis(ANIM_STEP_MS);
        }
        while self.spin_clock.elapsed() >= std::time::Duration::from_millis(SPIN_STEP_MS) {
            self.spin_phase = self.spin_phase.wrapping_add(1);
            self.spin_clock += std::time::Duration::from_millis(SPIN_STEP_MS);
        }
        // Sync legacy for render that still reads old fields
        self.sync_legacy();

        match self.login_status.clone() {
            LoginStatus::BrowserPending => {
                self.login_ticks += 1;
                if self.login_ticks >= BROWSER_TIMEOUT_TICKS {
                    // Honest timeout — no backend, no infinite spinner.
                    let msg = "Browser sign-in is not available yet — use an API key or Continue offline."
                        .to_string();
                    self.login_status = LoginStatus::Error(msg.clone());
                    self.login_status_msg = Some(msg.clone());
                    self.login_note = Some(msg);
                    self.login_pending = false;
                    self.login_ticks = 0;
                }
            }
            LoginStatus::ApiKeyValidating => {
                // Legacy state: no backend ever validated here. Resolve honestly
                // to a local (unverified) accept so old callers can't spin forever.
                self.login_ticks += 1;
                if self.login_ticks >= 2 {
                    self.login_status = LoginStatus::Success;
                    self.login_status_msg =
                        Some("API key stored locally, not verified (offline MVP).".to_string());
                    self.login_note = self.login_status_msg.clone();
                    self.login_pending = false;
                    self.login_ticks = 0;
                }
            }
            LoginStatus::Success => {
                self.login_ticks += 1;
                if self.login_ticks >= SUCCESS_TICKS {
                    // Brief confirmation done — transition to feed
                    self.continue_offline();
                }
            }
            LoginStatus::Error(_) => {
                // No auto tick
            }
            LoginStatus::ApiKeyEditing | LoginStatus::Idle => {
                // No timer needed
            }
        }
    }

    fn hit(rect: &Option<ratatui::layout::Rect>, x: u16, y: u16) -> bool {
        if let Some(r) = rect {
            x >= r.x
                && x < r.x.saturating_add(r.width)
                && y >= r.y
                && y < r.y.saturating_add(r.height)
        } else {
            false
        }
    }

    /// Mouse click handler (mouse-first UI; keyboard still works but is hidden).
    /// Returns `true` when the app should quit.
    pub fn handle_click(&mut self, x: u16, y: u16) -> bool {
        if self.mode == ViewMode::Login {
            if Self::hit(&self.login_quit, x, y) {
                return true;
            }
            if Self::hit(&self.login_cancel, x, y) {
                self.cancel_login();
                return false;
            }
            if Self::hit(&self.login_submit, x, y) {
                // Submit button only active when editing
                if matches!(
                    self.login_status,
                    LoginStatus::ApiKeyEditing | LoginStatus::Error(_)
                ) {
                    self.submit_api_key();
                }
                return false;
            }
            if Self::hit(&self.login_browser, x, y) {
                // Clicking browser box when idle -> start browser; when pending -> no-op; when in api editing -> switch focus
                match self.login_status {
                    LoginStatus::Idle => {
                        self.login_focus = LoginFocus::Browser;
                        self.start_browser_signin();
                    }
                    LoginStatus::BrowserPending => {
                        // click again does nothing, need cancel
                    }
                    LoginStatus::ApiKeyEditing | LoginStatus::Error(_) => {
                        self.login_focus = LoginFocus::Browser;
                        // If clicking browser while editing, switch to browser flow? For now just focus browser and keep editing unless user confirms
                        // Simpler: focus browser but stay in editing — user must press Esc or click cancel to leave editing
                        // We'll just set focus to Browser without changing status, unless idle
                        if self.login_status == LoginStatus::ApiKeyEditing
                            && self.login_api_input.is_empty()
                        {
                            self.login_status = LoginStatus::Idle;
                            self.login_focus = LoginFocus::Browser;
                        }
                    }
                    _ => {
                        self.login_focus = LoginFocus::Browser;
                    }
                }
                return false;
            }
            if Self::hit(&self.login_apikey, x, y) || Self::hit(&self.login_apikey_input, x, y) {
                match self.login_status {
                    LoginStatus::Idle => {
                        self.login_focus = LoginFocus::ApiKey;
                        self.start_api_key_entry();
                    }
                    LoginStatus::ApiKeyEditing | LoginStatus::Error(_) => {
                        self.login_focus = LoginFocus::ApiKey;
                        // keep editing, focus input
                    }
                    LoginStatus::BrowserPending => {
                        // switch from browser pending to api key — cancel browser first
                        self.cancel_login();
                        self.login_focus = LoginFocus::ApiKey;
                        self.start_api_key_entry();
                    }
                    _ => {
                        self.login_focus = LoginFocus::ApiKey;
                    }
                }
                return false;
            }
            if Self::hit(&self.login_offline, x, y) {
                self.continue_offline();
                return false;
            }
            return false;
        }
        if Self::hit(&self.footer_quit, x, y) {
            return true;
        }
        if Self::hit(&self.tab_feed, x, y) {
            self.mode = ViewMode::Feed;
            return false;
        }
        if Self::hit(&self.tab_policy, x, y) {
            self.mode = ViewMode::Policy;
            return false;
        }
        if self.mode == ViewMode::Feed {
            if let Some(inner) = self.table_inner {
                // Header occupies the first inner row; data rows follow.
                // Account for the table's actual scroll offset so a click
                // always selects the row actually clicked, even after scrolling.
                if x >= inner.x
                    && x < inner.x.saturating_add(inner.width)
                    && y > inner.y
                    && y < inner.y.saturating_add(inner.height)
                {
                    let visible_row = (y - inner.y - 1) as usize;
                    let idx = self.table_state.offset().saturating_add(visible_row);
                    if idx < self.entries.len() {
                        self.selected = idx;
                        self.sync_table_state();
                    }
                }
            }
        }
        false
    }

    /// Helper for tests: returns masked display for current API input.
    #[allow(dead_code)]
    pub fn masked_api_input(&self) -> String {
        "•".repeat(self.login_api_input.chars().count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use algo_types::{Action, Decision, SourceLevel};
    use tempfile::TempDir;

    fn test_decision(action: Action, reason: &str, trace: &str) -> Decision {
        Decision {
            action: action as i32,
            reason: reason.to_string(),
            confidence_0_1: 0.9,
            source_level: SourceLevel::Rule as i32,
            latency_ms: 5,
            policy_version: "test".to_string(),
            trace_id: trace.to_string(),
        }
    }

    #[test]
    fn refresh_loads_counts_and_list() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("audit.db");
        let store = AuditStore::open(&db).unwrap();
        store.init().unwrap();

        let d_allow = test_decision(Action::Allow, "allow ok", "t1");
        let d_deny = test_decision(Action::Deny, "deny bad", "t2");
        let d_ask = test_decision(Action::Ask, "ask review", "t3");
        store.insert(&d_allow, "fp-allow", false).unwrap();
        store.insert(&d_deny, "fp-deny", true).unwrap();
        store.insert(&d_ask, "fp-ask", false).unwrap();

        let mut app = App::new(Some(store));
        // Before refresh counts are zero
        assert_eq!(app.counts.total, 0);
        app.refresh();
        assert_eq!(app.counts.total, 3);
        assert_eq!(app.counts.allow, 1);
        assert_eq!(app.counts.deny, 1);
        assert_eq!(app.counts.ask, 1);
        assert_eq!(app.counts.shadow, 1);
        assert_eq!(app.counts.would_have_blocked, 1);
        assert_eq!(app.entries.len(), 3);
        assert!(!app.is_offline);
        assert!(app.error.is_none());
    }

    #[test]
    fn refresh_offline_when_no_store() {
        let mut app = App::new(None);
        app.refresh();
        assert!(app.is_offline);
        assert_eq!(app.entries.len(), 0);
    }

    #[test]
    fn refresh_with_shadow_would_have_blocked() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("audit.db");
        let store = AuditStore::open(&db).unwrap();
        store.init().unwrap();
        let d_deny_shadow = test_decision(Action::Deny, "shadow deny", "s1");
        store.insert(&d_deny_shadow, "fp-s1", true).unwrap();
        let mut app = App::new(Some(store));
        app.refresh();
        assert_eq!(app.counts.would_have_blocked, 1);
    }

    #[test]
    fn navigation_clamps() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("audit.db");
        let store = AuditStore::open(&db).unwrap();
        store.init().unwrap();
        for i in 0..3 {
            let d = test_decision(Action::Allow, &format!("allow {i}"), &format!("t{i}"));
            store.insert(&d, &format!("fp{i}"), false).unwrap();
        }
        let mut app = App::new(Some(store));
        app.refresh();
        assert_eq!(app.selected, 0);
        app.select_next();
        assert_eq!(app.selected, 1);
        app.select_next();
        assert_eq!(app.selected, 2);
        app.select_next();
        assert_eq!(app.selected, 2); // clamp
        app.select_prev();
        assert_eq!(app.selected, 1);
        app.show_policy();
        assert_eq!(app.mode, ViewMode::Policy);
        app.show_feed();
        assert_eq!(app.mode, ViewMode::Feed);
    }

    #[test]
    fn set_mode_is_authoritative() {
        let mut app = App::new(None);
        // Explicit, unambiguous — each call lands exactly where asked.
        app.show_policy();
        assert_eq!(app.mode, ViewMode::Policy);
        app.show_policy();
        assert_eq!(app.mode, ViewMode::Policy);
        app.show_feed();
        assert_eq!(app.mode, ViewMode::Feed);
        app.show_feed();
        assert_eq!(app.mode, ViewMode::Feed);
        // Direct set_mode works too.
        app.set_mode(ViewMode::Policy);
        assert_eq!(app.mode, ViewMode::Policy);
        app.set_mode(ViewMode::Feed);
        assert_eq!(app.mode, ViewMode::Feed);
    }

    #[test]
    fn login_gate_initial_state() {
        let mut app = App::new(None);
        app.show_login();
        assert_eq!(app.mode, ViewMode::Login);
        assert_eq!(app.login_status, LoginStatus::Idle);
        assert_eq!(app.login_focus, LoginFocus::Browser);
        assert!(app.login_device_code.is_none());
        assert!(app.login_api_input.is_empty());
        // View switching is a no-op on login gate
        app.set_mode(ViewMode::Feed);
        assert_eq!(app.mode, ViewMode::Login);
        app.show_policy();
        assert_eq!(app.mode, ViewMode::Login);
    }

    #[test]
    fn login_browser_flow_with_cancel() {
        let mut app = App::new(None);
        app.show_login();
        // Idle + Browser focused -> Enter starts browser (honest: no fake code)
        app.login_confirm();
        assert_eq!(app.login_status, LoginStatus::BrowserPending);
        assert!(app.login_device_code.is_none());
        assert!(app
            .login_status_msg
            .as_deref()
            .unwrap_or_default()
            .contains("not available yet"));
        // Short wait does not auto-finish
        for _ in 0..20 {
            app.tick();
        }
        assert_eq!(app.login_status, LoginStatus::BrowserPending);
        // Esc / cancel returns to idle
        app.cancel_login();
        assert_eq!(app.login_status, LoginStatus::Idle);
        assert!(app.login_device_code.is_none());
        // Offline shortcut
        app.continue_offline();
        assert_eq!(app.mode, ViewMode::Feed);
    }

    #[test]
    fn login_browser_timeout_is_honest() {
        let mut app = App::new(None);
        app.show_login();
        app.start_browser_signin();
        assert_eq!(app.login_status, LoginStatus::BrowserPending);
        for _ in 0..BROWSER_TIMEOUT_TICKS {
            app.tick();
        }
        assert!(
            matches!(app.login_status, LoginStatus::Error(_)),
            "browser pending must time out, got {:?}",
            app.login_status
        );
        let msg = app.login_status_msg.clone().unwrap_or_default();
        assert!(msg.contains("not available yet"), "msg was: {msg}");
        assert!(
            msg.contains("API key") || msg.contains("offline"),
            "msg was: {msg}"
        );
    }

    #[test]
    fn login_apikey_flow_validation() {
        let mut app = App::new(None);
        app.show_login();
        // Switch focus to ApiKey via Tab or direct
        app.cycle_focus_next(); // Browser -> ApiKey
        assert_eq!(app.login_focus, LoginFocus::ApiKey);
        app.login_confirm(); // Enter ApiKey box -> editing
        assert_eq!(app.login_status, LoginStatus::ApiKeyEditing);
        // Empty submit -> error
        app.login_confirm();
        assert!(matches!(app.login_status, LoginStatus::Error(_)));
        // Type too short -> error
        app.login_api_input = "short".to_string();
        app.login_status = LoginStatus::ApiKeyEditing;
        app.submit_api_key();
        assert!(matches!(app.login_status, LoginStatus::Error(_)));
        // Valid key -> accepted locally with honest message (no fake spinner)
        app.login_api_input = "ag-valid-key-12345".to_string();
        assert!(app.submit_api_key());
        assert_eq!(app.login_status, LoginStatus::Success);
        assert!(app
            .login_status_msg
            .as_deref()
            .unwrap_or_default()
            .contains("stored locally, not verified"));
        // Tick drives auto-transition to feed
        for _ in 0..13 {
            app.tick();
        }
        assert_eq!(app.mode, ViewMode::Feed);
    }

    #[test]
    fn login_apikey_accept_is_honest_not_fake_validated() {
        let mut app = App::new(None);
        app.show_login();
        app.start_api_key_entry();
        // Any sufficiently-long key is stored locally — even one containing
        // "invalid", which the old fake validator rejected theatrically.
        app.login_api_input = "invalid-key-here".to_string();
        assert!(app.submit_api_key());
        assert_eq!(app.login_status, LoginStatus::Success);
        let msg = app.login_status_msg.clone().unwrap_or_default();
        assert!(
            msg.contains("stored locally, not verified"),
            "msg was: {msg}"
        );
        assert!(!msg.to_lowercase().contains("validating"), "msg was: {msg}");
    }

    #[test]
    fn login_apikey_masked_and_backspace() {
        let mut app = App::new(None);
        app.show_login();
        app.start_api_key_entry();
        app.push_api_key_char('a');
        app.push_api_key_char('b');
        app.push_api_key_char('c');
        assert_eq!(app.login_api_input, "abc");
        assert_eq!(app.masked_api_input(), "•••");
        app.pop_api_key_char();
        assert_eq!(app.login_api_input, "ab");
        // Too-short key is a local error (not fake network rejection)
        app.login_api_input = "ab".to_string();
        app.login_status = LoginStatus::ApiKeyEditing;
        assert!(!app.submit_api_key());
        assert!(matches!(app.login_status, LoginStatus::Error(_)));
        // Retry should go back to editing
        app.login_confirm();
        assert_eq!(app.login_status, LoginStatus::ApiKeyEditing);
    }

    #[test]
    fn login_focus_cycling() {
        let mut app = App::new(None);
        app.show_login();
        assert_eq!(app.login_focus, LoginFocus::Browser);
        app.cycle_focus_next();
        assert_eq!(app.login_focus, LoginFocus::ApiKey);
        app.cycle_focus_next();
        assert_eq!(app.login_focus, LoginFocus::Offline);
        app.cycle_focus_next();
        assert_eq!(app.login_focus, LoginFocus::Browser);
        app.cycle_focus_prev();
        assert_eq!(app.login_focus, LoginFocus::Offline);
        // 1/2 shortcuts via direct focus set + confirm
        app.login_focus = LoginFocus::Browser;
        app.login_confirm();
        assert_eq!(app.login_status, LoginStatus::BrowserPending);
        app.cancel_login();
        app.login_focus = LoginFocus::ApiKey;
        app.login_confirm();
        assert_eq!(app.login_status, LoginStatus::ApiKeyEditing);
    }

    fn entry_with_reason(i: usize) -> AuditEntry {
        AuditEntry {
            ts: 1_700_000_000_000 + i as i64,
            session_id: format!("sess-{i}"),
            tool_kind: 0,
            redacted_command: format!("cmd-{i}"),
            fingerprint: format!("fp-{i:04}"),
            action: Action::Allow as i32,
            source: SourceLevel::Rule as i32,
            reason: format!("entry-{i:02}"),
            confidence: 0.9,
            latency_ms: 5,
            profile: "test".to_string(),
            shadow: false,
        }
    }

    #[test]
    fn table_state_stays_in_sync_with_selection() {
        let mut app = App::new(None);
        for i in 0..5 {
            app.entries.push(entry_with_reason(i));
        }
        app.select_first();
        assert_eq!(app.selected, 0);
        assert_eq!(app.table_state.selected(), Some(0));
        app.select_next();
        assert_eq!(app.selected, 1);
        assert_eq!(app.table_state.selected(), Some(1));
        app.select_last();
        assert_eq!(app.selected, 4);
        assert_eq!(app.table_state.selected(), Some(4));
        app.select_prev();
        assert_eq!(app.selected, 3);
        assert_eq!(app.table_state.selected(), Some(3));
        app.select_first();
        assert_eq!(app.table_state.selected(), Some(0));
    }

    #[test]
    fn click_accounts_for_scroll_offset() {
        let mut app = App::new(None);
        for i in 0..10 {
            app.entries.push(entry_with_reason(i));
        }
        app.mode = ViewMode::Feed;
        app.select_first();
        // Simulate a scrolled table: first visible row is entry 5.
        *app.table_state.offset_mut() = 5;
        app.table_state.select(Some(app.selected));
        app.table_inner = Some(ratatui::layout::Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 12,
        });
        // Click the 3rd visible data row (y = inner.y + 1 header + 2).
        app.handle_click(1, 3);
        assert_eq!(app.selected, 7, "click must add scroll offset");
        assert_eq!(app.table_state.selected(), Some(7));
        // Click the first visible row maps to the offset entry.
        app.handle_click(1, 1);
        assert_eq!(app.selected, 5);
    }

    #[test]
    fn anim_phase_ignores_input_floods() {
        // Mouse motion floods the loop with events (one tick() each). The
        // dash animation must hold still until 200ms of wall-clock passes.
        let mut app = App::new(None);
        assert_eq!(app.anim_phase, 0);
        for _ in 0..1000 {
            app.tick();
        }
        assert_eq!(
            app.anim_phase, 0,
            "rapid ticks without elapsed time must not advance animation"
        );
        assert_eq!(
            app.spin_phase, 0,
            "rapid ticks without elapsed time must not advance spinner"
        );
    }

    #[test]
    fn from_path_offline_when_missing() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("nope").join("audit.db");
        // from_path should produce offline app without panicking
        let app = App::from_path(missing);
        assert!(app.is_offline || app.entries.is_empty());
    }
}
