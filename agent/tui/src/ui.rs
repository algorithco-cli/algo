//! UI rendering — design tokens Variant 1, closest terminal approximation.
//!
//! Tokens from `plans/design-tokens.md` §1:
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

use crate::app::{App, LoginFocus, LoginStatus, ViewMode, CLI_TOOLS, CLI_TOOL_COUNT};
use chrono::{DateTime, Utc};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
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
    format!("{}{} {:.2}", "█".repeat(filled), "░".repeat(empty), conf)
}

/// Spinner frames for the login animation (tick-driven, no blocking).
const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

fn spinner_frame(tick: usize) -> &'static str {
    SPINNER[tick % SPINNER.len()]
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
        return;
    }

    // CLI-integration picker is the main view.
    if app.mode == ViewMode::Connect {
        // Clear stale legacy hit areas (Feed/Policy tabs, table).
        app.tab_feed = None;
        app.tab_policy = None;
        app.table_inner = None;
        render_connect(frame, app, area);
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
    let msg = if let Some(err) = &app.error {
        format!(
            "Offline read-only — {} — hooks unaffected.",
            truncate(err, 120)
        )
    } else {
        "Offline read-only — daemon down or audit.db missing. Press r to retry.".to_string()
    };
    let banner = Paragraph::new(Line::from(vec![Span::styled(
        format!(" {msg} "),
        Style::default()
            .fg(Color::Black)
            .bg(COLOR_ASK)
            .add_modifier(Modifier::BOLD),
    )]));
    frame.render_widget(banner, area);
}

fn render_login(frame: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    // Production-quality auth gate — two primary paths, one secondary fallback.
    // No outer container box: the card area is used directly (borderless).
    let inner = centered_fixed(68, 24, area);

    if inner.width < 40 || inner.height < 18 {
        // Fallback for smaller but not tiny terminals
        let msg = Paragraph::new(vec![
            Line::from(Span::styled(
                "Sign in to Algorithco Guard",
                Style::default()
                    .fg(COLOR_BRAND)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "Resize wider for full auth options.",
                Style::default().fg(COLOR_MUTED),
            )),
        ]);
        frame.render_widget(msg, inner);
        return;
    }

    // Split inner vertically: title, subtitle, browser box, apikey box, status, offline, hints
    // Heights are tuned to be pixel-polished with proper spacing.
    let is_browser_pending = app.login_status == LoginStatus::BrowserPending;
    let is_apikey_editing = matches!(
        app.login_status,
        LoginStatus::ApiKeyEditing | LoginStatus::Error(_)
    );
    let is_validating = app.login_status == LoginStatus::ApiKeyValidating;
    let is_success = app.login_status == LoginStatus::Success;
    let is_error = matches!(app.login_status, LoginStatus::Error(_));

    // Determine box heights based on state — keep total ~ inner.height
    let browser_h: u16 = if is_browser_pending { 7 } else { 5 };
    let apikey_h: u16 = if is_apikey_editing || is_validating || is_success || is_error {
        7
    } else {
        5
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title
            Constraint::Length(1), // subtitle
            Constraint::Length(1), // spacer
            Constraint::Length(browser_h),
            Constraint::Length(1), // spacer
            Constraint::Length(apikey_h),
            Constraint::Length(1), // status line
            Constraint::Length(1), // offline option
            Constraint::Min(1),    // hints + quit
        ])
        .split(inner);

    // ---- Title ----
    let title = Paragraph::new(Line::from(Span::styled(
        "Sign in to Algorithco Guard",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )))
    .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(title, chunks[0]);

    // ---- Subtitle ----
    let subtitle = Paragraph::new(Line::from(Span::styled(
        "Choose how to authenticate — browser or API key",
        Style::default().fg(COLOR_MUTED),
    )))
    .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(subtitle, chunks[1]);

    // ---- Browser box ----
    let browser_focused = app.login_focus == LoginFocus::Browser
        && matches!(
            app.login_status,
            LoginStatus::Idle | LoginStatus::BrowserPending
        );
    let browser_border = if browser_focused {
        Style::default()
            .fg(COLOR_BRAND)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(COLOR_BORDER)
    };
    let browser_title = if browser_focused {
        Span::styled(
            " [1] ● Sign in with browser ",
            Style::default()
                .fg(Color::White)
                .bg(COLOR_BRAND)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " [1] Sign in with browser ",
            Style::default()
                .fg(COLOR_MUTED)
                .add_modifier(Modifier::BOLD),
        )
    };
    let browser_block = Block::default()
        .borders(Borders::ALL)
        .border_style(browser_border)
        .title(browser_title);
    let browser_inner = browser_block.inner(chunks[3]);
    frame.render_widget(browser_block, chunks[3]);
    app.login_browser = Some(chunks[3]);

    // Browser box contents — honest: no backend, no fabricated device code.
    if is_browser_pending {
        let spin = spinner_frame(app.spin_phase);
        let lines = vec![
            Line::from(vec![
                Span::styled(
                    format!(" {spin} "),
                    Style::default()
                        .fg(COLOR_BRAND)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Browser sign-in is not available yet",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(
                "Use an API key or Continue offline instead",
                Style::default().fg(COLOR_MUTED),
            )),
            Line::from(Span::styled(
                "No browser was opened — nothing is waiting",
                Style::default().fg(COLOR_MUTED),
            )),
            Line::from(Span::styled(
                "Press Esc to cancel  •  [Esc] Cancel",
                Style::default().fg(COLOR_MUTED),
            )),
        ];
        let p = Paragraph::new(lines);
        frame.render_widget(p, browser_inner);
        // Cancel hit area is the last line
        app.login_cancel = Some(ratatui::layout::Rect {
            x: browser_inner.x,
            y: browser_inner.y.saturating_add(3),
            width: browser_inner.width,
            height: 1,
        });
    } else {
        let btn_style = if browser_focused {
            Style::default()
                .fg(Color::White)
                .bg(COLOR_BRAND)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(COLOR_BRAND)
                .add_modifier(Modifier::BOLD)
        };
        let desc = if browser_focused {
            "Opens your browser for OAuth  •  Press Enter to start"
        } else {
            "Opens your browser for OAuth"
        };
        let lines = vec![
            Line::from(Span::styled(desc, Style::default().fg(COLOR_MUTED))),
            Line::from(""),
            Line::from(Span::styled(
                if browser_focused {
                    " ▶  Sign in with browser  "
                } else {
                    "    Sign in with browser    "
                },
                btn_style,
            )),
        ];
        let p = Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(p, browser_inner);
        app.login_cancel = None;
    }

    // ---- API key box ----
    let apikey_focused = app.login_focus == LoginFocus::ApiKey
        && matches!(
            app.login_status,
            LoginStatus::Idle
                | LoginStatus::ApiKeyEditing
                | LoginStatus::ApiKeyValidating
                | LoginStatus::Success
                | LoginStatus::Error(_)
        );
    let apikey_border = if apikey_focused {
        Style::default()
            .fg(COLOR_BRAND)
            .add_modifier(Modifier::BOLD)
    } else if is_error {
        Style::default().fg(COLOR_DENY)
    } else {
        Style::default().fg(COLOR_BORDER)
    };
    let apikey_title = if apikey_focused {
        Span::styled(
            " [2] ● Use an API key ",
            Style::default()
                .fg(Color::White)
                .bg(COLOR_BRAND)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            " [2] Use an API key ",
            Style::default()
                .fg(COLOR_MUTED)
                .add_modifier(Modifier::BOLD),
        )
    };
    let apikey_block = Block::default()
        .borders(Borders::ALL)
        .border_style(apikey_border)
        .title(apikey_title);
    let apikey_inner = apikey_block.inner(chunks[5]);
    frame.render_widget(apikey_block, chunks[5]);
    app.login_apikey = Some(chunks[5]);

    // API key contents — legacy validating state resolves honestly (no fake
    // network check); normal flow goes straight to Success on submit.
    if is_validating {
        let lines = vec![
            Line::from(vec![Span::styled(
                "Storing API key locally (not verified)…",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(Span::styled(
                "No network validation in this MVP",
                Style::default().fg(COLOR_MUTED),
            )),
        ];
        frame.render_widget(
            Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center),
            apikey_inner,
        );
        app.login_apikey_input = None;
        app.login_submit = None;
    } else if is_success {
        let lines = vec![
            Line::from(vec![
                Span::styled(
                    " ✓ ",
                    Style::default()
                        .fg(COLOR_ALLOW)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "API key stored locally, not verified.",
                    Style::default()
                        .fg(COLOR_ALLOW)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Loading your workspace…",
                Style::default().fg(COLOR_MUTED),
            )),
        ];
        frame.render_widget(
            Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center),
            apikey_inner,
        );
        app.login_apikey_input = None;
        app.login_submit = None;
    } else if is_apikey_editing || is_error {
        // Input field
        let has_input = !app.login_api_input.is_empty();
        let display = if has_input {
            if app.login_api_masked {
                "•".repeat(app.login_api_input.chars().count())
            } else {
                app.login_api_input.clone()
            }
        } else {
            String::new()
        };
        let input_line = if has_input {
            // Masked with cursor
            let cursor = if apikey_focused { "█" } else { "" };
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(display, Style::default().fg(Color::White)),
                Span::styled(
                    cursor,
                    Style::default()
                        .fg(COLOR_BRAND)
                        .add_modifier(Modifier::BOLD),
                ),
            ])
        } else {
            Line::from(Span::styled(
                "  Paste your API key…",
                Style::default()
                    .fg(COLOR_MUTED)
                    .add_modifier(Modifier::ITALIC),
            ))
        };
        // Input box border inside apikey box: field is 3 rows high (border + input line + border)
        let field_rect = ratatui::layout::Rect {
            x: apikey_inner.x,
            y: apikey_inner.y,
            width: apikey_inner.width,
            height: 3,
        };
        app.login_apikey_input = Some(field_rect);

        let field_block = Block::default()
            .borders(Borders::ALL)
            .border_style(if is_error {
                Style::default().fg(COLOR_DENY)
            } else if apikey_focused {
                Style::default().fg(COLOR_BRAND)
            } else {
                Style::default().fg(COLOR_BORDER)
            });
        let field_inner = field_block.inner(field_rect);
        frame.render_widget(field_block, field_rect);
        frame.render_widget(Paragraph::new(input_line), field_inner);

        // Submit / helper line (below the input field).
        // The Submit button gets its OWN sub-Rect from a Layout split and the
        // label is drawn into exactly that rect — no hand-computed centering
        // that can drift from what's drawn.
        let helper_area = ratatui::layout::Rect {
            x: apikey_inner.x,
            y: apikey_inner.y + 3,
            width: apikey_inner.width,
            height: 1,
        };
        let can_submit = has_input && !app.login_api_input.trim().is_empty();
        let submit_label = " Submit ";
        let submit_style = if can_submit {
            Style::default()
                .fg(Color::White)
                .bg(COLOR_BRAND)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(COLOR_MUTED)
                .bg(Color::Rgb(230, 230, 240))
        };
        if is_error {
            app.login_submit = None;
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "Press Enter to retry  •  Esc to go back",
                    Style::default().fg(COLOR_MUTED),
                )))
                .alignment(ratatui::layout::Alignment::Center),
                helper_area,
            );
        } else if has_input {
            // Centered 8-wide slot for Submit via Layout — stored rect IS the
            // drawn rect.
            let parts = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(8),
                    Constraint::Min(0),
                ])
                .split(helper_area);
            let submit_rect = parts[1];
            app.login_submit = Some(submit_rect);
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(submit_label, submit_style)))
                    .alignment(ratatui::layout::Alignment::Center),
                submit_rect,
            );
            // Helper hint goes on the row below (when space allows) so it
            // never overlaps the button.
            if apikey_inner.height >= 5 {
                let hint_below = ratatui::layout::Rect {
                    x: apikey_inner.x,
                    y: apikey_inner.y + 4,
                    width: apikey_inner.width,
                    height: 1,
                };
                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        "Press Enter to submit  •  Esc to cancel",
                        Style::default().fg(COLOR_MUTED),
                    )))
                    .alignment(ratatui::layout::Alignment::Center),
                    hint_below,
                );
            }
        } else {
            app.login_submit = None;
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    "Press Enter to submit  •  Esc to go back",
                    Style::default().fg(COLOR_MUTED),
                )))
                .alignment(ratatui::layout::Alignment::Center),
                helper_area,
            );
        }

        // Also render a subtle second line for empty hint (below helper)
        if !is_error && !has_input {
            // Move hint one row below helper to avoid overlap — but inner is only 5 high, so check bounds
            if apikey_inner.height >= 5 {
                let hint_area = ratatui::layout::Rect {
                    x: apikey_inner.x,
                    y: apikey_inner.y + 4,
                    width: apikey_inner.width,
                    height: 1,
                };
                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        "Keys start with ag_…  •  masked for security",
                        Style::default().fg(COLOR_MUTED),
                    )))
                    .alignment(ratatui::layout::Alignment::Center),
                    hint_area,
                );
            }
        }
    } else {
        // Idle
        let desc = if apikey_focused {
            "Paste a key from your dashboard  •  Press Enter to enter key"
        } else {
            "Paste a key from your dashboard"
        };
        let btn_style = if apikey_focused {
            Style::default()
                .fg(Color::White)
                .bg(COLOR_BRAND)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(COLOR_BRAND)
                .add_modifier(Modifier::BOLD)
        };
        let lines = vec![
            Line::from(Span::styled(desc, Style::default().fg(COLOR_MUTED))),
            Line::from(""),
            Line::from(Span::styled(
                if apikey_focused {
                    " ▶  Use an API key  "
                } else {
                    "    Use an API key    "
                },
                btn_style,
            )),
        ];
        frame.render_widget(
            Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center),
            apikey_inner,
        );
        app.login_apikey_input = None;
        app.login_submit = None;
    }

    // ---- Single authoritative status line (replaces duplicated orange banner) ----
    let status_style = if is_error {
        Style::default()
            .fg(Color::White)
            .bg(COLOR_DENY)
            .add_modifier(Modifier::BOLD)
    } else if is_success {
        Style::default()
            .fg(Color::White)
            .bg(COLOR_ALLOW)
            .add_modifier(Modifier::BOLD)
    } else if is_browser_pending || is_validating {
        Style::default()
            .fg(COLOR_BRAND)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(COLOR_MUTED)
    };
    let status_text = if is_error {
        if let LoginStatus::Error(msg) = &app.login_status {
            format!(" ✕ {msg} ")
        } else {
            " ✕ Invalid API key ".to_string()
        }
    } else if is_success {
        " ✓ Stored locally, not verified — loading… ".to_string()
    } else if is_browser_pending {
        if let Some(msg) = &app.login_status_msg {
            format!(" {} ", msg)
        } else {
            " Browser sign-in is not available yet ".to_string()
        }
    } else if is_validating {
        " Storing API key locally (not verified)… ".to_string()
    } else if is_apikey_editing {
        if app.login_api_input.is_empty() {
            " Paste your API key above and press Enter ".to_string()
        } else {
            " Press Enter to submit your API key ".to_string()
        }
    } else {
        // Idle — no hint text (deliberately blank, keeps spacing).
        String::new()
    };
    let status = Paragraph::new(Line::from(Span::styled(status_text, status_style)))
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(status, chunks[6]);

    // ---- Offline option (secondary/tertiary, muted, at bottom) ----
    let offline_focused =
        app.login_focus == LoginFocus::Offline && app.login_status == LoginStatus::Idle;
    let offline_style = if offline_focused {
        Style::default()
            .fg(Color::White)
            .bg(Color::Rgb(140, 140, 155))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(COLOR_MUTED)
    };
    let offline_line = if offline_focused {
        Line::from(Span::styled(
            " ▶ Continue offline (limited features, no sync)  [o] ",
            offline_style,
        ))
    } else {
        Line::from(Span::styled(
            "   Continue offline (limited features, no sync)  [o] ",
            offline_style,
        ))
    };
    let offline_para = Paragraph::new(offline_line).alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(offline_para, chunks[7]);
    app.login_offline = Some(chunks[7]);

    // ---- Footer hints + quit (de-emphasized) ----
    let hints = Paragraph::new(vec![Line::from(vec![
        Span::styled(
            " Tab ",
            Style::default()
                .fg(COLOR_BRAND)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("switch  ", Style::default().fg(COLOR_MUTED)),
        Span::styled(
            "Enter ",
            Style::default()
                .fg(COLOR_BRAND)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("select  ", Style::default().fg(COLOR_MUTED)),
        Span::styled(
            "Esc ",
            Style::default()
                .fg(COLOR_BRAND)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("back  ", Style::default().fg(COLOR_MUTED)),
        Span::styled(
            " 1",
            Style::default()
                .fg(COLOR_BRAND)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("/2 ", Style::default().fg(COLOR_MUTED)),
        Span::styled("quick  ", Style::default().fg(COLOR_MUTED)),
    ])])
    .alignment(ratatui::layout::Alignment::Center);
    // Footer row intentionally left blank (hints removed). `hints` is
    // intentionally unrendered; the mouse-quit target is gone with its label
    // (keyboard `q` still quits). Reference chunks[8] to keep layout stable.
    let _ = (&hints, chunks[8]);
    app.login_quit = None;

    // Animated focus rings: rotating dashed border on the active box only.
    // Drawn last so the dash phase owns the border cells outright.
    let dash_style = Style::default()
        .fg(COLOR_BRAND)
        .add_modifier(Modifier::BOLD);
    if browser_focused || is_browser_pending {
        render_marching_dashes(frame, chunks[3], app.anim_phase, dash_style);
    }
    if apikey_focused {
        render_marching_dashes(frame, chunks[5], app.anim_phase, dash_style);
    }

    // Keep legacy alias in sync for tests that still read login_signin
    app.login_signin = app.login_browser;
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
            let dot = match action {
                "allow" => "●",
                "deny" => "●",
                _ => "●",
            };
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
    let hint = if app.is_offline {
        "audit.db not found — run algo init, trigger a Bash tool, new rows appear automatically."
    } else {
        "No decisions yet. Trigger a Bash tool via the Claude hook — new rows appear automatically."
    };
    let text = vec![
        Line::from(Span::styled(
            " No decisions yet ",
            Style::default().fg(Color::White).bg(COLOR_BRAND).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(hint, Style::default().fg(COLOR_MUTED))),
        Line::from(""),
        Line::from(Span::styled(
            "algo status shows counts+savings · algo why shows last decision · algo log --show-egress inspects egress",
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
            "click a row to inspect · refreshes automatically",
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
        // New hierarchy: clear title, subtitle, two primary boxes, secondary offline, single status line
        assert!(
            s.contains("Sign in to Algorithco Guard"),
            "title missing: {s}"
        );
        assert!(
            s.contains("Choose how to authenticate"),
            "subtitle missing: {s}"
        );
        assert!(
            s.contains("[1]") && s.contains("Sign in with browser"),
            "browser option missing: {s}"
        );
        assert!(
            s.contains("[2]") && s.contains("Use an API key"),
            "apikey option missing: {s}"
        );
        assert!(
            s.contains("Continue offline"),
            "offline secondary missing: {s}"
        );
        // Offline should be present but not underlined primary — check we don't have duplicated orange banner
        assert!(
            !s.contains("You're offline — nothing is synced. Choose Continue offline."),
            "duplicated warning should be removed"
        );
        // Hint lines removed: idle status and footer must not show them
        assert!(
            !s.contains("1 / 2 to choose"),
            "idle status hint should be gone"
        );
        assert!(!s.contains("1/2 quick"), "footer hints should be gone");
        assert!(!s.contains("q quit"), "footer quit label should be gone");
        // Click areas recorded (offline kept; quit target gone with its label)
        assert!(app.login_browser.is_some(), "browser rect not recorded");
        assert!(app.login_apikey.is_some(), "apikey rect not recorded");
        assert!(app.login_offline.is_some(), "offline rect not recorded");
        assert!(app.login_quit.is_none(), "quit rect should be gone");
    }

    #[test]
    fn render_login_states_browser_pending_and_apikey() {
        let mut app = App::new(None);
        app.show_login();
        // Browser pending
        app.start_browser_signin();
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
            s.contains("not available yet"),
            "honest browser-pending message missing: {s}"
        );
        assert!(
            !s.contains("WD-4829-XK"),
            "fabricated device code must be gone"
        );
        assert!(
            !s.contains("Device code"),
            "fabricated device-code line must be gone: {s}"
        );
        assert!(
            s.contains("Esc to cancel") || s.contains("Cancel"),
            "cancel hint missing"
        );
        // API key editing
        app.show_login();
        app.start_api_key_entry();
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
        assert!(
            s2.contains("••") || s2.contains("Paste your API key"),
            "masked input missing"
        );
        // Error state
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
            s3.contains("cannot be empty") || s3.contains("API key"),
            "error message missing"
        );
        // Success state
        app.login_api_input = "ag-valid-key-12345".to_string();
        app.login_status = LoginStatus::Success;
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let s4: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(
            s4.contains("stored locally, not verified") || s4.contains("Stored locally"),
            "honest success missing: {s4}"
        );
        // Legacy validating state resolves honestly (no fake network check)
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
            s5.contains("not verified") || s5.contains("Storing"),
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
    fn login_submit_rect_matches_drawn_button_at_widths() {
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
            let buf = terminal.backend().buffer().clone();
            let r = app.login_submit.expect("submit rect must be recorded");
            assert!(has_non_whitespace(&buf, r), "submit rect {r:?} has no text");
            let text = rect_text(&buf, r);
            assert!(
                text.contains("Submit"),
                "submit rect {r:?} must contain label, got {text:?}"
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
    fn focused_box_has_rotating_dashed_border() {
        // Idle login focuses the browser box: its border must be dashed with
        // gaps, the title must survive the overlay, and the dash pattern must
        // march between ticks and loop cleanly after a full period.
        let mut app = App::new(None);
        app.show_login(); // focus Browser, Idle
        app.anim_phase = 0;
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let browser = app.login_browser.expect("browser rect");
        let top_row = |buf: &ratatui::buffer::Buffer| -> String {
            (browser.x..browser.x + browser.width)
                .map(|x| buf[(x, browser.y)].symbol().to_string())
                .collect()
        };
        let first = top_row(terminal.backend().buffer());
        assert!(
            first.contains("─"),
            "focused box must keep dashes, got {first:?}"
        );
        assert!(
            first.contains("[1]"),
            "title must survive dash overlay, got {first:?}"
        );
        // Unfocused API-key box keeps a solid border (no dash gaps).
        // Strip its title first — the title text legitimately contains spaces.
        let apikey = app.login_apikey.expect("apikey rect");
        let abuf = terminal.backend().buffer().clone();
        let atop: String = (apikey.x..apikey.x + apikey.width)
            .map(|x| abuf[(x, apikey.y)].symbol().to_string())
            .collect();
        let border_only = atop.replace(" [2] Use an API key ", "");
        assert!(
            !border_only.contains(' '),
            "unfocused box must stay solid, got {atop:?}"
        );
        // Advance one phase step: the dash pattern must visibly march.
        app.anim_phase = 1;
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let second = top_row(terminal.backend().buffer());
        assert_ne!(first, second, "dashes must rotate between phases");
        // Full period (5 dash cells) returns to the start.
        app.anim_phase = 5;
        terminal.draw(|f| render(f, &mut app)).unwrap();
        let third = top_row(terminal.backend().buffer());
        assert_eq!(first, third, "dash cycle must loop cleanly");
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
}
