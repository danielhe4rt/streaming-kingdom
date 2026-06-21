//! The nav-shell chrome: topbar (primary sections), sidebar (contextual
//! sub-nav), and the status bar. Content is rendered by the section panes.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::application::AppState;

use super::super::nav::{self, Section, SubItem};
use super::super::service;
use super::super::state::TuiState;
use super::super::theme::*;
use super::format;

/// Top bar: the primary section tabs (left) and a live health strip (right).
/// The brand lives on the outer frame, so it isn't repeated here.
pub fn draw_topbar(frame: &mut Frame, area: Rect, app: &AppState, tui: &TuiState) {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(COLOR_BORDER));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [tabs_area, health_area] =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Min(0)]).areas(inner);

    // Left: section tabs.
    let mut tabs = vec![Span::raw(" ")];
    for section in Section::ALL {
        let selected = tui.nav.section == section;
        let style = if selected {
            Style::default()
                .fg(Color::Black)
                .bg(COLOR_PRIMARY)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(COLOR_MUTED)
        };
        tabs.push(Span::styled(format!(" {} ", section.label()), style));
        tabs.push(Span::raw(" "));
    }
    frame.render_widget(Paragraph::new(Line::from(tabs)), tabs_area);

    // Right: channel + key service health dots, always visible.
    let plain = |on: bool| {
        if on {
            ("●", COLOR_CONNECTED)
        } else {
            ("○", COLOR_INACTIVE)
        }
    };
    let channel = if tui.twitch_chat.channel.is_empty() {
        "—".to_string()
    } else {
        format!("#{}", tui.twitch_chat.channel)
    };
    let (es, es_c) = plain(tui.twitch_eventsub.connected);
    let (ch, ch_c) = plain(tui.twitch_chat.connected);
    let (ov, ov_c) = if tui.overlays.running {
        ("●", COLOR_CONNECTED)
    } else if app.overlays_enabled {
        ("●", COLOR_STARTING)
    } else {
        ("○", COLOR_INACTIVE)
    };

    let health = Line::from(vec![
        Span::styled(
            channel,
            Style::default()
                .fg(COLOR_ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("    "),
        Span::styled(format!("{es} "), Style::default().fg(es_c)),
        Span::styled("EventSub  ", Style::default().fg(COLOR_MUTED)),
        Span::styled(format!("{ch} "), Style::default().fg(ch_c)),
        Span::styled("Chat  ", Style::default().fg(COLOR_MUTED)),
        Span::styled(format!("{ov} "), Style::default().fg(ov_c)),
        Span::styled(
            format!("Overlays:{} ", tui.overlays_port),
            Style::default().fg(COLOR_MUTED),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(health).alignment(Alignment::Right),
        health_area,
    );
}

/// Sidebar: the contextual sub-nav for the active section, with the active
/// section name as a breadcrumb header and a live status dot per Service/Overlay.
pub fn draw_sidebar(frame: &mut Frame, area: Rect, app: &AppState, tui: &TuiState) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(COLOR_BORDER));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let section = tui.nav.section;
    let current = tui.nav.current_sub();

    let mut lines = vec![
        Line::from(Span::styled(
            format!(" ▸ {} ", section.label().to_uppercase()),
            Style::default()
                .fg(COLOR_ACCENT)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for item in nav::sub_items(section) {
        let selected = item == current;
        // Consistent selection bar: bright-purple ▌ + bold purple label.
        let (marker, style) = if selected {
            (
                "▌ ",
                Style::default()
                    .fg(COLOR_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            ("  ", Style::default().fg(COLOR_MUTED))
        };
        let mut spans = vec![Span::styled(marker, Style::default().fg(COLOR_PRIMARY))];
        // Live status dot for Service/Overlay items (keeps labels aligned).
        match sub_item_dot(item, app, tui) {
            Some((dot, color)) => {
                spans.push(Span::styled(format!("{dot} "), Style::default().fg(color)))
            }
            None => spans.push(Span::raw("  ")),
        }
        spans.push(Span::styled(sub_label_with_tag(item), style));
        lines.push(Line::from(spans));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

/// Live status dot for a sidebar sub-item, when it maps to a Service/Overlay.
fn sub_item_dot(item: SubItem, app: &AppState, tui: &TuiState) -> Option<(&'static str, Color)> {
    match item {
        SubItem::Service(id) => {
            let def = service::registry().iter().find(|s| s.id == id).copied()?;
            let (dot, color, _) = format::service_status(&def, app, tui);
            Some((dot, color))
        }
        SubItem::Overlay(_) => Some(if tui.overlays.running {
            ("●", COLOR_CONNECTED)
        } else {
            ("○", COLOR_INACTIVE)
        }),
        _ => None,
    }
}

/// Status bar with context-aware key hints.
pub fn draw_status_bar(frame: &mut Frame, area: Rect, app: &AppState) {
    if let Some(msg) = &app.status_message {
        let bar = Paragraph::new(format!(" {msg}")).style(
            Style::default()
                .fg(COLOR_ERROR)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_widget(bar, area);
    } else {
        let help = Line::from(vec![
            Span::styled(" q ", Style::default().fg(COLOR_ACCENT)),
            Span::styled("quit   ", Style::default().fg(COLOR_MUTED)),
            Span::styled("Tab ", Style::default().fg(COLOR_ACCENT)),
            Span::styled("section   ", Style::default().fg(COLOR_MUTED)),
            Span::styled("j/k ", Style::default().fg(COLOR_ACCENT)),
            Span::styled("sub-nav   ", Style::default().fg(COLOR_MUTED)),
            Span::styled("Space ", Style::default().fg(COLOR_ACCENT)),
            Span::styled("toggle   ", Style::default().fg(COLOR_MUTED)),
            Span::styled("J/K ", Style::default().fg(COLOR_ACCENT)),
            Span::styled("scroll   ", Style::default().fg(COLOR_MUTED)),
            Span::styled("1-6 ", Style::default().fg(COLOR_ACCENT)),
            Span::styled("filters", Style::default().fg(COLOR_MUTED)),
        ]);
        frame.render_widget(Paragraph::new(help), area);
    }
}

fn sub_label_with_tag(item: SubItem) -> String {
    nav::sub_label(item)
}
