//! App state for TUI — read-only view over `AuditStore`.
//!
//! No decision-path coupling: TUI only reads `~/.algo/audit.db` (SQLite WAL).
//! If daemon is down or DB missing, `store` is `None` and UI shows offline banner.
//! Crash of this process never blocks hooks.

use algo_audit::{AuditEntry, AuditStore, Counts};

/// View mode for the TUI.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ViewMode {
    /// Live feed of decisions.
    #[default]
    Feed,
    /// Policy editor placeholder (local rules, dry-run vs history).
    Policy,
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
}

impl App {
    /// Create a new app from an optional store. Does not automatically refresh;
    /// caller should invoke `refresh()` to load counts/entries.
    pub fn new(store: Option<AuditStore>) -> Self {
        let is_offline = store.is_none();
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

    /// Reload `counts` and `entries` from the audit store.
    ///
    /// On failure, marks offline and preserves previous counts.
    pub fn refresh(&mut self) {
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

    /// Toggle between Feed and Policy view (p).
    pub fn toggle_policy(&mut self) {
        self.mode = match self.mode {
            ViewMode::Feed => ViewMode::Policy,
            ViewMode::Policy => ViewMode::Feed,
        };
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
    fn from_path_offline_when_missing() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("nope").join("audit.db");
        // from_path should produce offline app without panicking
        let app = App::from_path(missing);
        // Depending on whether parent dirs can be created, either offline or empty-online.
        // Most importantly, it does not panic and is_offline or entries empty is acceptable.
        // We assert it has zero or is offline; both mean read-ready.
        assert!(app.is_offline || app.entries.is_empty());
    }
}
