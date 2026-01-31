use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use crate::app::{AppState, StreamEvent};
use super::{Pane, TuiState};

const TOGGLE_LABELS: [&str; 3] = ["Waybar", "Privacy Monitor", "Alerts Browser"];

pub fn draw(frame: &mut Frame, app: &AppState, tui: &TuiState) {
    let [main_area, status_bar] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    let [left, center, right] = Layout::horizontal([
        Constraint::Percentage(25),
        Constraint::Percentage(35),
        Constraint::Percentage(40),
    ])
    .areas(main_area);

    draw_toggles(frame, left, app, tui);
    draw_stats(frame, center, app, tui);
    draw_event_log(frame, right, app, tui);
    draw_status_bar(frame, status_bar, app);
}

// ---------------------------------------------------------------------------
// Left panel: feature toggles
// ---------------------------------------------------------------------------

fn draw_toggles(frame: &mut Frame, area: Rect, app: &AppState, tui: &TuiState) {
    let focused = tui.focused_pane == Pane::Toggles;
    let block = make_block("Features", focused);

    let enabled = [app.waybar_enabled, app.privacy_enabled, app.alerts_enabled];

    let items: Vec<ListItem> = TOGGLE_LABELS
        .iter()
        .zip(enabled.iter())
        .enumerate()
        .map(|(i, (label, &on))| {
            let checkbox = if on { "[x]" } else { "[ ]" };
            let marker = if focused && i == tui.toggle_cursor {
                ">"
            } else {
                " "
            };
            let style = if focused && i == tui.toggle_cursor {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else if on {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            ListItem::new(format!("{marker} {checkbox} {label}")).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

// ---------------------------------------------------------------------------
// Center panel: live stream stats
// ---------------------------------------------------------------------------

fn draw_stats(frame: &mut Frame, area: Rect, app: &AppState, tui: &TuiState) {
    let focused = tui.focused_pane == Pane::Stats;
    let block = make_block("Stream Stats", focused);

    let uptime = app.started_at.elapsed();
    let hours = uptime.as_secs() / 3600;
    let mins = (uptime.as_secs() % 3600) / 60;
    let secs = uptime.as_secs() % 60;

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Viewers:  ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{}", app.stats.viewer_count)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Followers: ", Style::default().fg(Color::Magenta)),
            Span::raw(format!("{}", app.stats.followers_today)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Subs:      ", Style::default().fg(Color::Green)),
            Span::raw(format!("{}", app.stats.subs_today)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Uptime:    ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{hours:02}:{mins:02}:{secs:02}")),
        ]),
    ];

    let paragraph = Paragraph::new(text).block(block);
    frame.render_widget(paragraph, area);
}

// ---------------------------------------------------------------------------
// Right panel: scrolling event log
// ---------------------------------------------------------------------------

fn draw_event_log(frame: &mut Frame, area: Rect, app: &AppState, tui: &TuiState) {
    let focused = tui.focused_pane == Pane::EventLog;
    let block = make_block("Event Log", focused);

    let uptime_base = app.started_at;
    let lines: Vec<Line> = app
        .stats
        .last_events
        .iter()
        .rev()
        .map(|ev| format_event(ev, uptime_base))
        .collect();

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true })
        .scroll((tui.event_log_scroll, 0));

    frame.render_widget(paragraph, area);
}

fn format_event(event: &StreamEvent, base: std::time::Instant) -> Line<'static> {
    let elapsed = base.elapsed();
    let ts = format!(
        "{:02}:{:02}:{:02}",
        elapsed.as_secs() / 3600,
        (elapsed.as_secs() % 3600) / 60,
        elapsed.as_secs() % 60
    );

    let (icon, body) = match event {
        StreamEvent::Follow { username } => ("♥", format!("{username} followed")),
        StreamEvent::Sub {
            username,
            tier,
            months,
        } => (
            "★",
            format!("{username} subbed ({tier:?}, {months}mo)"),
        ),
        StreamEvent::Donation {
            username,
            amount_cents,
            message,
        } => (
            "$",
            format!(
                "{username} donated ${:.2}: {message}",
                *amount_cents as f64 / 100.0
            ),
        ),
        StreamEvent::Cheer {
            username,
            bits,
            message,
        } => ("◆", format!("{username} cheered {bits} bits: {message}")),
        StreamEvent::Raid {
            from_channel,
            viewers,
        } => ("⚡", format!("{from_channel} raided with {viewers} viewers")),
        StreamEvent::ViewerCountUpdate { count } => {
            ("👁", format!("Viewers updated to {count}"))
        }
    };

    Line::from(vec![
        Span::styled(format!("[{ts}] "), Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{icon} "), Style::default().fg(Color::Yellow)),
        Span::raw(body),
    ])
}

// ---------------------------------------------------------------------------
// Status bar
// ---------------------------------------------------------------------------

fn draw_status_bar(frame: &mut Frame, area: Rect, app: &AppState) {
    if let Some(msg) = &app.status_message {
        let bar = Paragraph::new(format!(" {msg}"))
            .style(Style::default().fg(Color::Red).bg(Color::Black));
        frame.render_widget(bar, area);
    } else {
        let help = " q: quit | j/k: navigate | Space/Enter: toggle | Tab: switch pane";
        let bar =
            Paragraph::new(help).style(Style::default().fg(Color::DarkGray).bg(Color::Black));
        frame.render_widget(bar, area);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_block(title: &str, focused: bool) -> Block<'_> {
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_style(border_style)
}
