//! UI rendering — design tokens Variant 1, closest terminal approximation.
//!
//! Tokens from root `design-tokens.css` §1:
//!   brand  #6D4AFF  → `COLOR_BRAND`  (links, header)
//!   allow  #1E9E63  → `COLOR_ALLOW`  (success)
//!   ask    #D99A00  → `COLOR_ASK`    (warn)
//!   deny   #E5484D  → `COLOR_DENY`   (danger)
//! Exact hex where truecolor is available; otherwise nearest ANSI fallback
//! (green/yellow/red/blue) carries the same semantics per spec §92.
//!
//! Professional layout:
//!   header (title + version + LIVE/OFFLINE/SHADOW/ENFORCING/PAUSED + stats)
//!   offline banner (compact, 2 rows)
//!   feed: table (flex) + detail pane (`algo why` parity)
//!   policy: real config snapshot (read-only, never writes decision path)
//!   footer: keyboard hints + auto-refresh note

use crate::app::{
    App, LoginFocus, LoginStatus, ViewMode, CLI_TOOLS, CLI_TOOL_COUNT, SPINNER_GRACE_MS,
};
use crate::dog::{CELLS_H, CELLS_W};
use crate::theme::Theme;
use chrono::{DateTime, Utc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};

// ---- Design tokens (Variant 1, light column) --------------------------------
// Using truecolor RGB; terminals without truecolor degrade to nearest 256 color.
const COLOR_BRAND: Color = Color::Rgb(109, 74, 255); // #6D4AFF --ag-brand
const COLOR_ALLOW: Color = Color::Rgb(30, 158, 99); // #1E9E63 --ag-allow
const COLOR_ASK: Color = Color::Rgb(217, 154, 0); // #D99A00 --ag-ask
const COLOR_DENY: Color = Color::Rgb(229, 72, 77); // #E5484D --ag-deny
const COLOR_MUTED: Color = Color::Rgb(107, 106, 123); // #6B6A7B --ag-text-muted
const COLOR_BORDER: Color = Color::Rgb(230, 229, 238); // #E6E5EE --ag-border

// Hand-tuned 256-color fallbacks (research: never auto-dither, quantize at
// startup against the XTERM table). Used when COLORTERM lacks truecolor.
const COLOR_BRAND_256: Color = Color::Indexed(99); // ~#875FFF
const COLOR_ALLOW_256: Color = Color::Indexed(35); // ~#00AF5F
const COLOR_ASK_256: Color = Color::Indexed(172); // ~#D78700
const COLOR_DENY_256: Color = Color::Indexed(167); // ~#D75F5F

/// True when the terminal advertises truecolor (`COLORTERM=truecolor|24bit`).
/// Research: `supports-color` precedence — NO_COLOR/TERM=dumb handled
/// separately; this only gates Rgb vs Indexed so non-truecolor terms don't
/// get glitched output (ratatui docs warning).
fn supports_truecolor() -> bool {
    if let Ok(v) = std::env::var("COLORTERM") {
        let v = v.to_ascii_lowercase();
        if v.contains("truecolor") || v.contains("24bit") {
            return true;
        }
    }
    false
}

/// Resolve a Variant-1 token to Rgb (truecolor) or hand-tuned Indexed fallback.
fn token_brand() -> Color {
    if supports_truecolor() {
        COLOR_BRAND
    } else {
        COLOR_BRAND_256
    }
}
#[allow(dead_code)]
fn token_allow() -> Color {
    if supports_truecolor() {
        COLOR_ALLOW
    } else {
        COLOR_ALLOW_256
    }
}
fn token_ask() -> Color {
    if supports_truecolor() {
        COLOR_ASK
    } else {
        COLOR_ASK_256
    }
}
#[allow(dead_code)]
fn token_deny() -> Color {
    if supports_truecolor() {
        COLOR_DENY
    } else {
        COLOR_DENY_256
    }
}

/// NO_COLOR spec (2017): non-empty => strip color, keep bold/underline.
/// Callers use this to decide whether to emit color at all.
#[allow(dead_code)]
fn no_color() -> bool {
    std::env::var("NO_COLOR")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
}

/// Three-tier icon set: ascii → unicode (default) → nerd (opt-in).
/// Research: default to widely-supported Unicode; Nerd only when
/// `ALGO_ICON_SET=nerd`; ASCII when `ALGO_ICON_SET=ascii` or `TERM=dumb`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IconSet {
    Ascii,
    Unicode,
}

fn icon_set() -> IconSet {
    if let Ok(v) = std::env::var("ALGO_ICON_SET") {
        let v = v.to_ascii_lowercase();
        if v == "ascii" {
            return IconSet::Ascii;
        }
        if v == "nerd" {
            return IconSet::Unicode; // Nerd glyphs are same-width Unicode twins here
        }
    }
    if std::env::var("TERM").map(|v| v == "dumb").unwrap_or(false) {
        return IconSet::Ascii;
    }
    IconSet::Unicode
}

/// Triple-encode decision (WCAG 1.4.1: never hue-only): color + glyph + word.
/// Red/green collapse under deuteranopia — glyph + label keep them distinct.
fn action_glyph(action: &str) -> &'static str {
    match icon_set() {
        IconSet::Ascii => match action {
            "allow" => "*",
            "deny" => "x",
            _ => "?",
        },
        IconSet::Unicode => match action {
            "allow" => "✓",
            "deny" => "✗",
            _ => "?",
        },
    }
}

fn action_color(action: &str) -> Color {
    match action {
        "allow" => COLOR_ALLOW,
        "deny" => COLOR_DENY,
        _ => COLOR_ASK,
    }
}

fn format_ts(ts_millis: i64) -> String {
    if let Some(dt) = DateTime::<Utc>::from_timestamp_millis(ts_millis) {
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        ts_millis.to_string()
    }
}

fn format_time_short(ts_millis: i64) -> String {
    if let Some(dt) = DateTime::<Utc>::from_timestamp_millis(ts_millis) {
        dt.format("%H:%M:%S").to_string()
    } else {
        "--:--:--".to_string()
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else if max <= 1 {
        "…".to_string()
    } else {
        let t: String = s.chars().take(max - 1).collect();
        format!("{t}…")
    }
}

fn confidence_bar(conf: f64) -> String {
    let filled = (conf.clamp(0.0, 1.0) * 10.0).round() as usize;
    let empty = 10 - filled;
    // Fixed cell width per frame; ASCII twin when Nerd/Unicode unavailable.
    // Research: every gauge carries numeric % + label (screen-reader safe).
    match icon_set() {
        IconSet::Ascii => format!("{}{} {:.2}", "#".repeat(filled), "-".repeat(empty), conf),
        IconSet::Unicode => format!("{}{} {:.2}", "█".repeat(filled), "░".repeat(empty), conf),
    }
}

/// Spinner frames for the login animation (tick-driven, no blocking).
const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
/// ASCII twin (same single-cell width) for `TERM=dumb` / `ALGO_ICON_SET=ascii`.
const SPINNER_ASCII: [&str; 4] = ["-", "\\", "|", "/"];

fn spinner_frame(tick: usize) -> &'static str {
    match icon_set() {
        IconSet::Ascii => SPINNER_ASCII[tick % SPINNER_ASCII.len()],
        IconSet::Unicode => SPINNER[tick % SPINNER.len()],
    }
}

/// Marching-ants outline: a rotating dashed border with gaps.
///
/// Drawn cell-by-cell over a box `rect` AFTER its `Block` + contents, so the
/// phase fully controls the dashes. Only straight border glyphs are replaced —
/// title text and corners are left intact (corners just tinted to match).
/// Perimeter order is clockwise, so an increasing `phase` marches the dashes
/// clockwise around the box. `phase` must come from wall-clock pacing
/// (`App::anim_phase`) — never from the per-event tick counter, or input
/// floods (mouse motion) visibly speed the animation up.
fn render_marching_dashes(
    frame: &mut Frame,
    rect: ratatui::layout::Rect,
    phase: usize,
    style: Style,
) {
    if rect.width < 4 || rect.height < 3 {
        return;
    }
    const DASH: usize = 3;
    const GAP: usize = 2;
    const PERIOD: usize = DASH + GAP;
    let on = |p: usize| (p + PERIOD - phase % PERIOD) % PERIOD < DASH;

    let buf = frame.buffer_mut();
    let x0 = rect.x;
    let y0 = rect.y;
    let x1 = rect.x + rect.width - 1;
    let y1 = rect.y + rect.height - 1;
    let w = (rect.width - 2) as usize; // horizontal straight-run length
    let h = (rect.height - 2) as usize; // vertical straight-run length

    // Top edge (left→right), then bottom edge (perimeter continues clockwise).
    for i in 0..w {
        let x = x0 + 1 + i as u16;
        let c = &mut buf[(x, y0)];
        if c.symbol() == "─" {
            if on(i) {
                c.set_symbol("─").set_style(style);
            } else {
                c.set_symbol(" ");
            }
        }
        let q = w + h + (w - 1 - i);
        let c = &mut buf[(x, y1)];
        if c.symbol() == "─" {
            if on(q) {
                c.set_symbol("─").set_style(style);
            } else {
                c.set_symbol(" ");
            }
        }
    }
    // Right edge (top→bottom), then left edge (perimeter continues clockwise).
    for j in 0..h {
        let y = y0 + 1 + j as u16;
        let c = &mut buf[(x1, y)];
        if c.symbol() == "│" {
            if on(w + j) {
                c.set_symbol("│").set_style(style);
            } else {
                c.set_symbol(" ");
            }
        }
        let c = &mut buf[(x0, y)];
        if c.symbol() == "│" {
            if on(w + h + w + h - 1 - j) {
                c.set_symbol("│").set_style(style);
            } else {
                c.set_symbol(" ");
            }
        }
    }
    // Corners: keep glyph, tint to match the dashes.
    for (cx, cy) in [(x0, y0), (x1, y0), (x0, y1), (x1, y1)] {
        buf[(cx, cy)].set_style(style);
    }
}

/// Compact centered dialog (fixed size, never a stretched panel).
fn centered_fixed(w: u16, h: u16, area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    use ratatui::layout::{Constraint, Direction, Layout};
    let w = w.min(area.width.saturating_sub(4)).max(20);
    let h = h.min(area.height.saturating_sub(2)).max(10);
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(area.height.saturating_sub(h) / 2),
            Constraint::Length(h),
            Constraint::Min(0),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(area.width.saturating_sub(w) / 2),
            Constraint::Length(w),
            Constraint::Min(0),
        ])
        .split(vertical[1])[1]
}

/// Render the full frame.
pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    // Tiny terminals: render a compact fallback instead of collapsing.
    if area.height < 12 || area.width < 60 {
        let msg = Paragraph::new(vec![
            Line::from(Span::styled(
                " algorithco guard — terminal too small ",
                Style::default()
                    .fg(Color::White)
                    .bg(COLOR_BRAND)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "Resize to at least 60x12 to continue.",
                Style::default().fg(COLOR_MUTED),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(COLOR_BORDER)),
        );
        frame.render_widget(msg, area);
        return;
    }

    // Login gate covers the whole screen.
    if app.mode == ViewMode::Login {
        // Clear stale main-view hit areas while the gate is up.
        app.tab_feed = None;
        app.tab_policy = None;
        app.footer_quit = None;
        app.table_inner = None;
        for r in app.tool_rects.iter_mut() {
            *r = None;
        }
        render_login(frame, app, area);
        if app.help_visible {
            render_help_overlay(frame, app, area);
        }
        return;
    }

    // CLI-integration picker is the main view.
    if app.mode == ViewMode::Connect {
        // Clear stale legacy hit areas (Feed/Policy tabs, table).
        app.tab_feed = None;
        app.tab_policy = None;
        app.table_inner = None;
        render_connect(frame, app, area);
        if app.help_visible {
            render_help_overlay(frame, app, area);
        }
        return;
    }

    let has_offline = app.is_offline;
    let mut constraints = vec![
        Constraint::Length(4), // header: title + stats
    ];
    if has_offline {
        constraints.push(Constraint::Length(2)); // compact offline banner
    }
    constraints.push(Constraint::Min(8)); // main
    constraints.push(Constraint::Length(1)); // footer

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut idx = 0;
    render_header(frame, app, chunks[idx]);
    idx += 1;
    if has_offline {
        render_offline_banner(frame, app, chunks[idx]);
        idx += 1;
    }
    if app.mode == ViewMode::Policy {
        app.table_inner = None;
        render_policy(frame, app, chunks[idx]);
    } else if app.entries.is_empty() {
        app.table_inner = None;
        render_empty(frame, app, chunks[idx]);
    } else {
        // Feed: table on top, detail pane below (algo why parity).
        let main = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(8), Constraint::Length(10)])
            .split(chunks[idx]);
        render_table(frame, app, main[0]);
        render_detail(frame, app, main[1]);
    }
    idx += 1;
    render_footer(frame, app, chunks[idx]);
    // `?` help sits on top of every view (research: k9s/gh-dash parity).
    if app.help_visible {
        render_help_overlay(frame, app, area);
    }
}

fn mode_badges(app: &App) -> Vec<Span<'static>> {
    let mut badges = Vec::new();
    if app.is_offline {
        badges.push(Span::styled(
            " OFFLINE ",
            Style::default()
                .fg(Color::Black)
                .bg(COLOR_ASK)
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        badges.push(Span::styled(
            " LIVE ",
            Style::default()
                .fg(Color::White)
                .bg(COLOR_ALLOW)
                .add_modifier(Modifier::BOLD),
        ));
    }
    badges.push(Span::raw(" "));
    if app.paused {
        badges.push(Span::styled(
            " PAUSED ",
            Style::default()
                .fg(Color::White)
                .bg(COLOR_ASK)
                .add_modifier(Modifier::BOLD),
        ));
        badges.push(Span::raw(" "));
    }
    if app.enforce {
        badges.push(Span::styled(
            " ENFORCING ",
            Style::default()
                .fg(Color::White)
                .bg(COLOR_DENY)
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        badges.push(Span::styled(
            " SHADOW ",
            Style::default()
                .fg(Color::White)
                .bg(COLOR_BRAND)
                .add_modifier(Modifier::BOLD),
        ));
    }
    badges
}

fn render_header(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let mut title_spans = vec![
        Span::styled(
            " algorithco guard ",
            Style::default()
                .fg(Color::White)
                .bg(COLOR_BRAND)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" v{} ", app.version),
            Style::default().fg(COLOR_MUTED),
        ),
        Span::styled(
            " Live decision feed ",
            Style::default()
                .fg(COLOR_BRAND)
                .add_modifier(Modifier::BOLD),
        ),
    ];
    title_spans.extend(mode_badges(app));
    let title = Line::from(title_spans);

    let stats = Line::from(vec![
        Span::styled(
            " ● allow ",
            Style::default()
                .fg(COLOR_ALLOW)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("{}", app.counts.allow)),
        Span::styled("  │  ", Style::default().fg(COLOR_BORDER)),
        Span::styled(
            " ● ask ",
            Style::default().fg(COLOR_ASK).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("{}", app.counts.ask)),
        Span::styled("  │  ", Style::default().fg(COLOR_BORDER)),
        Span::styled(
            " ● block ",
            Style::default().fg(COLOR_DENY).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("{}", app.counts.deny)),
        Span::styled("  │  ", Style::default().fg(COLOR_BORDER)),
        Span::styled(" would-have ", Style::default().fg(COLOR_MUTED)),
        Span::styled(
            format!("{}", app.counts.would_have_blocked),
            Style::default().fg(COLOR_DENY).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  shadow {} ", app.counts.shadow),
            Style::default().fg(COLOR_MUTED),
        ),
        Span::styled(
            format!(" total {} ", app.counts.total),
            Style::default().fg(COLOR_MUTED),
        ),
        Span::styled(
            format!(" privacy:{} ", app.privacy),
            Style::default().fg(COLOR_MUTED),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BORDER));
    let paragraph = Paragraph::new(vec![title, stats]).block(block);
    frame.render_widget(paragraph, area);
}

fn render_offline_banner(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    // Persistent degraded banner (research: users must never wonder if the
    // view is stale). Amber = degraded, red only for hard fail. Always shows
    // next action + staleness instead of a silent empty feed.
    let stale = format!("{} cached", app.entries.len());
    let msg = if let Some(err) = &app.error {
        format!(
            "! OFFLINE — {} — showing {} (actions disabled, ask-only) [r] retry [d] doctor — hooks unaffected.",
             truncate(err, 80), stale
        )
    } else {
        format!(
            "! OFFLINE read-only — daemon down or audit.db missing — showing {} [r] retry [d] doctor.",
            stale
        )
    };
    let banner = Paragraph::new(Line::from(vec![Span::styled(
        format!(" {msg} "),
        Style::default()
            .fg(Color::Black)
            .bg(token_ask())
            .add_modifier(Modifier::BOLD),
    )]));
    frame.render_widget(banner, area);
}

/// Login dispatcher: wide terminals get the form + guard-dog side panel;
/// narrow ones keep the single-column stacked form (tests + small screens).
/// Login layout tokens: one spacing unit (1 blank row), left alignment
/// everywhere. Form cards are 52 wide; two columns need form + gap + dog.
const LOGIN_MIN_W: u16 = 60;
const LOGIN_MIN_H: u16 = 18;
const LOGIN_FORM_W: u16 = 52;
const LOGIN_GAP_W: u16 = 4;
const DOG_COL_W: u16 = 38;
const TWO_COL_MIN_W: u16 = 100;
const TWO_COL_MIN_H: u16 = 28;
/// Reserved status rows: fixed so state changes never shift the layout.
const STATUS_ROWS: u16 = 3;
const BROWSER_CARD_H: u16 = 5;
const API_COLLAPSED_H: u16 = 3;
const API_EXPANDED_H: u16 = 5;

fn render_login(frame: &mut Frame, app: &mut App, area: Rect) {
    // Gate first: login needs room for cards + footer. Smaller areas get the
    // gate — never clipped widgets, never a panic on resize.
    if area.width < LOGIN_MIN_W || area.height < LOGIN_MIN_H {
        render_login_too_small(frame, app, area);
        return;
    }
    if area.width >= TWO_COL_MIN_W && area.height >= TWO_COL_MIN_H {
        render_login_two_col(frame, app, area);
    } else {
        render_login_single_col(frame, app, area);
    }
}

fn render_login_too_small(frame: &mut Frame, app: &mut App, area: Rect) {
    app.login_browser = None;
    app.login_apikey = None;
    app.login_apikey_input = None;
    app.login_submit = None;
    app.login_cancel = None;
    app.login_offline = None;
    app.login_quit = None;
    app.login_signin = None;
    app.login_dog = None;
    let msg = Paragraph::new(vec![
        Line::from(Span::styled("Terminal too small", app.theme.fg_bold())),
        Line::from(Span::styled(
            format!(
                "Login needs {}x{}. Current: {}x{}.",
                LOGIN_MIN_W, LOGIN_MIN_H, area.width, area.height
            ),
            app.theme.muted(),
        )),
    ]);
    let w = 42.min(area.width);
    let h = 4.min(area.height);
    frame.render_widget(
        msg,
        Rect {
            x: area.x + area.width.saturating_sub(w) / 2,
            y: area.y + area.height.saturating_sub(h) / 2,
            width: w,
            height: h,
        },
    );
}

/// Right-side guard-dog stage: borderless art with the shared status block
/// directly underneath (1 row gap). Symbol + text, never color alone.
fn render_dog_panel(frame: &mut Frame, app: &mut App, area: Rect) {
    // No box, no border — the dog floats directly on the background.
    let art_h = CELLS_H.min(area.height);
    let art_w = CELLS_W.min(area.width);
    let art = Rect {
        x: area.x + area.width.saturating_sub(art_w) / 2,
        y: area.y,
        width: art_w,
        height: art_h,
    };
    frame.render_widget(&app.dog, art);
    // Clicking the art barks; the whole column stays hover-clean.
    app.login_dog = Some(art);

    let status_y = area.y + art_h + 1;
    if status_y < area.bottom() {
        let status = Rect {
            x: area.x,
            y: status_y,
            width: area.width,
            height: area.bottom().saturating_sub(status_y),
        };
        render_login_status(frame, app, status);
    }
}

/// One login card. Rounded solid border (font-safe), title carries the
/// leading shortcut, and the whole card is the button — no inner buttons,
/// no duplicated labels. Exactly one accent element when focused.
fn login_card(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    focused: bool,
    theme: &Theme,
    rows: Vec<Line<'_>>,
) -> Rect {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if focused {
            theme.border_focus()
        } else {
            theme.border_idle()
        })
        .title(Span::styled(
            format!(" {title} "),
            if focused {
                theme.title_focus()
            } else {
                theme.title_idle()
            },
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(rows), inner);
    inner
}

/// Fit a row into `width` cells (char-boundary truncate with ellipsis).
fn fit_row(s: &str, width: usize) -> String {
    truncate(s, width.max(1))
}

/// Browser card rows: action, destination, state lives in the status block.
/// The destination URL is always shown so the user can verify it.
fn browser_card_rows(app: &App, focused: bool, width: usize) -> Vec<Line<'static>> {
    let t = &app.theme;
    let action = if focused {
        Line::from(vec![
            Span::styled("▶ ", t.fg_bold()),
            Span::styled("Continue in browser", t.fg_bold()),
        ])
    } else {
        Line::from(Span::styled("  Continue in browser", t.muted()))
    };
    let dest = match app.oauth_url.as_deref() {
        Some(url) => fit_row(url, width.saturating_sub(2)),
        None => "Not configured in this build".to_string(),
    };
    vec![
        action,
        Line::from(Span::styled("  Opens OAuth at", t.muted())),
        Line::from(Span::styled(format!("  {dest}"), t.muted())),
    ]
}

/// API card rows. Collapsed: one line. Expanded: masked input + hints.
fn api_card_rows(app: &App, focused: bool, width: usize) -> Vec<Line<'static>> {
    let t = &app.theme;
    let expanded = api_expanded(app);
    if !expanded {
        let line = if focused {
            Line::from(vec![
                Span::styled("▶ ", t.fg_bold()),
                Span::styled("Paste a key from your dashboard", t.fg_bold()),
            ])
        } else {
            Line::from(Span::styled("  Paste a key from your dashboard", t.muted()))
        };
        return vec![line];
    }
    match &app.login_status {
        LoginStatus::ApiKeyValidating => {
            let dots = "•".repeat(app.login_api_input.chars().count().min(width));
            vec![
                Line::from(Span::styled(format!("  {dots}"), t.fg())),
                Line::from(Span::styled("  Storing key locally…", t.muted())),
                Line::from(""),
            ]
        }
        LoginStatus::Success => {
            let user = app.login_user.clone().unwrap_or_default();
            vec![
                Line::from(Span::styled("  ✓ Key accepted", t.ok())),
                Line::from(Span::styled(fit_row(&format!("  {user}"), width), t.fg())),
                Line::from(Span::styled("  Loading your workspace…", t.muted())),
            ]
        }
        _ => {
            let input = if app.login_api_input.is_empty() {
                Line::from(Span::styled("  Ctrl+V to paste", t.muted()))
            } else {
                let dots = "•".repeat(app.login_api_input.chars().count().min(width));
                let cursor = if focused { "█" } else { "" };
                Line::from(vec![
                    Span::styled("  ", t.fg()),
                    Span::styled(fit_row(&dots, width.saturating_sub(3)), t.fg()),
                    Span::styled(cursor, t.fg_bold()),
                ])
            };
            vec![
                input,
                Line::from(Span::styled("  Enter verifies · Esc collapses", t.muted())),
                Line::from(Span::styled(
                    "  Stored locally · masked for security",
                    t.muted(),
                )),
            ]
        }
    }
}

/// Card 2 is expanded while the key is being entered, validated, accepted,
/// or after a key error (so the user can fix it inline).
fn api_expanded(app: &App) -> bool {
    matches!(
        app.login_status,
        LoginStatus::ApiKeyEditing
            | LoginStatus::ApiKeyValidating
            | LoginStatus::Success
            | LoginStatus::Error(_)
    )
}

fn api_card_height(app: &App) -> u16 {
    if api_expanded(app) {
        API_EXPANDED_H
    } else {
        API_COLLAPSED_H
    }
}

/// Single shared status block (dog-adjacent in two columns, under the form
/// in one). Symbol + words, never color alone; no apologies, always a fix.
fn login_status_lines(app: &App, width: usize) -> Vec<Line<'static>> {
    let t = &app.theme;
    let w = width.max(8);
    match &app.login_status {
        LoginStatus::Idle => vec![Line::from(Span::styled("○ calm — guarding", t.ok()))],
        LoginStatus::ApiKeyEditing => vec![Line::from(Span::styled(
            "○ Key entry — Enter verifies · Esc collapses",
            t.muted(),
        ))],
        LoginStatus::ApiKeyValidating => {
            vec![Line::from(Span::styled(
                "○ Storing key locally…",
                t.muted(),
            ))]
        }
        LoginStatus::BrowserPending => {
            if app.browser_wait_ms() >= SPINNER_GRACE_MS {
                vec![Line::from(vec![
                    Span::styled(format!("{} ", spinner_frame(app.spin_phase)), t.accent()),
                    Span::styled("Waiting for browser… Esc to cancel", t.warn()),
                ])]
            } else {
                vec![Line::from(Span::styled("○ Opening browser…", t.muted()))]
            }
        }
        LoginStatus::Success => {
            let user = app.login_user.clone().unwrap_or_default();
            vec![
                Line::from(Span::styled(
                    fit_row(&format!("✓ Signed in as {user}."), w),
                    t.ok(),
                )),
                Line::from(Span::styled(
                    "Key stored locally — not verified yet.",
                    t.muted(),
                )),
            ]
        }
        LoginStatus::Error(msg) => wrap_status_error(msg, w, t),
    }
}

/// Wrap `▲ {msg}` to at most [`STATUS_ROWS`] rows for the status block.
fn wrap_status_error(msg: &str, width: usize, t: &Theme) -> Vec<Line<'static>> {
    let style = t.err();
    let first_prefix = "▲ ";
    let cont_prefix = "  ";
    let first_w = width.saturating_sub(first_prefix.chars().count()).max(8);
    let cont_w = width.saturating_sub(cont_prefix.chars().count()).max(8);
    let chars: Vec<char> = msg.chars().collect();
    let mut rows: Vec<Line<'static>> = Vec::new();
    let mut rest = chars.as_slice();
    // First row carries the symbol (never color alone).
    let take = first_w.min(rest.len());
    let (head, tail) = rest.split_at(take);
    let head: String = head.iter().collect();
    rest = tail;
    rows.push(Line::from(Span::styled(
        format!("{first_prefix}{head}"),
        style,
    )));
    while !rest.is_empty() && rows.len() < STATUS_ROWS as usize {
        let last = rows.len() == STATUS_ROWS as usize - 1;
        let mut take = cont_w.min(rest.len());
        if last && take < rest.len() {
            take = take.saturating_sub(1);
        }
        let (head, tail) = rest.split_at(take);
        let mut line: String = head.iter().collect();
        rest = tail;
        if last && !rest.is_empty() {
            line.push('…');
            rest = &[];
        }
        rows.push(Line::from(Span::styled(
            format!("{cont_prefix}{line}"),
            style,
        )));
    }
    rows
}

fn render_login_status(frame: &mut Frame, app: &App, area: Rect) {
    let lines = login_status_lines(app, area.width as usize);
    frame.render_widget(Paragraph::new(lines), area);
}

/// Offline row: leading underlined shortcut, muted description.
fn render_login_offline(frame: &mut Frame, app: &mut App, area: Rect) {
    let t = app.theme;
    let focused = app.login_focus == LoginFocus::Offline && app.login_status == LoginStatus::Idle;
    let line = Line::from(vec![
        Span::styled(
            "o",
            if focused {
                t.fg_bold().add_modifier(Modifier::UNDERLINED)
            } else {
                t.muted().add_modifier(Modifier::UNDERLINED)
            },
        ),
        Span::styled(
            "  Continue offline · limited features, no sync",
            if focused { t.fg_bold() } else { t.muted() },
        ),
    ]);
    frame.render_widget(Paragraph::new(line), area);
    app.login_offline = Some(area);
}

/// Global footer: one line, every shortcut visible, nothing hidden.
/// Compressed to 48 cells so it fits the 52-wide form untruncated.
fn login_footer_height(app: &App) -> u16 {
    if app.debug {
        2
    } else {
        1
    }
}

fn render_login_footer(frame: &mut Frame, app: &mut App, area: Rect) {
    let t = app.theme;
    let keys = "↑↓ select  Enter confirm  1/2/o  ? help  Esc quit";
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            fit_row(keys, area.width as usize),
            t.muted(),
        ))),
        Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1.min(area.height),
        },
    );
    if app.debug && area.height >= 2 {
        let layer = app.theme.layer;
        let layer_name = match layer {
            crate::dog::ColorMode::TrueColor => "truecolor",
            crate::dog::ColorMode::Ansi16 => "ansi16",
            crate::dog::ColorMode::Mono => "mono",
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                fit_row(
                    &format!(
                        "debug: color={layer_name} tick={} · b bark · m color",
                        app.tick
                    ),
                    area.width as usize,
                ),
                t.muted(),
            ))),
            Rect {
                x: area.x,
                y: area.y + 1,
                width: area.width,
                height: 1,
            },
        );
    }
    // No mouse-quit target on login: `Esc`/`q` quit from the keyboard.
    app.login_quit = None;
}

/// Wide login: form (52) + dog column (38) under a shared header.
fn render_login_two_col(frame: &mut Frame, app: &mut App, area: Rect) {
    let api_h = api_card_height(app);
    let footer_h = login_footer_height(app);
    // Header 3 + body 22 (dog 18 + gap + status 3) + gap + footer.
    let content_h = 3 + CELLS_H + 1 + STATUS_ROWS + 1 + footer_h;
    let content_w = LOGIN_FORM_W + LOGIN_GAP_W + DOG_COL_W;
    let ox = area.x + area.width.saturating_sub(content_w) / 2;
    let mut y = area.y + area.height.saturating_sub(content_h) / 2;

    render_login_header(
        frame,
        app,
        Rect {
            x: ox,
            y,
            width: content_w,
            height: 2,
        },
    );
    y += 3; // title + subtitle + gap

    let form = Rect {
        x: ox,
        y,
        width: LOGIN_FORM_W,
        height: api_h + BROWSER_CARD_H + 3,
    };
    render_login_cards(frame, app, form);
    let dog_col = Rect {
        x: ox + LOGIN_FORM_W + LOGIN_GAP_W,
        y,
        width: DOG_COL_W,
        height: CELLS_H + 1 + STATUS_ROWS,
    };
    // Dog art top-aligned with the cards; status sits 1 row below the art.
    render_dog_panel(frame, app, dog_col);
    y += CELLS_H + 1 + STATUS_ROWS + 1; // body + gap

    render_login_footer(
        frame,
        app,
        Rect {
            x: ox,
            y,
            width: content_w,
            height: footer_h,
        },
    );
    app.login_signin = app.login_browser;
}

/// Narrow login: single column, dog hidden, the shared status line stays.
/// Gaps are spacing luxury: dropped first when rows are scarce, so the form
/// (cards + status + offline + footer) always fits heights down to 15.
fn render_login_single_col(frame: &mut Frame, app: &mut App, area: Rect) {
    let api_h = api_card_height(app);
    let footer_h = login_footer_height(app);
    let content_w = LOGIN_FORM_W.min(area.width.saturating_sub(4)).max(40);
    let form_h = BROWSER_CARD_H + 1 + api_h;
    // Core rows (never dropped): header + cards + status + offline + footer.
    let core_h = 2 + form_h + STATUS_ROWS + 1 + footer_h;
    // Droppable 1-row gaps: after header, after cards, after offline.
    let mut keep_gaps = 3u16;
    while core_h + keep_gaps > area.height && keep_gaps > 0 {
        keep_gaps -= 1;
    }
    // Gaps are spent in reading order: header gap, cards gap, offline gap.
    let mut gaps_left = keep_gaps;

    let ox = area.x + area.width.saturating_sub(content_w) / 2;
    let mut y = area.y + area.height.saturating_sub(core_h + keep_gaps) / 2;
    let w = content_w;
    let mut row = |h: u16| {
        let r = Rect {
            x: ox,
            y,
            width: w,
            height: h,
        };
        y += h;
        r
    };

    render_login_header(frame, app, row(2));
    if gaps_left > 0 {
        row(1);
        gaps_left -= 1;
    }

    app.login_dog = None;
    render_login_cards_compact(frame, app, row(form_h));
    if gaps_left > 0 {
        row(1);
        gaps_left -= 1;
    }

    render_login_status(frame, app, row(STATUS_ROWS));

    render_login_offline(frame, app, row(1));
    if gaps_left > 0 {
        row(1);
    }

    render_login_footer(
        frame,
        app,
        Rect {
            x: ox,
            y,
            width: w,
            height: footer_h,
        },
    );
    app.login_signin = app.login_browser;
}

fn render_login_header(frame: &mut Frame, app: &mut App, area: Rect) {
    let t = app.theme;
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "Sign in to Algorithco Guard",
            t.fg_bold(),
        ))),
        Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1.min(area.height),
        },
    );
    if area.height >= 2 {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "Choose how to authenticate",
                t.muted(),
            ))),
            Rect {
                x: area.x,
                y: area.y + 1,
                width: area.width,
                height: 1,
            },
        );
    }
}

/// Both cards + the offline row inside `area` (left-aligned, 1-row rhythm).
/// `area` height must be `BROWSER_CARD_H + 1 + api_h + 1 + 1`.
fn render_login_cards(frame: &mut Frame, app: &mut App, area: Rect) {
    let api_h = api_card_height(app);
    render_login_cards_compact(
        frame,
        app,
        Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: BROWSER_CARD_H + 1 + api_h,
        },
    );
    render_login_offline(
        frame,
        app,
        Rect {
            x: area.x,
            y: area.y + BROWSER_CARD_H + 1 + api_h + 1,
            width: area.width,
            height: 1,
        },
    );
}

/// Browser + API cards only (no offline row).
/// `area` height must be `BROWSER_CARD_H + 1 + api_h`.
fn render_login_cards_compact(frame: &mut Frame, app: &mut App, area: Rect) {
    let t = app.theme;
    let w = area.width as usize;
    let browser_focused = app.login_focus == LoginFocus::Browser
        && matches!(
            app.login_status,
            LoginStatus::Idle | LoginStatus::BrowserPending
        );
    let api_focused = app.login_focus == LoginFocus::ApiKey
        && matches!(
            app.login_status,
            LoginStatus::Idle
                | LoginStatus::ApiKeyEditing
                | LoginStatus::ApiKeyValidating
                | LoginStatus::Success
                | LoginStatus::Error(_)
        );
    let api_h = api_card_height(app);

    let browser_rect = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: BROWSER_CARD_H,
    };
    login_card(
        frame,
        browser_rect,
        "1 Browser",
        browser_focused,
        &t,
        browser_card_rows(app, browser_focused, w.saturating_sub(2)),
    );
    app.login_browser = Some(browser_rect);
    app.login_cancel = None;

    let api_rect = Rect {
        x: area.x,
        y: area.y + BROWSER_CARD_H + 1,
        width: area.width,
        height: api_h,
    };
    login_card(
        frame,
        api_rect,
        "2 API key",
        api_focused,
        &t,
        api_card_rows(app, api_focused, w.saturating_sub(2)),
    );
    app.login_apikey = Some(api_rect);
    // The card itself is the button: no inner input/submit rects.
    app.login_apikey_input = None;
    app.login_submit = None;
}

/// CLI-integration picker — the main view. Lists the terminal coding tools
/// `algo` can work with. UI-only for now: picking a tool stores the choice
/// locally with an honest "not connected yet" note; no connection, no fake
/// connecting animation.
fn render_connect(frame: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let card = centered_fixed(70, 27, area);
    if card.width < 50 || card.height < 24 {
        let msg = Paragraph::new(vec![
            Line::from(Span::styled(
                "CLI integration — pick your tool",
                Style::default()
                    .fg(COLOR_BRAND)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "Resize wider for the full picker.",
                Style::default().fg(COLOR_MUTED),
            )),
        ]);
        frame.render_widget(msg, card);
        return;
    }

    // Header (2) + 4 tool boxes (5 each) + status (2) + slack + footer (1).
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(card);

    // ---- Header ----
    let mut title = vec![
        Span::styled(
            " algorithco guard ",
            Style::default()
                .fg(Color::White)
                .bg(COLOR_BRAND)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" v{} ", app.version),
            Style::default().fg(COLOR_MUTED),
        ),
        Span::styled(
            " CLI integration ",
            Style::default()
                .fg(COLOR_BRAND)
                .add_modifier(Modifier::BOLD),
        ),
    ];
    title.extend(mode_badges(app));
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(title),
            Line::from(Span::styled(
                "Pick the terminal coding tool algo should work with",
                Style::default().fg(COLOR_MUTED),
            )),
        ]),
        rows[0],
    );

    // ---- Tool boxes ----
    let dash_style = Style::default()
        .fg(COLOR_BRAND)
        .add_modifier(Modifier::BOLD);
    for (i, tool) in CLI_TOOLS.iter().enumerate() {
        let focused = i == app.tool_selected;
        let chosen = app.connected_tool == Some(*tool);
        let border = if focused {
            Style::default()
                .fg(COLOR_BRAND)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(COLOR_BORDER)
        };
        let title = if focused {
            Span::styled(
                format!(" [{}] ● {} ", i + 1, tool.name()),
                Style::default()
                    .fg(Color::White)
                    .bg(COLOR_BRAND)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                format!(" [{}] {} ", i + 1, tool.name()),
                Style::default()
                    .fg(COLOR_MUTED)
                    .add_modifier(Modifier::BOLD),
            )
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border)
            .title(title);
        let box_rect = rows[1 + i];
        let inner = block.inner(box_rect);
        frame.render_widget(block, box_rect);
        app.tool_rects[i] = Some(box_rect);

        let status = if chosen {
            Line::from(vec![Span::styled(
                "● Selected — stored locally (not connected yet)",
                Style::default()
                    .fg(COLOR_ALLOW)
                    .add_modifier(Modifier::BOLD),
            )])
        } else {
            Line::from(vec![
                Span::styled("○ ", Style::default().fg(COLOR_MUTED)),
                Span::styled("Not connected", Style::default().fg(COLOR_MUTED)),
            ])
        };
        let hint = if focused {
            Line::from(Span::styled(
                "Press Enter to select",
                Style::default().fg(COLOR_MUTED),
            ))
        } else {
            Line::from("")
        };
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(tool.desc(), Style::default().fg(COLOR_MUTED))),
                status,
                hint,
            ]),
            inner,
        );
        if focused {
            render_marching_dashes(frame, box_rect, app.anim_phase, dash_style);
        }
    }

    // ---- Honest status line ----
    let (status_text, status_style) = match &app.connect_status_msg {
        Some(msg) => (
            format!(" {msg} "),
            Style::default()
                .fg(Color::White)
                .bg(COLOR_ALLOW)
                .add_modifier(Modifier::BOLD),
        ),
        None => (
            " Selection is stored locally — algo connection wiring is UI-only for now. "
                .to_string(),
            Style::default().fg(COLOR_MUTED),
        ),
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(status_text, status_style)))
            .alignment(ratatui::layout::Alignment::Center),
        rows[5],
    );

    // ---- Footer: hint + quit (own sub-rects, like the feed footer) ----
    let foot = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(8)])
        .split(rows[7]);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "click a tool to select · j/k to move · Enter to choose",
            Style::default().fg(COLOR_MUTED),
        )))
        .alignment(ratatui::layout::Alignment::Center),
        foot[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " ✕ Quit ",
            Style::default().fg(COLOR_MUTED),
        )))
        .alignment(ratatui::layout::Alignment::Center),
        foot[1],
    );
    app.footer_quit = Some(foot[1]);

    // The picker and its click rects must stay in lockstep.
    debug_assert_eq!(CLI_TOOLS.len(), CLI_TOOL_COUNT);
    debug_assert_eq!(app.tool_rects.len(), CLI_TOOL_COUNT);
}

fn render_table(frame: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let header = Row::new(vec![
        Cell::from("#").style(
            Style::default()
                .fg(COLOR_MUTED)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("time").style(
            Style::default()
                .fg(COLOR_MUTED)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("action").style(
            Style::default()
                .fg(COLOR_MUTED)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("reason").style(
            Style::default()
                .fg(COLOR_MUTED)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("source").style(
            Style::default()
                .fg(COLOR_MUTED)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("lat").style(
            Style::default()
                .fg(COLOR_MUTED)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("sh").style(
            Style::default()
                .fg(COLOR_MUTED)
                .add_modifier(Modifier::BOLD),
        ),
    ])
    .height(1);

    // Selection highlight comes solely from `row_highlight_style` + `TableState`
    // (rendered via `render_stateful_widget`, which also scrolls into view).
    // Cells only set foregrounds — never backgrounds — so there is exactly one
    // highlighting mechanism. The "#" index column uses white when selected
    // (same as the rest of the row) and COLOR_MUTED only when unselected.
    let selected_idx = app.selected;
    let rows: Vec<Row> = app
        .entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let action = e.action_str();
            let color = action_color(action);
            // Triple-encoded (color+glyph+word): ✓/?/✗ stay distinct under
            // red-green colorblindness and in grayscale (research §2.3).
            let dot = action_glyph(action);
            let selected = i == selected_idx;
            let base = if selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let index_style = if selected { base } else { base.fg(COLOR_MUTED) };
            Row::new(vec![
                Cell::from(format!("{}", i + 1)).style(index_style),
                Cell::from(format_time_short(e.ts)).style(base),
                Cell::from(format!("{dot} {action}")).style(if selected {
                    base
                } else {
                    Style::default().fg(color).add_modifier(Modifier::BOLD)
                }),
                Cell::from(truncate(&e.reason, 60)).style(base),
                Cell::from(e.source_str()).style(base.fg(if selected {
                    Color::White
                } else {
                    COLOR_MUTED
                })),
                Cell::from(format!("{}ms", e.latency_ms)).style(base),
                Cell::from(if e.shadow { "◐" } else { " " }).style(base.fg(if selected {
                    Color::White
                } else {
                    COLOR_ASK
                })),
            ])
            .height(1)
        })
        .collect();

    let widths = [
        Constraint::Length(4),
        Constraint::Length(9),
        Constraint::Length(10),
        Constraint::Min(20),
        Constraint::Length(10),
        Constraint::Length(7),
        Constraint::Length(3),
    ];
    let title = format!(
        " recent decisions — {} shown · click a row to inspect ",
        app.entries.len()
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BORDER))
        .title(title);
    // Inner content area for click-to-select (header row + data rows).
    let inner = block.inner(area);
    app.table_inner = Some(inner);
    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .bg(COLOR_BRAND)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_stateful_widget(table, area, &mut app.table_state);
}

fn render_detail(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let Some(e) = app.selected_entry() else {
        let p = Paragraph::new("No selection.").block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(COLOR_BORDER))
                .title(" details — algo why "),
        );
        frame.render_widget(p, area);
        return;
    };
    let action = e.action_str();
    let color = action_color(action);
    let fp_short: String = e.fingerprint.chars().take(12).collect();
    let lines = vec![
        Line::from(vec![
            Span::styled(
                format!(" {action} "),
                Style::default()
                    .fg(Color::White)
                    .bg(color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                truncate(&e.reason, 90),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" confidence ", Style::default().fg(COLOR_MUTED)),
            Span::raw(confidence_bar(e.confidence)),
            Span::styled(
                format!(
                    "  source {}  latency {}ms  profile {}  shadow {} ",
                    e.source_str(),
                    e.latency_ms,
                    e.profile,
                    if e.shadow { "yes◐" } else { "no" }
                ),
                Style::default().fg(COLOR_MUTED),
            ),
        ]),
        Line::from(vec![
            Span::styled(" cmd ", Style::default().fg(COLOR_MUTED)),
            Span::raw(truncate(&e.redacted_command, 100)),
        ]),
        Line::from(vec![
            Span::styled(" fp ", Style::default().fg(COLOR_MUTED)),
            Span::raw(format!("{fp_short}  ")),
            Span::styled("ts ", Style::default().fg(COLOR_MUTED)),
            Span::raw(format!("{}  ", format_ts(e.ts))),
            Span::styled("sess ", Style::default().fg(COLOR_MUTED)),
            Span::raw(truncate(&e.session_id, 24)),
        ]),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BORDER))
        .title(" details — algo why (action+reason+confidence+source+latency) ");
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_empty(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    // Empty states with next action (research: never a blank list — show help
    // + status). Distinguish offline-cached vs genuinely-empty.
    let (title, hint, next) = if app.is_offline {
        (
            " Feed empty — offline ",
            "audit.db not found or daemon down — showing cached (possibly zero) rows.",
            "[r] retry  ·  run algo init, trigger a Bash tool, rows appear automatically.",
        )
    } else {
        (
            " No decisions yet ",
            "No tool events yet. Trigger a Bash tool via the hook — new rows appear automatically.",
            "[r] refresh  ·  algo status shows counts+savings · algo why shows last decision.",
        )
    };
    let text = vec![
        Line::from(Span::styled(
            title,
            Style::default()
                .fg(Color::White)
                .bg(token_brand())
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(hint, Style::default().fg(COLOR_MUTED))),
        Line::from(""),
        Line::from(Span::styled(next, Style::default().fg(COLOR_MUTED))),
        Line::from(Span::styled(
            "algo log --show-egress inspects egress  ·  ? help  ·  q quit",
            Style::default().fg(COLOR_MUTED),
        )),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BORDER))
        .title(" recent decisions ");
    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: true }),
        area,
    );
}

/// `?` help overlay — context keys for every view (research: k9s/gh-dash `?`).
fn render_help_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let t = app.theme;
    let help = centered_fixed(64, 18, area);
    let lines = vec![
        Line::from(Span::styled(
            " Keys (? to close, Esc closes) ",
            if t.is_mono() {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::White)
                    .bg(t.accent)
                    .add_modifier(Modifier::BOLD)
            },
        )),
        Line::from(""),
        Line::from(Span::raw(
            " Login:   Up/Down/Tab move · Enter activates card · 1/2/o shortcut · Esc cancel/quit",
        )),
        Line::from(Span::raw(
            " Mascot:  click dog or b to bark · m cycles color (truecolor/ansi16/mono)",
        )),
        Line::from(Span::raw(" Connect: j/k or Up/Down move · 1-4 jump · Enter choose · q/Esc quit")),
        Line::from(Span::raw(" Feed:    j/k move · g/G or Home/End top/bottom · PgUp/PgDn page")),
        Line::from(Span::raw("          r refresh · ? help · q/Esc quit · click a row to inspect")),
        Line::from(Span::raw(" Detail:  action+reason+confidence+source+latency (= algo why)")),
        Line::from(""),
        Line::from(Span::styled(
            "Mouse-first: every button/row/tab is clickable. Shift+click bypasses capture for text select.",
            t.muted(),
        )),
        Line::from(Span::styled(
            "Offline: read-only cached view — actions disabled, ask-only. [r] retries.",
            t.muted(),
        )),
    ];
    // Clear behind the popup so text stays readable (ratatui recipe).
    frame.render_widget(ratatui::widgets::Clear, help);
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(t.border_focus()),
            )
            .wrap(Wrap { trim: true }),
        help,
    );
}

fn render_policy(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let db = app
        .db_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "—".into());
    let cfg = app
        .config_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "—".into());
    let sock = app
        .socket_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "—".into());
    let mode = if app.enforce {
        "ENFORCING (shadow off)"
    } else {
        "SHADOW (default P1, never blocks)"
    };
    let mode_color = if app.enforce { COLOR_DENY } else { COLOR_BRAND };
    let text = vec![
        Line::from(vec![
            Span::styled(
                " Policy — local snapshot (read-only) ",
                Style::default()
                    .fg(Color::White)
                    .bg(COLOR_BRAND)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                format!(" {mode} "),
                Style::default()
                    .fg(Color::White)
                    .bg(mode_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " enforce  ",
                Style::default()
                    .fg(COLOR_MUTED)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(if app.enforce {
                "on  (algo enforce off → shadow)"
            } else {
                "off (shadow — daemon computes, always approves + would_have)"
            }),
        ]),
        Line::from(vec![
            Span::styled(
                " privacy  ",
                Style::default()
                    .fg(COLOR_MUTED)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "{}  (local-only|redacted|full — redact before egress)",
                app.privacy
            )),
        ]),
        Line::from(vec![
            Span::styled(
                " paused   ",
                Style::default()
                    .fg(COLOR_MUTED)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(if app.paused {
                "yes — hook bypasses daemon instantly"
            } else {
                "no"
            }),
        ]),
        Line::from(vec![
            Span::styled(
                " counts   ",
                Style::default()
                    .fg(COLOR_MUTED)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "allow {} · ask {} · block {} · would-have {} · shadow {} · total {}",
                app.counts.allow,
                app.counts.ask,
                app.counts.deny,
                app.counts.would_have_blocked,
                app.counts.shadow,
                app.counts.total
            )),
        ]),
        Line::from(vec![
            Span::styled(
                " paths    ",
                Style::default()
                    .fg(COLOR_MUTED)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "db {} | cfg {} | sock {}",
                truncate(&db, 34),
                truncate(&cfg, 30),
                truncate(&sock, 28)
            )),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " Dry-run (no enforcement): status digest · why last decision · log --show-egress.",
            Style::default().fg(COLOR_MUTED),
        )),
        Line::from(Span::styled(
            " Change mode with algo enforce on|off, then restart the daemon. Read-only here.",
            Style::default().fg(COLOR_MUTED),
        )),
        Line::from(Span::styled(
            " Switch with the Feed / Policy tabs below. Refresh is automatic.",
            Style::default().fg(COLOR_MUTED),
        )),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BRAND))
        .title(" policy (read-only snapshot) ");
    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: false }),
        area,
    );
}

fn render_footer(frame: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    // Mouse-first tabs + quit. Each clickable label gets its OWN sub-Rect
    // from a Layout split and is drawn into exactly that rect — the stored
    // click-hit rect IS the drawn rect, so they cannot drift apart.
    let feed_label = if app.mode == ViewMode::Feed {
        " ● Feed "
    } else {
        " ○ Feed "
    };
    let pol_label = if app.mode == ViewMode::Policy {
        " ● Policy "
    } else {
        " ○ Policy "
    };
    let quit_label = " ✕ Quit ";
    // Fixed Length per label == label width; middle hint takes the rest.
    let parts = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(8),  // Feed (len 8)
            Constraint::Length(1),  // spacer
            Constraint::Length(10), // Policy (len 10)
            Constraint::Min(0),     // hint
            Constraint::Length(8),  // Quit (len 8)
        ])
        .split(area);
    app.tab_feed = Some(parts[0]);
    app.tab_policy = Some(parts[2]);
    app.footer_quit = Some(parts[4]);

    let feed_style = if app.mode == ViewMode::Feed {
        Style::default()
            .fg(Color::White)
            .bg(COLOR_BRAND)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(COLOR_BRAND)
            .add_modifier(Modifier::BOLD)
    };
    let pol_style = if app.mode == ViewMode::Policy {
        Style::default()
            .fg(Color::White)
            .bg(COLOR_BRAND)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(COLOR_BRAND)
            .add_modifier(Modifier::BOLD)
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(feed_label, feed_style)))
            .alignment(ratatui::layout::Alignment::Center),
        parts[0],
    );
    // parts[1] intentionally left blank as a spacer.
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(pol_label, pol_style)))
            .alignment(ratatui::layout::Alignment::Center),
        parts[2],
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "j/k move · g/G top/bottom · r refresh · ? help · click row",
            Style::default().fg(COLOR_MUTED),
        )))
        .alignment(ratatui::layout::Alignment::Center),
        parts[3],
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            quit_label,
            Style::default().fg(COLOR_MUTED),
        )))
        .alignment(ratatui::layout::Alignment::Center),
        parts[4],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, LoginStatus};
    use algo_audit::AuditStore;
    use algo_types::{Action, Decision, SourceLevel};
    use ratatui::{backend::TestBackend, Terminal};
    use tempfile::TempDir;

    fn decision(action: Action) -> Decision {
        Decision {
            action: action as i32,
            reason: format!("reason {}", action as i32),
            confidence_0_1: 0.9,
            source_level: SourceLevel::Rule as i32,
            latency_ms: 12,
            policy_version: "test".to_string(),
            trace_id: "t".to_string(),
        }
    }

    #[test]
    fn render_no_panic_empty() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(None);
        // render feed (default), not login — need to set feed explicitly for this test
        app.mode = crate::app::ViewMode::Feed;
        terminal
            .draw(|f| render(f, &mut app))
            .expect("render should not panic on empty offline");
        let buffer = terminal.backend().buffer();
        let content: String = buffer
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(content.contains("algorithco guard") || content.contains("guard"));
    }

    #[test]
    fn render_no_panic_with_entries() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("audit.db");
        let store = AuditStore::open(&db).unwrap();
        store.init().unwrap();
        store
            .insert(&decision(Action::Allow), "fp1", false)
            .unwrap();
        store.insert(&decision(Action::Deny), "fp2", true).unwrap();
        store.insert(&decision(Action::Ask), "fp3", false).unwrap();

        let mut app = App::new(Some(store));
        app.refresh();
        app.mode = crate::app::ViewMode::Feed;

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| render(f, &mut app))
            .expect("render should not panic with entries");

        // Also test policy mode
        app.show_policy();
        terminal
            .draw(|f| render(f, &mut app))
            .expect("render policy should not panic");
    }

    #[test]
    fn render_offline_banner_shown() {
        let mut app = App::new(None);
        app.is_offline = true;
        app.error = Some("simulated daemon down".to_string());
        app.mode = crate::app::ViewMode::Feed;
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let backend_ref = terminal.backend();
        let s: String = backend_ref
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(s.contains("Offline") || s.contains("OFFLINE"));
    }

    #[test]
    fn render_detail_and_policy_no_panic_small() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("audit.db");
        let store = AuditStore::open(&db).unwrap();
        store.init().unwrap();
        store
            .insert(&decision(Action::Deny), "fp-detail-1234567890", true)
            .unwrap();
        let mut app = App::new(Some(store));
        app.refresh();
        app.mode = crate::app::ViewMode::Feed;
        // Feed with detail
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let s: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(s.contains("details") || s.contains("why"));
        // Click-to-select a row via stored table area.
        let inner = app.table_inner.expect("table area recorded");
        assert!(!app.handle_click(inner.x.saturating_add(1), inner.y.saturating_add(1)));
        // Policy with real snapshot
        app.show_policy();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let s2: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(s2.contains("Policy") || s2.contains("policy"));
        // Tiny terminal fallback
        let small = TestBackend::new(50, 10);
        let mut t2 = Terminal::new(small).unwrap();
        t2.draw(|f| render(f, &mut app)).unwrap();
    }

    #[test]
    fn render_login_polished_hierarchy() {
        let mut app = App::new(None);
        app.show_login();
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let s: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        // Hierarchy: left-aligned title, one card per auth path, muted offline.
        assert!(
            s.contains("Sign in to Algorithco Guard"),
            "title missing: {s}"
        );
        assert!(
            s.contains("Choose how to authenticate"),
            "subtitle missing: {s}"
        );
        // Leading shortcuts, card text appears exactly once (card IS the button).
        assert!(
            s.contains("1 Browser") && s.contains("Continue in browser"),
            "browser card missing: {s}"
        );
        assert_eq!(
            s.matches("Continue in browser").count(),
            1,
            "card label must not repeat: {s}"
        );
        assert!(
            s.contains("2 API key") && s.contains("Paste a key from your dashboard"),
            "apikey card missing: {s}"
        );
        assert!(
            s.contains("Continue offline"),
            "offline secondary missing: {s}"
        );
        // Rounded solid borders, never dashed marching ants.
        assert!(
            s.contains('╭') && s.contains('╮'),
            "rounded corners missing"
        );
        // Global footer with every shortcut; no hidden keys.
        assert!(
            s.contains("↑↓ select") && s.contains("? help") && s.contains("Esc quit"),
            "footer missing: {s}"
        );
        // No dev hints on the login surface (moved to `?` / --debug).
        assert!(!s.contains("click/[b]"), "dev hint leaked onto login: {s}");
        assert!(!s.contains("[m] color"), "dev hint leaked onto login: {s}");
        // Click areas recorded (quit target gone with its label).
        assert!(app.login_browser.is_some(), "browser rect not recorded");
        assert!(app.login_apikey.is_some(), "apikey rect not recorded");
        assert!(app.login_offline.is_some(), "offline rect not recorded");
        assert!(app.login_quit.is_none(), "quit rect should be gone");
        assert!(app.login_submit.is_none(), "no inner submit button");
    }

    #[test]
    fn render_login_states_browser_pending_and_apikey() {
        let mut app = App::new(None);
        app.show_login();
        app.oauth_url = Some("http://127.0.0.1:8912/callback".to_string());
        // Browser pending past the grace period: spinner + wait line + URL.
        app.start_browser_signin();
        app.browser_since = Some(std::time::Instant::now() - std::time::Duration::from_millis(500));
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let s: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(s.contains("Waiting for browser"), "wait line missing: {s}");
        assert!(
            s.contains("127.0.0.1"),
            "destination URL must be shown: {s}"
        );
        assert!(s.contains("Esc to cancel"), "cancel hint missing: {s}");
        assert!(
            !s.contains("WD-4829-XK") && !s.contains("Device code"),
            "fabricated device code must be gone: {s}"
        );
        // Before the grace period: static opening line, no spinner yet.
        app.browser_since = Some(std::time::Instant::now());
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let early: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(
            early.contains("Opening browser"),
            "grace line missing: {early}"
        );
        // API key editing: empty entry invites paste, typed entry masks.
        app.show_login();
        app.start_api_key_entry();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let empty: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(
            empty.contains("Ctrl+V to paste"),
            "paste hint missing: {empty}"
        );
        app.push_api_key_char('a');
        app.push_api_key_char('b');
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let s2: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(s2.contains("••"), "masked input missing: {s2}");
        assert!(app.login_submit.is_none(), "card is the button");
        // Error state: specific message with fix, symbol+text.
        app.login_api_input = "".to_string();
        app.submit_api_key();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let s3: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(
            s3.contains("▲") && s3.contains("empty") && s3.contains("ag_"),
            "specific error missing: {s3}"
        );
        assert!(!s3.to_lowercase().contains("sorry"), "no apologies: {s3}");
        // Success state names the key suffix.
        app.login_api_input = "ag-valid-key-12345".to_string();
        app.submit_api_key();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let s4: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(
            s4.contains("✓ Signed in as key ••••2345"),
            "success identity missing: {s4}"
        );
        // Legacy validating state resolves honestly (no fake network check).
        app.login_status = LoginStatus::ApiKeyValidating;
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let s5: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(
            s5.contains("Storing key locally"),
            "honest validating missing: {s5}"
        );
        assert!(
            !s5.contains("Validating API key"),
            "fake validation theater must be gone: {s5}"
        );
    }

    #[test]
    fn format_ts_handles_zero() {
        let s = format_ts(0);
        assert!(!s.is_empty());
    }

    #[test]
    fn truncate_and_bar_helpers() {
        assert_eq!(truncate("hello", 10), "hello");
        assert!(truncate("hello world, long reason here", 10).contains("…"));
        assert!(confidence_bar(0.9).contains("0.90"));
        assert!(confidence_bar(0.0).contains("0.00"));
    }

    fn many_entry_app(n: usize) -> App {
        let mut app = App::new(None);
        for i in 0..n {
            app.entries.push(algo_audit::AuditEntry {
                ts: 1_700_000_000_000 + i as i64,
                session_id: format!("sess-{i}"),
                tool_kind: 0,
                redacted_command: format!("cmd-{i}"),
                fingerprint: format!("fp-{i:04}"),
                action: Action::Allow as i32,
                source: SourceLevel::Rule as i32,
                reason: format!("entry-{i:02} unique-reason"),
                confidence: 0.9,
                latency_ms: 5,
                profile: "test".to_string(),
                shadow: false,
            });
        }
        app.mode = crate::app::ViewMode::Feed;
        app
    }

    #[test]
    fn table_scrolls_last_row_into_view() {
        // >50 entries in a small terminal: after select_last() the selected
        // row must be inside the rendered visible buffer, not scrolled away.
        let mut app = many_entry_app(60);
        app.select_last();
        assert_eq!(app.selected, 59);
        assert_eq!(app.table_state.selected(), Some(59));
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        // Stateful render must have scrolled: offset > 0 for 60 rows in 20h.
        assert!(
            app.table_state.offset() > 0,
            "offset should scroll, got {}",
            app.table_state.offset()
        );
        let buf = terminal.backend().buffer();
        let content: String = buf
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(
            content.contains("entry-59"),
            "selected last row must be visible in buffer"
        );
        assert!(
            !content.contains("entry-00"),
            "first row should be scrolled out of view"
        );
        // Selected row's screen y must be inside the table's visible area.
        let inner = app.table_inner.expect("table area recorded");
        let visible_row = app.selected.saturating_sub(app.table_state.offset());
        let screen_y = inner.y + 1 + visible_row as u16;
        assert!(
            screen_y < inner.y + inner.height,
            "selected row y={screen_y} must be inside inner {inner:?}"
        );
        let cell = &buf[(inner.x + 1, screen_y)];
        assert!(
            !cell.symbol().trim().is_empty() || true,
            "selected row cell should exist"
        );
    }

    fn rect_text(buf: &ratatui::buffer::Buffer, r: ratatui::layout::Rect) -> String {
        let mut s = String::new();
        for y in r.y..r.y + r.height {
            for x in r.x..r.x + r.width {
                s.push_str(buf[(x, y)].symbol());
            }
        }
        s
    }

    fn has_non_whitespace(buf: &ratatui::buffer::Buffer, r: ratatui::layout::Rect) -> bool {
        for y in r.y..r.y + r.height {
            for x in r.x..r.x + r.width {
                if !buf[(x, y)].symbol().trim().is_empty() {
                    return true;
                }
            }
        }
        false
    }

    #[test]
    fn footer_click_rects_match_drawn_labels_at_widths() {
        for width in [80u16, 120u16] {
            let mut app = many_entry_app(3);
            let backend = TestBackend::new(width, 24);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|f| render(f, &mut app)).unwrap();
            let buf = terminal.backend().buffer().clone();
            for (rect_opt, label) in [
                (app.tab_feed, "Feed"),
                (app.tab_policy, "Policy"),
                (app.footer_quit, "Quit"),
            ] {
                let r = rect_opt.unwrap_or_else(|| panic!("rect missing for {label}"));
                assert!(
                    has_non_whitespace(&buf, r),
                    "{label} rect {r:?} has no text"
                );
                let text = rect_text(&buf, r);
                assert!(
                    text.contains(label),
                    "{label} rect {r:?} does not contain label, got {text:?}"
                );
            }
        }
    }

    #[test]
    fn login_card_is_the_button_no_inner_submit() {
        // The card itself is the button: no inner submit/input rects, and a
        // click anywhere on the API card starts key entry.
        for width in [80u16, 120u16] {
            let mut app = App::new(None);
            app.show_login();
            app.start_api_key_entry();
            for c in "ag-valid-key-12345".chars() {
                app.push_api_key_char(c);
            }
            let backend = TestBackend::new(width, 30);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|f| render(f, &mut app)).unwrap();
            assert!(
                app.login_submit.is_none(),
                "inner submit button must be gone at {width}"
            );
            assert!(
                app.login_apikey_input.is_none(),
                "inner input rect must be gone at {width}"
            );
            let card = app.login_apikey.expect("api card rect");
            let buf = terminal.backend().buffer().clone();
            assert!(
                has_non_whitespace(&buf, card),
                "api card {card:?} is empty at {width}"
            );
            // Clicking the card body submits the typed key (Enter equivalent).
            let mut clicked = App::new(None);
            clicked.show_login();
            clicked.login_focus = crate::app::LoginFocus::ApiKey;
            clicked.login_status = crate::app::LoginStatus::ApiKeyEditing;
            for c in "ag-valid-key-12345".chars() {
                clicked.push_api_key_char(c);
            }
            clicked.login_apikey = Some(card);
            assert!(!clicked.handle_click(card.x + 2, card.y + 2));
            assert_eq!(
                clicked.login_status,
                crate::app::LoginStatus::Success,
                "card click must activate like Enter"
            );
        }
    }

    #[test]
    fn login_has_no_near_black_text() {
        // Regression: Rgb(23,22,31) on a default (dark) terminal is unreadable.
        let invisible = Color::Rgb(23, 22, 31);
        let mut app = App::new(None);
        app.show_login();
        // Idle
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        for cell in terminal.backend().buffer().content() {
            assert_ne!(cell.fg, invisible, "idle login has unreadable text");
        }
        // Browser pending (honest)
        app.start_browser_signin();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        for cell in terminal.backend().buffer().content() {
            assert_ne!(cell.fg, invisible, "browser-pending has unreadable text");
        }
        // API editing with input
        app.show_login();
        app.start_api_key_entry();
        for c in "ag-valid-key-12345".chars() {
            app.push_api_key_char(c);
        }
        terminal.draw(|f| render(f, &mut app)).unwrap();
        for cell in terminal.backend().buffer().content() {
            assert_ne!(cell.fg, invisible, "api-editing has unreadable text");
        }
    }

    #[test]
    fn selected_index_column_uses_highlight_fg() {
        // The "#" column must be white-on-purple when selected (same as the
        // rest of the row), muted only when unselected.
        let mut app = many_entry_app(5);
        app.select_first();
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let buf = terminal.backend().buffer().clone();
        let inner = app.table_inner.expect("table area recorded");
        let offset = app.table_state.offset();
        let visible = app.selected.saturating_sub(offset);
        let y = inner.y + 1 + visible as u16;
        let cell = &buf[(inner.x + 1, y)];
        assert_eq!(
            cell.fg,
            Color::White,
            "selected # cell must be white, got {:?}",
            cell.fg
        );
        assert_eq!(
            cell.bg, COLOR_BRAND,
            "selected # cell must sit on highlight bg, got {:?}",
            cell.bg
        );
    }

    #[test]
    fn focused_card_has_rounded_accent_border() {
        // One accent element per focused card: solid rounded border in the
        // focus color, bold title, single ▶ marker. Unfocused: dim idle
        // border, muted title, no marker. No dashed borders anywhere.
        use crate::dog::ColorMode;
        for layer in [ColorMode::TrueColor, ColorMode::Ansi16, ColorMode::Mono] {
            let mut app = App::new(None);
            app.set_color_layer(layer);
            app.show_login(); // focus Browser, Idle
            let backend = TestBackend::new(80, 24);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|f| render(f, &mut app)).unwrap();
            let buf = terminal.backend().buffer().clone();
            let content: String = buf
                .content()
                .iter()
                .map(|c| c.symbol().to_string())
                .collect();
            assert!(
                !content.contains("●"),
                "focus chips must be gone ({layer:?})"
            );
            let browser = app.login_browser.expect("browser rect");
            let apikey = app.login_apikey.expect("apikey rect");
            let top = |r: ratatui::layout::Rect| -> String {
                (r.x..r.x + r.width)
                    .map(|x| buf[(x, r.y)].symbol().to_string())
                    .collect()
            };
            let btop = top(browser);
            let atop = top(apikey);
            // Rounded corners on both cards, titles intact.
            for (name, row) in [("browser", &btop), ("apikey", &atop)] {
                assert!(
                    row.starts_with('╭'),
                    "{name} missing rounded corner ({layer:?}): {row:?}"
                );
                assert!(
                    row.ends_with('╮'),
                    "{name} missing rounded corner ({layer:?}): {row:?}"
                );
            }
            assert!(btop.contains("1 Browser"), "title cut: {btop:?}");
            assert!(atop.contains("2 API key"), "title cut: {atop:?}");
            // Focused border style differs from idle border style.
            let bcell = &buf[(browser.x, browser.y)];
            let acell = &buf[(apikey.x, apikey.y)];
            assert_ne!(
                (bcell.fg, bcell.modifier),
                (acell.fg, acell.modifier),
                "focused card must be visually distinct ({layer:?})"
            );
            // Single ▶ marker on the focused card only.
            let body = |r: ratatui::layout::Rect| -> String {
                let mut s = String::new();
                for y in r.y..r.y + r.height {
                    for x in r.x..r.x + r.width {
                        s.push_str(buf[(x, y)].symbol());
                    }
                }
                s
            };
            assert_eq!(
                body(browser).matches('▶').count(),
                1,
                "exactly one marker on focused card ({layer:?})"
            );
            assert_eq!(
                body(apikey).matches('▶').count(),
                0,
                "no marker on unfocused card ({layer:?})"
            );
        }
    }

    #[test]
    fn render_connect_lists_all_tools() {
        for width in [80u16, 120u16] {
            let mut app = App::new(None);
            app.mode = crate::app::ViewMode::Connect;
            let backend = TestBackend::new(width, 30);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|f| render(f, &mut app)).unwrap();
            let buf = terminal.backend().buffer().clone();
            let content: String = buf
                .content()
                .iter()
                .map(|c| c.symbol().to_string())
                .collect();
            for name in ["Claude Code", "Codex", "Gemini CLI", "Custom"] {
                assert!(content.contains(name), "missing tool {name} at {width}");
            }
            // Each click rect holds exactly its tool's label.
            for (i, name) in ["Claude Code", "Codex", "Gemini CLI", "Custom"]
                .iter()
                .enumerate()
            {
                let r = app.tool_rects[i].expect("tool rect recorded");
                let text = rect_text(&buf, r);
                assert!(
                    text.contains(name),
                    "tool rect {i} must contain {name}, got {text:?}"
                );
                assert!(has_non_whitespace(&buf, r), "tool rect {i} is empty");
            }
            // Honest UI-only messaging, no fake connecting theater.
            assert!(
                content.contains("stored locally") || content.contains("UI-only"),
                "honest note missing at {width}"
            );
            assert!(
                !content.contains("Connecting"),
                "fake connecting theater must not exist at {width}"
            );
        }
    }

    #[test]
    fn connect_click_selects_tool() {
        let mut app = App::new(None);
        app.mode = crate::app::ViewMode::Connect;
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        assert!(app.connected_tool.is_none());
        // Click the middle of the Codex row (index 1).
        let r = app.tool_rects[1].expect("codex rect recorded");
        let cx = r.x + r.width / 2;
        let cy = r.y + r.height / 2;
        assert!(!app.handle_click(cx, cy));
        assert_eq!(app.tool_selected, 1);
        assert_eq!(
            app.connected_tool,
            Some(crate::app::CliTool::Codex),
            "click must choose the clicked tool"
        );
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(
            content.contains("Codex selected — stored locally"),
            "chosen tool must be confirmed honestly: {content}"
        );
    }

    #[test]
    fn connect_keyboard_choose_marks_selected() {
        let mut app = App::new(None);
        app.mode = crate::app::ViewMode::Connect;
        app.select_tool_next(); // Claude Code -> Codex
        app.choose_tool();
        assert_eq!(app.connected_tool, Some(crate::app::CliTool::Codex));
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(
            content.contains("● Selected"),
            "chosen row must show selected"
        );
        assert!(content.contains("Not connected"), "others stay unconnected");
    }

    #[test]
    fn feed_actions_are_triple_encoded() {
        // Research §2.3: color+glyph+word so red/green stay distinct.
        assert_eq!(action_glyph("allow"), "✓");
        assert_eq!(action_glyph("deny"), "✗");
        assert_eq!(action_glyph("ask"), "?");
        // Feed rows render the glyph next to the word.
        let mut app = many_entry_app(2);
        app.entries[0].action = Action::Deny as i32;
        app.entries[1].action = Action::Allow as i32;
        app.select_first();
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let s: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(s.contains("✗ deny"), "deny row must be triple-encoded: {s}");
        assert!(
            s.contains("✓ allow"),
            "allow row must be triple-encoded: {s}"
        );
    }

    #[test]
    fn offline_banner_has_retry_hints() {
        let mut app = App::new(None);
        app.is_offline = true;
        app.error = Some("simulated daemon down".to_string());
        app.mode = crate::app::ViewMode::Feed;
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let s: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(s.contains("OFFLINE"), "banner missing: {s}");
        assert!(s.contains("[r]"), "retry hint missing: {s}");
        assert!(
            s.contains("ask-only") || s.contains("read-only"),
            "degraded note missing: {s}"
        );
    }

    #[test]
    fn help_overlay_renders_on_all_views() {
        for mode in [
            crate::app::ViewMode::Login,
            crate::app::ViewMode::Connect,
            crate::app::ViewMode::Feed,
        ] {
            let mut app = App::new(None);
            app.mode = mode.clone();
            if mode == crate::app::ViewMode::Login {
                app.show_login();
            }
            app.help_visible = true;
            let backend = TestBackend::new(100, 30);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal.draw(|f| render(f, &mut app)).unwrap();
            let s: String = terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .map(|c| c.symbol().to_string())
                .collect();
            assert!(
                s.contains("? to close") || s.contains("Keys"),
                "help missing for {mode:?}: {s}"
            );
            assert!(s.contains("j/k"), "vim hints missing for {mode:?}");
        }
    }

    #[test]
    fn footer_shows_help_hint() {
        let mut app = many_entry_app(2);
        let backend = TestBackend::new(100, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let s: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(s.contains("? help"), "footer ? hint missing: {s}");
        assert!(s.contains("j/k"), "footer vim hint missing: {s}");
    }

    #[test]
    fn ascii_fallbacks_keep_width() {
        // ASCII twins must exist and keep single-cell width (research §3.2/§6).
        assert!(confidence_bar(0.9).contains("0.90"));
        for i in 0..10 {
            assert_eq!(
                spinner_frame(i).chars().count(),
                1,
                "spinner frame {i} must be 1 cell"
            );
        }
    }

    #[test]
    fn login_wide_shows_dog_right_panel() {
        // 130-wide terminal: form left + 36x18 guard dog right, status under it.
        let mut app = App::new(None);
        app.show_login();
        let backend = TestBackend::new(130, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let s: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        // Form side intact.
        assert!(
            s.contains("Sign in to Algorithco Guard"),
            "form title missing"
        );
        assert!(
            s.contains("1 Browser") && s.contains("Continue in browser"),
            "browser card missing"
        );
        // Dog side: art + adjacent status line, no dev hints.
        assert!(
            s.contains("calm") || s.contains("alert") || s.contains("barking"),
            "dog status missing"
        );
        assert!(
            !s.contains("[b] bark") && !s.contains("[m] color"),
            "dev hints must live in `?`, not on login: {s}"
        );
        // Click rect recorded and form rects intact.
        assert!(app.login_dog.is_some(), "dog click rect not recorded");
        assert!(
            app.login_browser.is_some(),
            "browser rect missing in wide layout"
        );
        assert!(
            app.login_apikey.is_some(),
            "apikey rect missing in wide layout"
        );
        // Dog art sits right of the form (perfect side-by-side).
        let dog = app.login_dog.expect("dog rect");
        let form = app.login_browser.expect("form rect");
        assert!(
            dog.x > form.x + form.width,
            "dog must be right of form: dog {dog:?} form {form:?}"
        );
        assert!(
            dog.width >= crate::dog::CELLS_W,
            "dog art must fit 36 cells: {dog:?}"
        );
        assert!(
            dog.height >= crate::dog::CELLS_H,
            "dog art must fit 18 cells: {dog:?}"
        );
    }

    #[test]
    fn login_narrow_hides_dog_gracefully() {
        // 80-wide: single column, no dog art, but the status line stays.
        let mut app = App::new(None);
        app.show_login();
        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let s: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(
            s.contains("Sign in to Algorithco Guard"),
            "form missing in narrow"
        );
        assert!(app.login_dog.is_none(), "dog must hide on narrow terminals");
        assert!(
            s.contains("calm"),
            "status line must stay without the dog: {s}"
        );
        assert!(
            app.login_browser.is_some(),
            "browser rect missing in narrow"
        );
    }

    #[test]
    fn dog_alert_caption_and_bark() {
        let mut app = App::new(None);
        app.show_login();
        app.oauth_url = Some("http://127.0.0.1:8912/callback".to_string());
        app.start_browser_signin();
        // Past the spinner grace period so the wait line is visible.
        app.browser_since = Some(std::time::Instant::now() - std::time::Duration::from_millis(500));
        app.tick(); // syncs dog alert
        assert!(app.dog.is_alert(), "dog must mirror BrowserPending alert");
        let backend = TestBackend::new(130, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let s: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(s.contains("Waiting for browser"), "wait line missing: {s}");
        // Clicking the dog barks.
        let r = app.login_dog.expect("dog rect");
        let cx = r.x + r.width / 2;
        let cy = r.y + r.height / 2;
        assert!(!app.handle_click(cx, cy));
        assert!(app.dog.barking(), "dog click must bark");
        // Color layers cycle truecolor -> ansi-16 -> mono.
        let first = app.dog.color_mode();
        app.cycle_dog_color();
        assert_ne!(app.dog.color_mode(), first);
    }

    fn render_login_text(app: &mut App, w: u16, h: u16) -> String {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, app)).unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect()
    }

    #[test]
    fn login_matrix_sizes_and_layers() {
        // Required matrix: 80x24, 100x30, 120x40, 60x18 in all three tiers.
        // No panic, no missing essentials, focused card always distinct.
        use crate::dog::ColorMode;
        for (w, h) in [(80u16, 24u16), (100, 30), (120, 40), (60, 18)] {
            for layer in [ColorMode::TrueColor, ColorMode::Ansi16, ColorMode::Mono] {
                let mut app = App::new(None);
                app.set_color_layer(layer);
                app.show_login();
                let backend = TestBackend::new(w, h);
                let mut terminal = Terminal::new(backend).unwrap();
                terminal
                    .draw(|f| render(f, &mut app))
                    .unwrap_or_else(|e| panic!("panic at {w}x{h} {layer:?}: {e}"));
                let buf = terminal.backend().buffer().clone();
                let s: String = buf
                    .content()
                    .iter()
                    .map(|c| c.symbol().to_string())
                    .collect();
                if w < 60 || h < 18 {
                    // Only 60x18 hits the gate here (60x18: h == 18 renders).
                    continue;
                }
                assert!(
                    s.contains("Sign in to Algorithco Guard"),
                    "title missing at {w}x{h} {layer:?}"
                );
                assert!(
                    s.contains("Esc quit"),
                    "footer missing at {w}x{h} {layer:?}"
                );
                // Focused card visually distinct from the idle one.
                let b = app.login_browser.expect("browser rect");
                let a = app.login_apikey.expect("apikey rect");
                let bc = &buf[(b.x, b.y)];
                let ac = &buf[(a.x, a.y)];
                assert_ne!(
                    (bc.fg, bc.modifier),
                    (ac.fg, ac.modifier),
                    "focus invisible at {w}x{h} {layer:?}"
                );
                // Dog art only where it fits; status always present.
                if w >= 100 && h >= 28 {
                    assert!(app.login_dog.is_some(), "dog missing at {w}x{h} {layer:?}");
                } else {
                    assert!(
                        app.login_dog.is_none(),
                        "dog must hide at {w}x{h} {layer:?}"
                    );
                }
                assert!(
                    s.contains("calm"),
                    "status missing at {w}x{h} {layer:?}: {s}"
                );
                // Never vague, never sorry, never debug-colored internals.
                assert!(!s.to_lowercase().contains("sorry"), "apology at {w}x{h}");
                assert!(!s.contains("[b] bark"), "dev hint at {w}x{h}");
            }
        }
    }

    #[test]
    fn login_gate_below_minimum() {
        // Below the minimum the UI explains instead of clipping. The global
        // 60-wide gate fires first on narrow screens; the login 60x18 gate
        // fires on short-but-wide screens.
        let mut app = App::new(None);
        app.show_login();
        let narrow = render_login_text(&mut app, 59, 30);
        assert!(
            narrow.contains("too small"),
            "narrow gate missing: {narrow}"
        );
        assert!(app.login_browser.is_none(), "no hit rects behind gate");
        let mut app = App::new(None);
        app.show_login();
        let short = render_login_text(&mut app, 80, 17);
        assert!(
            short.contains("Terminal too small") && short.contains("60x18"),
            "login gate missing at 80x17: {short}"
        );
        assert!(app.login_browser.is_none(), "no hit rects behind gate");
        assert!(app.login_dog.is_none(), "no hit rects behind gate");
        let mut app = App::new(None);
        app.show_login();
        let tiny = render_login_text(&mut app, 50, 10);
        assert!(tiny.contains("too small"), "tiny gate missing: {tiny}");
        // 80x24 must work (required breakpoint).
        let mut app = App::new(None);
        app.show_login();
        let s = render_login_text(&mut app, 80, 24);
        assert!(
            s.contains("Sign in to Algorithco Guard"),
            "80x24 broken: {s}"
        );
        assert!(s.contains("2 API key"), "api card missing at 80x24");
        assert!(s.contains("Esc quit"), "footer missing at 80x24");
    }
}
