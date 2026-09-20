//! `algo-tui` — ratatui terminal UI for `algorithco guard`.
//!
//! Read-only over `~/.algo/audit.db` via `AuditStore`. Never writes to the
//! decision path and never dials `~/.algo/algo.sock` (so a crash cannot block
//! hooks). Offline-capable: if the daemon is down or DB missing we show a
//! read-only banner and render whatever cached data is available.
//!
//! Keyboard-only flow: `q` quit, `j/k` or `↑/↓` navigation, `r` refresh, `p` toggle policy.
//!
//! Design tokens: `plans/design-tokens.md` Variant 1 — brand blue, allow green,
//! ask yellow, deny red. See `ui.rs` for RGB values.

mod app;
mod ui;

use app::{App, ViewMode};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, path::PathBuf, time::Duration};

/// Default audit DB path: `~/.algo/audit.db`.
fn default_audit_path() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        home.join(".algo").join("audit.db")
    } else {
        PathBuf::from(".algo").join("audit.db")
    }
}

fn build_app() -> App {
    let path = default_audit_path();
    if path.exists() {
        // Try to open; on any error fall back to offline mode — still render.
        match algo_audit::AuditStore::open(&path) {
            Ok(store) => {
                // `init` is idempotent; ignore error — DB may already be valid.
                let _ = store.init();
                let mut app = App::new(Some(store));
                app.refresh();
                app
            }
            Err(e) => {
                let mut app = App::new(None);
                app.is_offline = true;
                app.error = Some(e.to_string());
                app
            }
        }
    } else {
        let mut app = App::new(None);
        app.is_offline = true;
        app.error = Some(format!("audit.db not found at {}", path.display()));
        // Still attempt to create a store handle pointing at the path so that
        // a later `r` refresh after daemon creates the DB can succeed without restart.
        if let Ok(store) = algo_audit::AuditStore::open(&path) {
            app.store = Some(store);
            // Do not clear offline flag — until refresh succeeds we remain offline.
        }
        app
    }
}

fn run_tui(mut app: App) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Ensure we restore terminal on early return/panic paths by using a scope.
    let result: io::Result<()> = (|| {
        loop {
            terminal.draw(|f| ui::render(f, &app))?;

            // Poll with timeout so UI stays responsive and we can handle resize.
            if event::poll(Duration::from_millis(250))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('j') | KeyCode::Down => app.select_next(),
                        KeyCode::Char('k') | KeyCode::Up => app.select_prev(),
                        KeyCode::Char('r') => app.refresh(),
                        KeyCode::Char('p') => app.toggle_policy(),
                        KeyCode::Esc => {
                            if app.mode == ViewMode::Policy {
                                app.toggle_policy();
                            } else {
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    })();

    // Restore terminal even if inner loop errored.
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn main() -> io::Result<()> {
    // Tokio is a declared dependency per spec (future async audit tail / tokio::fs).
    // Current loop is sync via crossterm; we keep the dep without needing a runtime
    // yet. The TUI remains read-only over audit.db and never dials ~/.algo/algo.sock
    // so a crash cannot block hooks.
    let app = build_app();
    run_tui(app)
}
