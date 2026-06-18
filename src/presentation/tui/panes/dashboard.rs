//! Dashboard section pane: a mission-control overview that fills the screen —
//! KPIs across the top, Services + a live Recent-Activity feed on the left, and
//! Overlays + Highlights on the right.

use ratatui::prelude::*;
use ratatui::widgets::{Paragraph, Wrap};

use crate::application::AppState;

use super::super::nav::{self, OverlayId};
use super::super::service;
use super::super::state::TuiState;
use super::super::theme::*;
use super::format;

/// Dashboard → Overview.
pub fn draw(frame: &mut Frame, area: Rect, app: &AppState, tui: &TuiState) {
    let [kpi_area, body] =
        Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(area);

    draw_kpis(frame, kpi_area, app);

    let [left, right] =
        Layout::horizontal([Constraint::Percentage(62), Constraint::Min(0)]).areas(body);

    let [services_area, activity_area] =
        Layout::vertical([Constraint::Length(12), Constraint::Min(0)]).areas(left);
    let [overlays_area, highlights_area] =
        Layout::vertical([Constraint::Length(7), Constraint::Min(0)]).areas(right);

    draw_services_summary(frame, services_area, app, tui);
    draw_recent_activity(frame, activity_area, app);
    draw_overlays_summary(frame, overlays_area, app, tui);
    draw_highlights_summary(frame, highlights_area, tui);
}

fn draw_kpis(frame: &mut Frame, area: Rect, app: &AppState) {
    let uptime = app.started_at.elapsed();
    let hours = uptime.as_secs() / 3600;
    let mins = (uptime.as_secs() % 3600) / 60;
    let secs = uptime.as_secs() % 60;

    let block = format::make_block("Stream Stats", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let label = |t: &'static str| Span::styled(t, Style::default().fg(COLOR_MUTED));
    let stat = |v: String, c: Color| {
        Span::styled(
            format!("{v}     "),
            Style::default().fg(c).add_modifier(Modifier::BOLD),
        )
    };
    let line = Line::from(vec![
        label("  VIEWERS "),
        stat(app.stats.viewer_count.to_string(), COLOR_ACCENT),
        label("FOLLOWERS "),
        stat(app.stats.followers_today.to_string(), COLOR_FOLLOW),
        label("SUBS "),
        stat(app.stats.subs_today.to_string(), COLOR_SUB),
        label("UPTIME "),
        Span::styled(
            format!("{hours:02}:{mins:02}:{secs:02}"),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(line), inner);
}

fn draw_services_summary(frame: &mut Frame, area: Rect, app: &AppState, tui: &TuiState) {
    let block = format::make_block("Services", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(format::section_label("Inputs"));
    for def in service::inputs() {
        lines.push(format::service_line(def, app, tui, false));
    }
    lines.push(format::section_label("Outputs"));
    for def in service::outputs() {
        lines.push(format::service_line(def, app, tui, false));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn draw_recent_activity(frame: &mut Frame, area: Rect, app: &AppState) {
    let block = format::make_block("Recent Activity", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.event_log.is_empty() {
        frame.render_widget(empty_state("Waiting for events…"), inner);
        return;
    }

    let lines: Vec<Line> = app
        .event_log
        .iter()
        .rev()
        .map(format::format_app_event)
        .collect();
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}

fn draw_overlays_summary(frame: &mut Frame, area: Rect, app: &AppState, tui: &TuiState) {
    let block = format::make_block("Overlays", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (dot, color, label) = if tui.overlays.running {
        ("●", COLOR_CONNECTED, "RUNNING")
    } else if app.overlays_enabled {
        ("●", COLOR_STARTING, "STARTING…")
    } else {
        ("○", COLOR_INACTIVE, "STOPPED")
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled(format!("{dot} "), Style::default().fg(color)),
            Span::styled(
                "Overlay server ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                label.to_string(),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];
    for o in OverlayId::ALL.iter() {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{:<6}", o.name),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                nav::overlay_url(o, tui.overlays_port),
                Style::default().fg(COLOR_ACCENT),
            ),
        ]));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn draw_highlights_summary(frame: &mut Frame, area: Rect, tui: &TuiState) {
    let block = format::make_block("Highlights", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if tui.highlights.is_empty() {
        frame.render_widget(
            empty_state("No highlights yet — subs, donations and raids land here."),
            inner,
        );
        return;
    }

    let lines: Vec<Line> = tui
        .highlights
        .iter()
        .rev()
        .map(|h| {
            Line::from(vec![
                Span::styled(format!("{} ", h.icon), Style::default().fg(h.color)),
                Span::raw(h.text.clone()),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}

/// A muted, italic placeholder for an empty panel.
fn empty_state(msg: &str) -> Paragraph<'static> {
    Paragraph::new(format!("  {msg}"))
        .style(
            Style::default()
                .fg(COLOR_MUTED)
                .add_modifier(Modifier::ITALIC),
        )
        .wrap(Wrap { trim: true })
}
