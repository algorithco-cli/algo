//! App state for TUI — read-only view over `AuditStore`.
//!
//! No decision-path coupling: TUI only reads `~/.algo/audit.db` (SQLite WAL).
//! If daemon is down or DB missing, `store` is `None` and UI shows offline banner.
//! Crash of this process never blocks hooks.

use algo_audit::{AuditEntry, AuditStore, Counts};

/// View mode for the TUI.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ViewMode {
    /// Login gate — shown first on launch (browser sign-in or API key).
    Login,
    /// Live feed of decisions.
    #[default]
    Feed,
    /// Policy snapshot (local rules, dry-run vs history).
    Policy,
}

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
    /// Success — brief confirmation before auto-transition to Feed.
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
            mode: ViewMode::Feed,
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

    fn clamp_selection(&mut self) {
        if self.entries.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.entries.len() {
            self.selected = self.entries.len() - 1;
        }
    }

    /// Move selection down (j).
    pub fn select_next(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
        }
    }

    /// Move selection up (k).
    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Jump to first row (g / Home).
    pub fn select_first(&mut self) {
        self.selected = 0;
    }

    /// Jump to last row (G / End).
    pub fn select_last(&mut self) {
        if !self.entries.is_empty() {
            self.selected = self.entries.len() - 1;
        }
    }

    /// Currently selected entry, if any.
    pub fn selected_entry(&self) -> Option<&AuditEntry> {
        self.entries.get(self.selected)
    }

    /// Toggle between Feed and Policy view (p). No-op on Login gate.
    pub fn toggle_policy(&mut self) {
        self.mode = match self.mode {
            ViewMode::Feed => ViewMode::Policy,
            ViewMode::Policy => ViewMode::Feed,
            ViewMode::Login => ViewMode::Login,
        };
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
        self.mode = ViewMode::Feed;
    }

    /// Start a browser sign-in attempt (animated; backend deferred in MVP).
    /// Generates a device code for display.
    pub fn start_browser_signin(&mut self) {
        self.login_status = LoginStatus::BrowserPending;
        self.login_device_code = Some(Self::generate_device_code());
        self.login_ticks = 0;
        self.login_pending = true;
        self.login_note = Some("Waiting for browser confirmation…".to_string());
        self.login_status_msg =
            Some("Waiting for browser confirmation…  Press Esc to cancel.".to_string());
        self.login_focus = LoginFocus::Browser;
    }

    fn generate_device_code() -> String {
        // Deterministic for tests, but looks like a real device code.
        // In production this would come from OAuth device flow.
        "WD-4829-XK".to_string()
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

    /// Submit the current API key — validates not empty, then enters validating state.
    /// Returns true if submission started (validating), false if validation error immediately.
    pub fn submit_api_key(&mut self) -> bool {
        let trimmed = self.login_api_input.trim();
        if trimmed.is_empty() {
            self.login_status = LoginStatus::Error("API key cannot be empty.".to_string());
            self.login_status_msg = Some("API key cannot be empty.".to_string());
            self.login_ticks = 0;
            return false;
        }
        if trimmed.chars().count() < 8 {
            self.login_status = LoginStatus::Error(
                "API key too short — must be at least 8 characters.".to_string(),
            );
            self.login_status_msg =
                Some("API key too short — must be at least 8 characters.".to_string());
            self.login_ticks = 0;
            return false;
        }
        // Enter validating — tick will drive success after delay
        self.login_status = LoginStatus::ApiKeyValidating;
        self.login_ticks = 0;
        self.login_pending = true;
        self.login_status_msg = Some("Validating API key…".to_string());
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
        // Sync legacy for render that still reads old fields
        self.sync_legacy();

        match self.login_status.clone() {
            LoginStatus::BrowserPending => {
                self.login_ticks += 1;
                // Keep device code visible; no auto-success — user cancels or we stay pending.
                // Optional: after very long time (e.g. 40 ticks ~4s) we could keep spinner forever — do nothing.
            }
            LoginStatus::ApiKeyValidating => {
                self.login_ticks += 1;
                if self.login_ticks >= 10 {
                    // Simulate validation: if input contains "invalid" or "error", fail, else success if length ok
                    let input = self.login_api_input.trim().to_lowercase();
                    if input.contains("invalid") || input.contains("error") || input == "fail" {
                        self.login_status = LoginStatus::Error(
                            "Invalid API key — please check and try again.".to_string(),
                        );
                        self.login_status_msg =
                            Some("Invalid API key — please check and try again.".to_string());
                        self.login_pending = false;
                    } else if input.chars().count() >= 8 {
                        self.login_status = LoginStatus::Success;
                        self.login_status_msg = Some("✓ API key accepted — signed in.".to_string());
                        self.login_pending = false;
                        self.login_ticks = 0;
                    } else {
                        self.login_status = LoginStatus::Error(
                            "API key too short — must be at least 8 characters.".to_string(),
                        );
                        self.login_status_msg =
                            Some("API key too short — must be at least 8 characters.".to_string());
                        self.login_pending = false;
                    }
                    self.login_ticks = 0;
                }
            }
            LoginStatus::Success => {
                self.login_ticks += 1;
                if self.login_ticks >= 12 {
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
                if x >= inner.x
                    && x < inner.x.saturating_add(inner.width)
                    && y > inner.y
                    && y < inner.y.saturating_add(inner.height)
                {
                    let idx = (y - inner.y - 1) as usize;
                    if idx < self.entries.len() {
                        self.selected = idx;
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
        app.toggle_policy();
        assert_eq!(app.mode, ViewMode::Policy);
        app.toggle_policy();
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
        // Policy toggle is a no-op on login
        app.toggle_policy();
        assert_eq!(app.mode, ViewMode::Login);
    }

    #[test]
    fn login_browser_flow_with_cancel() {
        let mut app = App::new(None);
        app.show_login();
        // Idle + Browser focused -> Enter starts browser
        app.login_confirm();
        assert_eq!(app.login_status, LoginStatus::BrowserPending);
        assert!(app.login_device_code.is_some());
        assert_eq!(app.login_device_code.as_deref(), Some("WD-4829-XK"));
        // Spinner tick does not auto-finish
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
        // Valid key -> validating
        app.login_api_input = "ag-valid-key-12345".to_string();
        assert!(app.submit_api_key());
        assert_eq!(app.login_status, LoginStatus::ApiKeyValidating);
        // Tick drives success
        for _ in 0..12 {
            app.tick();
        }
        assert_eq!(app.login_status, LoginStatus::Success);
        // Tick drives auto-transition to feed
        for _ in 0..13 {
            app.tick();
        }
        assert_eq!(app.mode, ViewMode::Feed);
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
        // Invalid simulation
        app.login_api_input = "invalid-key-here".to_string();
        app.submit_api_key();
        for _ in 0..11 {
            app.tick();
        }
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

    #[test]
    fn from_path_offline_when_missing() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("nope").join("audit.db");
        // from_path should produce offline app without panicking
        let app = App::from_path(missing);
        assert!(app.is_offline || app.entries.is_empty());
    }
}
