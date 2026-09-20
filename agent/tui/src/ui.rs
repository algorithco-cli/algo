//! UI rendering — design tokens Variant 1, closest terminal approximation.
//!
//! Tokens from `plans/design-tokens.md` §1:
//!   brand  #6D4AFF  → `COLOR_BRAND`  (links, header)
//!   allow  #1E9E63  → `COLOR_ALLOW`  (success)
//!   ask    #D99A00  → `COLOR_ASK`    (warn)
//!   deny   #E5484D  → `COLOR_DENY`   (danger)
//! Exact hex where truecolor is available; otherwise nearest ANSI fallback
//! (green/yellow/red/blue) carries the same semantics per spec §92.

use crate::app::{App, ViewMode};
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

/// Render the full frame.
///
/// Layout (vertical):
///   1. Header (brand title + stats line)
///   2. Optional offline banner
///   3. Main area: table of recent decisions OR policy placeholder
///   4. Footer help line
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Reserve footer + header; offline banner only if needed.
    let has_offline = app.is_offline;
    let mut constraints = Vec::new();
    constraints.push(Constraint::Length(3)); // header
    if has_offline {
        constraints.push(Constraint::Length(3)); // offline banner
    }
    constraints.push(Constraint::Min(8)); // main
    constraints.push(Constraint::Length(1)); // footer

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut idx = 0;

    // Header: title + stats
    render_header(frame, app, chunks[idx]);
    idx += 1;

    // Offline banner (read-only when daemon down)
    if has_offline {
        render_offline_banner(frame, app, chunks[idx]);
        idx += 1;
    }

    // Main
    if app.mode == ViewMode::Policy {
        render_policy(frame, app, chunks[idx]);
    } else {
        render_table(frame, app, chunks[idx]);
    }
    idx += 1;

    // Footer
    render_footer(frame, chunks[idx]);
}

fn render_header(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let title = Line::from(vec![
        Span::styled(
            " algorithco guard ",
            Style::default()
                .fg(Color::White)
                .bg(COLOR_BRAND)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  Live decision feed  ",
            Style::default().fg(COLOR_BRAND).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            if app.is_offline {
                " (read-only)"
            } else {
                ""
            },
            Style::default().fg(COLOR_MUTED),
        ),
    ]);

    let stats = Line::from(vec![
        Span::styled(
            " allow ",
            Style::default().fg(COLOR_ALLOW).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("{}", app.counts.allow)),
        Span::styled("  │  ", Style::default().fg(COLOR_BORDER)),
        Span::styled(
            " ask ",
            Style::default().fg(COLOR_ASK).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("{}", app.counts.ask)),
        Span::styled("  │  ", Style::default().fg(COLOR_BORDER)),
        Span::styled(
            " block ",
            Style::default().fg(COLOR_DENY).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("{}", app.counts.deny)),
        Span::styled("  │  ", Style::default().fg(COLOR_BORDER)),
        Span::styled(
            " would-have-blocked ",
            Style::default().fg(COLOR_MUTED),
        ),
        Span::styled(
            format!("{}", app.counts.would_have_blocked),
            Style::default().fg(COLOR_DENY).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  (shadow: {})", app.counts.shadow),
            Style::default().fg(COLOR_MUTED),
        ),
        Span::styled(
            format!("  total: {}", app.counts.total),
            Style::default().fg(COLOR_MUTED),
        ),
    ]);

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BORDER))
        .title_style(Style::default().fg(COLOR_BRAND));

    let paragraph = Paragraph::new(vec![title, stats])
        .block(header_block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

fn render_offline_banner(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let msg = if let Some(err) = &app.error {
        format!(" Offline (read-only) — daemon down or audit.db unavailable: {err} — TUI crash does not block hooks (read-only audit.db).")
    } else {
        " Offline (read-only) — daemon down or audit.db not found. Showing last cached state if available. TUI crash does not block hooks (read-only audit.db).".to_string()
    };
    let banner = Paragraph::new(Line::from(vec![Span::styled(
        msg,
        Style::default()
            .fg(Color::Black)
            .bg(COLOR_ASK)
            .add_modifier(Modifier::BOLD),
    )]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_ASK))
            .title(" status "),
    )
    .wrap(Wrap { trim: false });
    frame.render_widget(banner, area);
}

fn render_table(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let header = Row::new(vec![
        Cell::from("ts").style(Style::default().fg(COLOR_MUTED).add_modifier(Modifier::BOLD)),
        Cell::from("action").style(Style::default().fg(COLOR_MUTED).add_modifier(Modifier::BOLD)),
        Cell::from("reason").style(Style::default().fg(COLOR_MUTED).add_modifier(Modifier::BOLD)),
        Cell::from("source").style(Style::default().fg(COLOR_MUTED).add_modifier(Modifier::BOLD)),
        Cell::from("latency").style(Style::default().fg(COLOR_MUTED).add_modifier(Modifier::BOLD)),
    ])
    .height(1)
    .style(Style::default().bg(Color::Rgb(22, 21, 31)));

    let rows: Vec<Row> = app
        .entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let action = e.action_str();
            let color = action_color(action);
            let style = if i == app.selected {
                Style::default()
                    .fg(Color::White)
                    .bg(COLOR_BRAND)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(format_ts(e.ts)).style(style),
                Cell::from(action).style(Style::default().fg(color).add_modifier(Modifier::BOLD)),
                Cell::from(e.reason.clone()).style(style),
                Cell::from(e.source_str()).style(style.fg(COLOR_MUTED)),
                Cell::from(format!("{}ms", e.latency_ms)).style(style),
            ])
            .height(1)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(19),
            Constraint::Length(7),
            Constraint::Percentage(50),
            Constraint::Length(12),
            Constraint::Length(9),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_BORDER))
            .title(" recent decisions (ts • action • reason • source • latency) "),
    )
    .row_highlight_style(Style::default().bg(COLOR_BRAND).fg(Color::White));

    frame.render_widget(table, area);
}

fn render_policy(frame: &mut Frame, _app: &App, area: ratatui::layout::Rect) {
    let text = vec![
        Line::from(Span::styled(
            "Policy editor — local rules, dry-run vs history (placeholder)",
            Style::default()
                .fg(COLOR_BRAND)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "• Local rules: per-workspace allow/ask/deny overrides (not yet editable in TUI).",
            Style::default().fg(COLOR_MUTED),
        )),
        Line::from(Span::styled(
            "• Dry-run: evaluate a command or file path against current policy without enforcing.",
            Style::default().fg(COLOR_MUTED),
        )),
        Line::from(Span::styled(
            "• History compare: re-evaluate past decisions (audit.db) against a policy snapshot.",
            Style::default().fg(COLOR_MUTED),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Read-only when daemon down. ", Style::default().fg(COLOR_ASK)),
            Span::styled(
                "This view never writes to the decision path.",
                Style::default().fg(COLOR_MUTED),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press 'p' to return to feed, 'r' to refresh, 'q' to quit.",
            Style::default().fg(COLOR_MUTED),
        )),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BRAND))
        .title(" policy (read-only placeholder) ");
    let para = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
    frame.render_widget(para, area);
}

fn render_footer(frame: &mut Frame, area: ratatui::layout::Rect) {
    let help = Line::from(vec![
        Span::styled(" q ", Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)),
        Span::raw("quit "),
        Span::styled(" j/k ", Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)),
        Span::raw("nav "),
        Span::styled(" r ", Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)),
        Span::raw("refresh "),
        Span::styled(" p ", Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)),
        Span::raw("policy "),
        Span::styled(" ↑/↓ also navigates ", Style::default().fg(COLOR_MUTED)),
    ]);
    let p = Paragraph::new(help);
    frame.render_widget(p, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
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
        let app = App::new(None);
        terminal
            .draw(|f| render(f, &app))
            .expect("render should not panic on empty offline");
        let buffer = terminal.backend().buffer();
        // Header should contain brand title text
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
        store.insert(&decision(Action::Allow), "fp1", false).unwrap();
        store.insert(&decision(Action::Deny), "fp2", true).unwrap();
        store.insert(&decision(Action::Ask), "fp3", false).unwrap();

        let mut app = App::new(Some(store));
        app.refresh();

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| render(f, &app))
            .expect("render should not panic with entries");

        // Also test policy mode
        app.toggle_policy();
        terminal
            .draw(|f| render(f, &app))
            .expect("render policy should not panic");
    }

    #[test]
    fn render_offline_banner_shown() {
        let mut app = App::new(None);
        app.is_offline = true;
        app.error = Some("simulated daemon down".to_string());
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(f, &app)).unwrap();
        let backend_ref = terminal.backend();
        let s: String = backend_ref
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(s.contains("Offline"));
    }

    #[test]
    fn format_ts_handles_zero() {
        let s = format_ts(0);
        assert!(!s.is_empty());
    }
}
