//! The nav-shell chrome: topbar (primary sections), sidebar (contextual
//! sub-nav), and the status bar. Content is rendered by the section panes.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::application::AppState;

use super::super::nav::{self, Section, SubItem};
use super::super::state::TuiState;
use super::super::theme::*;

/// Top bar: brand, the primary section tabs, and live status chips.
pub fn draw_topbar(frame: &mut Frame, area: Rect, app: &AppState, tui: &TuiState) {
    let mut spans = vec![
        Span::styled(" ♥ streams-toolkit ", Style::default().fg(COLOR_SUB)),
        Span::raw("  "),
    ];

    for section in Section::ALL {
        let selected = tui.nav.section == section;
        let style = if selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        spans.push(Span::styled(format!(" {} ", section.label()), style));
        spans.push(Span::raw(" "));
    }

    // Right-aligned-ish status chips.
    let channel = if tui.twitch_chat.channel.is_empty() {
        "—".to_string()
    } else {
        format!("#{}", tui.twitch_chat.channel)
    };
    let overlays_chip = if tui.overlays.running {
        format!("overlays :{} ●", tui.overlays_port)
    } else if app.overlays_enabled {
        format!("overlays :{} ◌", tui.overlays_port)
    } else {
        "overlays off".to_string()
    };
    spans.push(Span::raw("   "));
    spans.push(Span::styled(channel, Style::default().fg(COLOR_CONNECTED)));
    spans.push(Span::raw("  "));
    spans.push(Span::styled(
        overlays_chip,
        Style::default().fg(if tui.overlays.running {
            COLOR_CONNECTED
        } else {
            COLOR_INACTIVE
        }),
    ));

    let bar = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(bar, area);
}

/// Sidebar: the contextual sub-nav for the active section, with the active
/// section name as a breadcrumb header.
pub fn draw_sidebar(frame: &mut Frame, area: Rect, tui: &TuiState) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let section = tui.nav.section;
    let current = tui.nav.current_sub();

    let mut lines = vec![
        Line::from(Span::styled(
            format!(" {} ", section.label()),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for item in nav::sub_items(section) {
        let selected = item == current;
        let marker = if selected { "›" } else { " " };
        let style = if selected {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {marker} "), style),
            Span::styled(sub_label_with_tag(item), style),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

/// Status bar with context-aware key hints.
pub fn draw_status_bar(frame: &mut Frame, area: Rect, app: &AppState) {
    if let Some(msg) = &app.status_message {
        let bar = Paragraph::new(format!(" {msg}"))
            .style(Style::default().fg(Color::Red).bg(Color::Black));
        frame.render_widget(bar, area);
    } else {
        let help =
            " q: quit | Tab: section | j/k: sub-nav | Space: toggle | J/K: scroll | 1-6: filters";
        let bar = Paragraph::new(help).style(Style::default().fg(Color::DarkGray).bg(Color::Black));
        frame.render_widget(bar, area);
    }
}

fn sub_label_with_tag(item: SubItem) -> String {
    nav::sub_label(item)
}
