//! `algo-tui` — ratatui terminal UI for `algorithco guard`.
//!
//! Read-only over `~/.algo/audit.db` via `AuditStore`. Never writes to the
//! decision path and never dials `~/.algo/algo.sock` (so a crash cannot block
//! hooks). Offline-capable: if the daemon is down or DB missing we show a
//! read-only banner and render whatever cached data is available.
//!
//! Mouse-first flow: click Sign in / Continue offline, click Feed/Policy tabs,
//! click a row to inspect, scroll to move, ✕ to quit.
//! Keyboard: Tab/1/2/Enter/Esc on login (masked API key input), q/j/k/g/G/r/p/1/2/Esc inside.
//!
//! Design tokens: `plans/design-tokens.md` Variant 1 — brand blue, allow green,
//! ask yellow, deny red. See `ui.rs` for RGB values.

mod app;
mod ui;

use app::{App, LoginFocus, LoginStatus, ViewMode};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, path::PathBuf, time::Duration};

/// Default audit DB path: `$ALGO_HOME/.algo/audit.db` else `~/.algo/audit.db`.
fn default_audit_path() -> PathBuf {
    if let Ok(v) = std::env::var("ALGO_HOME") {
        if !v.trim().is_empty() {
            return PathBuf::from(v).join(".algo").join("audit.db");
        }
    }
    if let Some(home) = dirs::home_dir() {
        home.join(".algo").join("audit.db")
    } else {
        PathBuf::from(".algo").join("audit.db")
    }
}

fn build_app() -> App {
    let path = default_audit_path();
    let mut app = if path.exists() {
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
    };
    // Login gate first (two-path auth). Tests use App::new directly (Feed).
    app.show_login();
    app
}

fn run_tui(mut app: App) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Ensure we restore terminal on early return/panic paths by using a scope.
    let result: io::Result<()> = (|| {
        use std::time::Instant;
        let mut last_auto = Instant::now();
        loop {
            terminal.draw(|f| ui::render(f, &mut app))?;

            // Poll faster (100ms) so the login spinner animates smoothly.
            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Mouse(mouse) => match mouse.kind {
                        MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                            if app.handle_click(mouse.column, mouse.row) {
                                break;
                            }
                        }
                        MouseEventKind::ScrollUp if app.mode == ViewMode::Feed => {
                            app.select_prev();
                        }
                        MouseEventKind::ScrollDown if app.mode == ViewMode::Feed => {
                            app.select_next();
                        }
                        _ => {}
                    },
                    Event::Key(key) => {
                        // Login gate: full keyboard navigation with masked input
                        if app.mode == ViewMode::Login {
                            match key.code {
                                KeyCode::Esc => {
                                    match app.login_status {
                                        LoginStatus::Idle => {
                                            // Fallback to offline from idle
                                            app.continue_offline();
                                        }
                                        LoginStatus::BrowserPending
                                        | LoginStatus::ApiKeyEditing
                                        | LoginStatus::ApiKeyValidating
                                        | LoginStatus::Error(_) => {
                                            app.cancel_login();
                                        }
                                        LoginStatus::Success => {
                                            app.continue_offline();
                                        }
                                    }
                                }
                                KeyCode::Enter => {
                                    app.login_confirm();
                                }
                                KeyCode::Tab => {
                                    app.cycle_focus_next();
                                }
                                KeyCode::BackTab => {
                                    app.cycle_focus_prev();
                                }
                                KeyCode::Backspace => {
                                    if matches!(
                                        app.login_status,
                                        LoginStatus::ApiKeyEditing | LoginStatus::Error(_)
                                    ) || (app.login_status == LoginStatus::Idle
                                        && app.login_focus == LoginFocus::ApiKey)
                                    {
                                        app.pop_api_key_char();
                                    }
                                }
                                KeyCode::Char('1') => {
                                    // Quick shortcut [1] — if editing, treat as input char '1'
                                    if matches!(
                                        app.login_status,
                                        LoginStatus::ApiKeyEditing | LoginStatus::Error(_)
                                    ) {
                                        app.push_api_key_char('1');
                                    } else if matches!(
                                        app.login_status,
                                        LoginStatus::BrowserPending
                                            | LoginStatus::ApiKeyValidating
                                            | LoginStatus::Success
                                    ) {
                                        // ignore during pending/validating/success
                                    } else {
                                        app.login_focus = LoginFocus::Browser;
                                        // If already on browser idle, Enter would start — but 1 is also quick-select, so start immediately
                                        if app.login_status == LoginStatus::Idle {
                                            app.start_browser_signin();
                                        }
                                    }
                                }
                                KeyCode::Char('2') => {
                                    if matches!(
                                        app.login_status,
                                        LoginStatus::ApiKeyEditing | LoginStatus::Error(_)
                                    ) {
                                        app.push_api_key_char('2');
                                    } else if matches!(
                                        app.login_status,
                                        LoginStatus::BrowserPending
                                            | LoginStatus::ApiKeyValidating
                                            | LoginStatus::Success
                                    ) {
                                    } else {
                                        app.login_focus = LoginFocus::ApiKey;
                                        if app.login_status == LoginStatus::Idle {
                                            app.start_api_key_entry();
                                        }
                                    }
                                }
                                KeyCode::Char('o') | KeyCode::Char('O') => {
                                    if matches!(
                                        app.login_status,
                                        LoginStatus::ApiKeyEditing | LoginStatus::Error(_)
                                    ) {
                                        // typing 'o' into API key should go to input, not offline
                                        app.push_api_key_char(
                                            if key.modifiers.contains(KeyModifiers::SHIFT) {
                                                'O'
                                            } else {
                                                'o'
                                            },
                                        );
                                    } else if matches!(
                                        app.login_status,
                                        LoginStatus::BrowserPending
                                            | LoginStatus::ApiKeyValidating
                                            | LoginStatus::Success
                                    ) {
                                        // ignore
                                    } else {
                                        app.continue_offline();
                                    }
                                }
                                KeyCode::Char('q') | KeyCode::Char('Q') => {
                                    if matches!(
                                        app.login_status,
                                        LoginStatus::ApiKeyEditing | LoginStatus::Error(_)
                                    ) {
                                        // 'q' is a valid key char when editing — don't quit, input it
                                        // Only quit if not editing (de-emphasized quit still works on idle/pending)
                                        let c = if key.modifiers.contains(KeyModifiers::SHIFT) {
                                            'Q'
                                        } else {
                                            'q'
                                        };
                                        app.push_api_key_char(c);
                                    } else {
                                        break;
                                    }
                                }
                                KeyCode::Char(c) => {
                                    // Typing into API key when focused or editing
                                    if matches!(
                                        app.login_status,
                                        LoginStatus::ApiKeyEditing | LoginStatus::Error(_)
                                    ) || (app.login_status == LoginStatus::Idle
                                        && app.login_focus == LoginFocus::ApiKey)
                                    {
                                        // Filter out control combos, but allow most printable
                                        if !key.modifiers.contains(KeyModifiers::CONTROL)
                                            && !key.modifiers.contains(KeyModifiers::ALT)
                                        {
                                            app.push_api_key_char(c);
                                        }
                                    } else if c == 'k' || c == 'j' {
                                        // ignore nav keys on login
                                    }
                                }
                                _ => {}
                            }
                        } else {
                            match key.code {
                                KeyCode::Char('q') => break,
                                KeyCode::Char('j') | KeyCode::Down => app.select_next(),
                                KeyCode::Char('k') | KeyCode::Up => app.select_prev(),
                                KeyCode::Char('g') | KeyCode::Home => app.select_first(),
                                KeyCode::Char('G') | KeyCode::End => app.select_last(),
                                KeyCode::Char('r') => {
                                    app.refresh();
                                    last_auto = Instant::now();
                                }
                                KeyCode::Char('p') => {
                                    if app.mode == ViewMode::Feed {
                                        app.show_policy();
                                    } else if app.mode == ViewMode::Policy {
                                        app.show_feed();
                                    }
                                }
                                KeyCode::Char('1') => {
                                    app.show_feed();
                                }
                                KeyCode::Char('2') => {
                                    app.show_policy();
                                }
                                KeyCode::Esc => {
                                    if app.mode == ViewMode::Policy {
                                        app.show_feed();
                                    } else {
                                        break;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            app.tick();
            // Auto-refresh live feed every 2s (read-only audit.db, cheap WAL read).
            if last_auto.elapsed() >= Duration::from_secs(2) {
                if app.mode != ViewMode::Login {
                    app.refresh();
                }
                last_auto = Instant::now();
            }
        }
        Ok(())
    })();

    // Restore terminal even if inner loop errored.
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
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
