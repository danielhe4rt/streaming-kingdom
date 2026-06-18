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
        Span::styled(
            " ♥ streams-toolkit ",
            Style::default()
                .fg(COLOR_ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
    ];

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
            .border_style(Style::default().fg(COLOR_BORDER)),
    );
    frame.render_widget(bar, area);
}

/// Sidebar: the contextual sub-nav for the active section, with the active
/// section name as a breadcrumb header.
pub fn draw_sidebar(frame: &mut Frame, area: Rect, tui: &TuiState) {
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
        lines.push(Line::from(vec![
            Span::styled(marker, Style::default().fg(COLOR_PRIMARY)),
            Span::styled(sub_label_with_tag(item), style),
        ]));
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

/// Status bar with context-aware key hints.
pub fn draw_status_bar(frame: &mut Frame, area: Rect, app: &AppState) {
    if let Some(msg) = &app.status_message {
        let bar = Paragraph::new(format!(" {msg}"))
            .style(Style::default().fg(COLOR_ERROR).add_modifier(Modifier::BOLD));
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
