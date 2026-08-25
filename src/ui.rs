//! Rendering layer (pure view code).
//!
//! Every function here takes a read-only snapshot of `App` and paints one
//! frame. Nothing in this module mutates state — all input handling lives in
//! `main.rs`, all logic in `app.rs`. This makes it safe to redesign layouts
//! without touching behavior.
//!
//! Layout (top to bottom):
//!   ┌ header: app name + status pill + theme name ┐
//!   ├ big ASCII stopwatch + start time / hint     ┤
//!   └ sessions table (#, START, END, FOCUSED)     ┘
//!   [ ? ] centered help popup when toggled

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Row, Table, TableState},
};

use crate::app::{App, State};

// ---------------------------------------------------------------------------
// Frame composition
// ---------------------------------------------------------------------------

/// Draw one complete frame. This is the only entry point used by main.rs.
pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    // Responsive vertical split: on short terminals (<20 rows) shrink the
    // timer area; otherwise let it grow a little with available height.
    let [header_h, timer_h, table_h] = if area.height < 20 {
        [
            Constraint::Length(3),
            Constraint::Length(7),
            Constraint::Min(3),
        ]
    } else {
        // 7 rows minimum for the 5-row digits + borders; grows up to 9.
        let big = 9u16.min((area.height.saturating_sub(10)) / 2 + 7).max(7);
        [
            Constraint::Length(3),
            Constraint::Length(big),
            Constraint::Min(4),
        ]
    };
    let [header, timer_area, table_area] =
        Layout::vertical([header_h, timer_h, table_h]).areas(area);

    draw_header(f, app, header);
    draw_timer(f, app, timer_area);
    draw_sessions(f, app, table_area);

    if app.show_help {
        draw_help(f, app);
    }
}

/// Top bar: "gavani" brand + colored status pill + active theme name.
fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let t = app.theme();
    // Status color encodes the state machine at a glance.
    let (label, color) = match &app.state {
        State::Focusing { .. } => ("FOCUSING", t.active),
        State::Paused { .. } => ("PAUSED", t.paused),
        State::Idle => ("IDLE", t.muted),
    };
    let title = Line::from(vec![
        Span::styled(
            " gavani ",
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            format!("● {label}"),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ]);
    f.render_widget(
        Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(t.border))
            .title(title)
            .title_bottom(
                Line::from(Span::styled(
                    format!(" {} ", t.name),
                    Style::default().fg(t.muted),
                ))
                .right_aligned(),
            ),
        area,
    );
}

// ---------------------------------------------------------------------------
// Big ASCII timer
// ---------------------------------------------------------------------------

/// Format seconds as HH:MM:SS (works past 99h since hours are unbounded).
pub fn fmt_hms(secs: u64) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

/// 5-row block glyphs, one entry per character the clock can show.
const GLYPHS: [(char, [&str; 5]); 11] = [
    ('0', ["███", "█ █", "█ █", "█ █", "███"]),
    ('1', [" █ ", "██ ", " █ ", " █ ", "███"]),
    ('2', ["███", "  █", "███", "█  ", "███"]),
    ('3', ["███", "  █", "███", "  █", "███"]),
    ('4', ["█ █", "█ █", "███", "  █", "  █"]),
    ('5', ["███", "█  ", "███", "  █", "███"]),
    ('6', ["███", "█  ", "███", "█ █", "███"]),
    ('7', ["███", "  █", "  █", "  █", "  █"]),
    ('8', ["███", "█ █", "███", "█ █", "███"]),
    ('9', ["███", "█ █", "███", "  █", "███"]),
    (':', ["   ", " █ ", "   ", " █ ", "   "]),
];

/// Look up the glyph for a character (falls back to '0' defensively).
fn glyph(ch: char) -> &'static [&'static str; 5] {
    GLYPHS
        .iter()
        .find(|(c, _)| *c == ch)
        .map(|(_, g)| g)
        .unwrap_or(&GLYPHS[0].1)
}

/// Convert "HH:MM:SS" into 5 Lines of joined block-glyph rows so the clock
/// reads like a giant segmented display.
fn big_timer_lines(text: &str, color: Style) -> Vec<Line<'static>> {
    let cols: Vec<_> = text.chars().map(glyph).collect();
    (0..5)
        .map(|row| {
            // Join each glyph's row with a single space between characters.
            let s: String = cols
                .iter()
                .enumerate()
                .map(|(i, g)| {
                    let mut cell = g[row].to_string();
                    if i + 1 < cols.len() {
                        cell.push(' ');
                    }
                    cell
                })
                .collect();
            Line::from(Span::styled(s, color))
        })
        .collect()
}

/// Center block: giant digits while running/paused, muted placeholder + hint
/// while idle.
fn draw_timer(f: &mut Frame, app: &App, area: Rect) {
    let t = app.theme();
    let inner = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border))
        .title(" Focus Timer ")
        .title_style(Style::default().fg(t.muted));

    match app.timer() {
        Some((elapsed, paused)) => {
            // Green while focusing, amber while paused — glanceable state.
            let color = if paused { t.paused } else { t.active };
            let mut lines = big_timer_lines(&fmt_hms(elapsed), Style::default().fg(color));
            lines.push(Line::default());
            lines.push(Line::from(Span::styled(
                match &app.state {
                    State::Paused { .. } => "paused — press p to resume".to_string(),
                    _ => match app.state.started_at() {
                        Some(s) => format!("started at {}", app.config.fmt_time(&s)),
                        None => String::new(),
                    },
                },
                Style::default().fg(t.muted),
            )));
            f.render_widget(
                Paragraph::new(lines)
                    .block(inner)
                    .alignment(Alignment::Center),
                area,
            );
        }
        None => {
            let mut lines = vec![
                Line::styled("00:00:00", Style::default().fg(t.muted)),
                Line::default(),
                Line::styled("press s to start focusing", Style::default().fg(t.muted)),
            ];
            if !app.sessions.is_empty() {
                lines.push(Line::styled(
                    format!("{} session(s) recorded", app.sessions.len()),
                    Style::default().fg(t.muted),
                ));
            }
            f.render_widget(
                Paragraph::new(lines)
                    .block(inner)
                    .alignment(Alignment::Center),
                area,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Session history table
// ---------------------------------------------------------------------------

/// Bottom block: scrollable session history with a highlighted selected row.
/// Falls back to a compact 2-column layout on narrow terminals.
fn draw_sessions(f: &mut Frame, app: &App, area: Rect) {
    let t = app.theme();
    let compact = area.width < 40;

    let widths: Vec<Constraint> = if compact {
        vec![Constraint::Length(6), Constraint::Fill(1)]
    } else {
        vec![
            Constraint::Length(6),
            Constraint::Length(14), // roomy enough for "05:38:00 PM"
            Constraint::Length(14),
            Constraint::Fill(1),
        ]
    };

    let header_cells: Vec<&str> = if compact {
        vec!["#", "FOCUSED"]
    } else {
        vec!["#", "START", "END", "FOCUSED"]
    };
    let header =
        Row::new(header_cells).style(Style::default().fg(t.accent).add_modifier(Modifier::BOLD));

    let rows = app.sessions.iter().enumerate().map(|(i, s)| {
        let cells: Vec<String> = if compact {
            vec![(i + 1).to_string(), fmt_hms(s.duration_secs)]
        } else {
            vec![
                (i + 1).to_string(),
                app.config.fmt_time(&s.start),
                app.config.fmt_time(&s.end),
                fmt_hms(s.duration_secs),
            ]
        };
        Row::new(cells).style(Style::default().fg(t.text))
    });

    let table = Table::new(rows, widths.clone())
        .header(header)
        .block(
            Block::new()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(t.border))
                .title(" Sessions "),
        )
        .row_highlight_style(
            Style::default()
                .bg(t.highlight_bg)
                .fg(t.accent)
                .add_modifier(Modifier::BOLD),
        )
        .widths(widths);
    f.render_stateful_widget(
        table,
        area,
        &mut TableState::default().with_selected(app.selected),
    );
}

// ---------------------------------------------------------------------------
// Help popup
// ---------------------------------------------------------------------------

/// Centered modal listing every keybinding. Rendered last so it floats above
/// everything else (`Clear` wipes the cells underneath first).
fn draw_help(f: &mut Frame, app: &App) {
    let t = app.theme();
    let area = centered_rect(f.area(), 48, 14);
    let keys = [
        ("s", "start or stop (record) session"),
        ("p", "pause or resume"),
        ("r", "reset — discard current session"),
        ("j / k, ↑/↓", "navigate session history"),
        ("d", "delete selected session"),
        ("t", "cycle themes (saved to config)"),
        ("?", "toggle this help"),
        ("q / Ctrl+C", "quit"),
    ];
    let lines: Vec<Line> = keys
        .iter()
        .map(|(k, v)| {
            Line::from(vec![
                Span::styled(format!("{k:<12}"), Style::default().fg(t.accent)),
                Span::styled(*v, Style::default().fg(t.text)),
            ])
        })
        .collect();
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines)
            .block(
                Block::new()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(t.accent))
                    .title(" keybindings "),
            )
            .alignment(Alignment::Left),
        area,
    );
}

/// Shrink `total` to a w×h rectangle centered inside it.
fn centered_rect(total: Rect, w: u16, h: u16) -> Rect {
    let x = total.x + (total.width.saturating_sub(w)) / 2;
    let y = total.y + (total.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w.min(total.width), h.min(total.height))
}
