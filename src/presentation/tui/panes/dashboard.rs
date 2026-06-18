//! Dashboard section pane: KPIs across the top, Services and Overlays summaries.

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
        Layout::vertical([Constraint::Length(4), Constraint::Min(0)]).areas(area);

    draw_kpis(frame, kpi_area, app);

    let [services_area, overlays_area] =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Min(0)]).areas(body);

    draw_services_summary(frame, services_area, app, tui);
    draw_overlays_summary(frame, overlays_area, app, tui);
}

fn draw_kpis(frame: &mut Frame, area: Rect, app: &AppState) {
    let uptime = app.started_at.elapsed();
    let hours = uptime.as_secs() / 3600;
    let mins = (uptime.as_secs() % 3600) / 60;
    let secs = uptime.as_secs() % 60;

    let block = format::make_block("Stream Stats", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let line = Line::from(vec![
        Span::styled("Viewers ", Style::default().fg(Color::Cyan)),
        Span::raw(format!("{}    ", app.stats.viewer_count)),
        Span::styled("Followers ", Style::default().fg(Color::Magenta)),
        Span::raw(format!("{}    ", app.stats.followers_today)),
        Span::styled("Subs ", Style::default().fg(Color::Green)),
        Span::raw(format!("{}    ", app.stats.subs_today)),
        Span::styled("Uptime ", Style::default().fg(Color::DarkGray)),
        Span::raw(format!("{hours:02}:{mins:02}:{secs:02}")),
    ]);
    frame.render_widget(Paragraph::new(line), inner);
}

fn draw_services_summary(frame: &mut Frame, area: Rect, app: &AppState, tui: &TuiState) {
    let block = format::make_block("Services", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(group_label("Inputs"));
    for def in service::inputs() {
        lines.push(format::service_line(def, app, tui, false));
    }
    lines.push(group_label("Outputs"));
    for def in service::outputs() {
        lines.push(format::service_line(def, app, tui, false));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
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
            Span::styled(label.to_string(), Style::default().fg(color)),
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
                Style::default().fg(COLOR_INFO),
            ),
        ]));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn group_label(label: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!(" {label} "),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ))
}
