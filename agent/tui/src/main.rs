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
mod dog;
mod theme;
mod ui;

use app::{App, LoginFocus, LoginStatus, ViewMode};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dog::ColorMode;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, path::PathBuf, time::Duration};

/// Parsed `algo-tui` flags. `--color` pins the login theme tier
/// (default `auto`: `NO_COLOR` → mono, `COLORTERM` → truecolor, else ansi16).
/// `--debug` shows tier/tick diagnostics on the login footer.
struct TuiArgs {
    color: Option<ColorMode>,
    debug: bool,
}

impl std::fmt::Debug for TuiArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TuiArgs")
            .field("color", &self.color)
            .field("debug", &self.debug)
            .finish()
    }
}

fn parse_color_mode(s: &str) -> Option<ColorMode> {
    match s.to_ascii_lowercase().as_str() {
        "truecolor" | "true-color" | "24bit" => Some(ColorMode::TrueColor),
        "ansi16" | "ansi-16" | "16" => Some(ColorMode::Ansi16),
        "mono" | "no-color" | "nocolor" => Some(ColorMode::Mono),
        "auto" => None,
        _ => None,
    }
}

fn parse_tui_args<I>(args: I) -> Result<TuiArgs, String>
where
    I: IntoIterator<Item = String>,
{
    let mut out = TuiArgs {
        color: None,
        debug: false,
    };
    let mut args = args.into_iter().peekable();
    // Skip argv[0] (binary path) when it doesn't look like a flag.
    if let Some(first) = args.peek() {
        if !first.starts_with('-') {
            args.next();
        }
    }
    while let Some(arg) = args.next() {
        if let Some(value) = arg.strip_prefix("--color=") {
            out.color = parse_color_mode(value);
            if arg == "--color=" || parse_color_mode(value).is_none() && value != "auto" {
                return Err(format!(
                    "unknown --color '{value}' (truecolor|ansi16|mono|auto)"
                ));
            }
        } else if arg == "--color" {
            match args.next() {
                Some(value) => {
                    if parse_color_mode(&value).is_none() && value != "auto" {
                        return Err(format!(
                            "unknown --color '{value}' (truecolor|ansi16|mono|auto)"
                        ));
                    }
                    out.color = parse_color_mode(&value);
                }
                None => return Err("--color needs a value (truecolor|ansi16|mono|auto)".into()),
            }
        } else if arg == "--debug" {
            out.debug = true;
        } else if arg == "--help" || arg == "-h" {
            return Err("__help__".into());
        } else {
            return Err(format!("unknown argument '{arg}' (see --help)"));
        }
    }
    Ok(out)
}

fn print_help() {
    println!(
        "\
algo-tui — terminal UI for algorithco guard (read-only over ~/.algo/audit.db)

USAGE:
    algo-tui [--color truecolor|ansi16|mono|auto] [--debug]

OPTIONS:
    --color <mode>  Pin the color tier. `auto` (default) detects:
                    NO_COLOR -> mono, COLORTERM=truecolor/24bit -> truecolor,
                    otherwise ansi16. An explicit tier wins over detection.
    --debug         Show tier/tick diagnostics on the login footer.
    -h, --help      Show this help.

KEYS (login):
    Up/Down, Tab   Move between cards        1/2/o   Shortcuts
    Enter          Activate focused card     ?       Help overlay
    Esc            Cancel, collapse, or quit"
    );
}
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
    // Panic hook: restore raw mode before the default hook prints, so a
    // panic never leaves the terminal hijacked (research: ratatui init recipe).
    // LeaveAlternateScreen is handled on the normal/error paths below; raw
    // mode is the critical bit inside the hook (no stdout handle available).
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        orig_hook(info);
    }));
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

            // Cap redraw at ~30 FPS: animations (80ms spinner, dog wag) stay
            // smooth, input is never blocked behind a long poll.
            if event::poll(Duration::from_millis(33))? {
                match event::read()? {
                    Event::Mouse(mouse) => match mouse.kind {
                        MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                            if app.handle_click(mouse.column, mouse.row) {
                                break;
                            }
                        }
                        MouseEventKind::ScrollUp => {
                            // Research: wheel maps to selection, not raw offset.
                            if app.mode == ViewMode::Connect {
                                app.select_tool_prev();
                            } else if app.mode == ViewMode::Feed {
                                app.select_prev();
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            if app.mode == ViewMode::Connect {
                                app.select_tool_next();
                            } else if app.mode == ViewMode::Feed {
                                app.select_next();
                            }
                        }
                        // Research: ignore Moved/Drag/Up storms — render only
                        // on Render ticks, not per mouse event (see crossterm#964).
                        _ => {}
                    },
                    Event::Resize(_, _) => {
                        // Research: ignore payload, recompute from frame.area()
                        // on next draw — nothing to do here, just redraw.
                    }
                    Event::Key(key) => {
                        // Research: Crossterm emits Press/Repeat/Release (esp.
                        // Windows) — handle Press only to avoid duplicates.
                        if key.kind != KeyEventKind::Press {
                            continue;
                        }
                        // Global Ctrl-C always quits (all views).
                        if key.code == KeyCode::Char('c')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            break;
                        }
                        // Global `?` toggles help — except while typing an API
                        // key (where `?` is valid input).
                        let typing_key = app.mode == ViewMode::Login
                            && matches!(
                                app.login_status,
                                LoginStatus::ApiKeyEditing | LoginStatus::Error(_)
                            );
                        if key.code == KeyCode::Char('?') && !typing_key {
                            app.toggle_help();
                            continue;
                        }
                        // Esc closes help first on every view.
                        if key.code == KeyCode::Esc && app.close_help() {
                            continue;
                        }
                        // Login gate: the card itself is the button.
                        if app.mode == ViewMode::Login {
                            match key.code {
                                KeyCode::Esc => {
                                    match app.login_status {
                                        LoginStatus::Idle => {
                                            // Nothing to cancel — Esc quits (see footer).
                                            break;
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
                                KeyCode::Up => {
                                    // Arrow-key card select (Idle only; never
                                    // steals keys while typing).
                                    if app.login_status == LoginStatus::Idle {
                                        app.cycle_focus_prev();
                                    }
                                }
                                KeyCode::Down => {
                                    if app.login_status == LoginStatus::Idle {
                                        app.cycle_focus_next();
                                    }
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
                                    // Guard-dog toys: [b]ark + [m] color-cycle when
                                    // NOT typing a key (typing takes precedence).
                                    if (c == 'b' || c == 'B' || c == 'm' || c == 'M')
                                        && !matches!(
                                            app.login_status,
                                            LoginStatus::ApiKeyEditing | LoginStatus::Error(_)
                                        )
                                        && !matches!(
                                            app.login_status,
                                            LoginStatus::BrowserPending
                                                | LoginStatus::ApiKeyValidating
                                                | LoginStatus::Success
                                        )
                                    {
                                        if c == 'b' || c == 'B' {
                                            app.bark_dog();
                                        } else {
                                            app.cycle_dog_color();
                                        }
                                        continue;
                                    }
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
                        } else if app.mode == ViewMode::Connect {
                            // CLI-integration picker: vim j/k + arrows, g/G +
                            // Home/End, 1-4 jump, Enter choose, r refresh,
                            // ? help, q/Esc quit (research: lazygit/k9s parity).
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => break,
                                KeyCode::Char('j') | KeyCode::Down => {
                                    app.select_tool_next();
                                }
                                KeyCode::Char('k') | KeyCode::Up => {
                                    app.select_tool_prev();
                                }
                                KeyCode::Home => {
                                    app.tool_selected = 0;
                                }
                                KeyCode::End => {
                                    app.tool_selected = crate::app::CLI_TOOL_COUNT - 1;
                                }
                                KeyCode::Enter => {
                                    app.choose_tool();
                                }
                                KeyCode::Char('1') => {
                                    app.choose_tool_idx(0);
                                }
                                KeyCode::Char('2') => {
                                    app.choose_tool_idx(1);
                                }
                                KeyCode::Char('3') => {
                                    app.choose_tool_idx(2);
                                }
                                KeyCode::Char('4') => {
                                    app.choose_tool_idx(3);
                                }
                                KeyCode::Char('r') | KeyCode::Char('R') => {
                                    app.refresh();
                                }
                                _ => {}
                            }
                        } else {
                            // Feed/Policy: vim j/k/g/G + Home/End + PgUp/PgDn
                            // + Ctrl-d/u page, r refresh, ? help, q/Esc quit
                            // (research: ratatui table example + lazygit).
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => break,
                                KeyCode::Char('j') | KeyCode::Down => app.select_next(),
                                KeyCode::Char('k') | KeyCode::Up => app.select_prev(),
                                KeyCode::Char('g') => app.select_first(),
                                KeyCode::Char('G') => app.select_last(),
                                KeyCode::Home => app.select_first(),
                                KeyCode::End => app.select_last(),
                                KeyCode::PageDown => app.select_page_down(),
                                KeyCode::PageUp => app.select_page_up(),
                                KeyCode::Char('d')
                                    if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
                                    app.select_page_down()
                                }
                                KeyCode::Char('u')
                                    if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
                                    app.select_page_up()
                                }
                                KeyCode::Char('r') | KeyCode::Char('R') => app.refresh(),
                                KeyCode::Char('1') => app.set_mode(ViewMode::Feed),
                                KeyCode::Char('2') => app.set_mode(ViewMode::Policy),
                                KeyCode::Char('p') | KeyCode::Char('P') => {
                                    app.set_mode(ViewMode::Policy)
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
    let args = match parse_tui_args(std::env::args()) {
        Ok(args) => args,
        Err(e) if e == "__help__" => {
            print_help();
            return Ok(());
        }
        Err(e) => {
            eprintln!("algo-tui: {e}");
            eprintln!("Try 'algo-tui --help'.");
            std::process::exit(2);
        }
    };
    let mut app = build_app();
    // Explicit `--color` wins over `ColorMode::detect()` (NO_COLOR/COLORTERM).
    if let Some(layer) = args.color {
        app.set_color_layer(layer);
    }
    app.debug = args.debug;
    run_tui(app)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn color_flag_forms() {
        for value in ["truecolor", "ansi16", "ansi-16", "mono"] {
            let a = parse_tui_args(args(&["algo-tui", "--color", value])).unwrap();
            assert!(a.color.is_some(), "{value}");
            let b = parse_tui_args(args(&["algo-tui", &format!("--color={value}")])).unwrap();
            assert_eq!(a.color, b.color, "{value}");
        }
        assert_eq!(
            parse_tui_args(args(&["algo-tui", "--color", "truecolor"]))
                .unwrap()
                .color,
            Some(ColorMode::TrueColor)
        );
        assert_eq!(
            parse_tui_args(args(&["algo-tui", "--color", "ansi16"]))
                .unwrap()
                .color,
            Some(ColorMode::Ansi16)
        );
        assert_eq!(
            parse_tui_args(args(&["algo-tui", "--color", "mono"]))
                .unwrap()
                .color,
            Some(ColorMode::Mono)
        );
        // `auto` (and no flag) means detect, not pin.
        assert!(parse_tui_args(args(&["algo-tui"])).unwrap().color.is_none());
        assert!(parse_tui_args(args(&["algo-tui", "--color", "auto"]))
            .unwrap()
            .color
            .is_none());
    }

    #[test]
    fn debug_and_errors() {
        assert!(
            parse_tui_args(args(&["algo-tui", "--debug"]))
                .unwrap()
                .debug
        );
        assert!(!parse_tui_args(args(&["algo-tui"])).unwrap().debug);
        assert!(parse_tui_args(args(&["algo-tui", "--color", "neon"])).is_err());
        assert!(parse_tui_args(args(&["algo-tui", "--color"])).is_err());
        assert!(parse_tui_args(args(&["algo-tui", "--frobnicate"])).is_err());
        assert_eq!(
            parse_tui_args(args(&["algo-tui", "--help"])).unwrap_err(),
            "__help__"
        );
    }
}
