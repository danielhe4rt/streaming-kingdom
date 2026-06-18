//! Activity section panes: Live Chat, Event Log, Highlights.
//!
//! The terminal chat view is preserved here exactly as before the reorg — it
//! just lives behind the Activity → Live Chat sub-nav now instead of a fixed
//! pane.

use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};

use crate::application::AppState;

use super::super::state::TuiState;
use super::super::theme::*;
use super::format;

/// Activity → Live Chat. The append-only terminal chat view (preserved).
pub fn draw_chat(frame: &mut Frame, area: Rect, tui: &TuiState) {
    let title = if tui.twitch_chat.channel.is_empty() {
        "Live Chat".to_string()
    } else {
        let status = if tui.twitch_chat.connected { "●" } else { "○" };
        format!("Live Chat #{} {status}", tui.twitch_chat.channel)
    };

    let block = format::make_block(&title, true);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines: Vec<Line> = tui
        .chat_messages
        .iter()
        .rev()
        .map(|msg| {
            Line::from(vec![
                Span::styled(
                    format!("{}: ", msg.username),
                    Style::default().fg(COLOR_CHAT).add_modifier(Modifier::BOLD),
                ),
                Span::raw(msg.text.clone()),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .scroll((tui.chat_scroll, 0));
    frame.render_widget(paragraph, inner);
}

/// Activity → Event Log. Full-width filtered log with the filter legend.
pub fn draw_event_log(frame: &mut Frame, area: Rect, app: &AppState, tui: &TuiState) {
    let block = event_log_block(tui);

    let lines: Vec<Line> = app
        .event_log
        .iter()
        .rev()
        .filter(|entry| format::is_group_visible(entry.event.group(), tui))
        .map(format::format_app_event)
        .collect();

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true })
        .scroll((tui.event_log_scroll, 0));
    frame.render_widget(paragraph, area);
}

/// Activity → Highlights. Recent notable stream events.
pub fn draw_highlights(frame: &mut Frame, area: Rect, tui: &TuiState) {
    let block = format::make_block("Recent Highlights", true);
    let inner = block.inner(area);
    frame.render_widget(block, area);

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

fn event_log_block(tui: &TuiState) -> Block<'static> {
    let on_style = Style::default()
        .fg(COLOR_CONNECTED)
        .add_modifier(Modifier::BOLD);
    let off_style = Style::default().fg(COLOR_MUTED);

    let filter_labels = [
        ("S", tui.filter_stream),
        ("P", tui.filter_privacy),
        ("Sys", tui.filter_system),
        ("H", tui.filter_hyprland),
        ("L", tui.filter_livepix),
        ("C", tui.filter_chat),
    ];

    let mut title_spans: Vec<Span> = vec![Span::styled(
        " Event Log ",
        Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD),
    )];
    for (label, active) in filter_labels {
        let style = if active { on_style } else { off_style };
        title_spans.push(Span::styled(format!("[{label}]"), style));
        title_spans.push(Span::raw(" "));
    }

    Block::default()
        .title(Line::from(title_spans))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(COLOR_PRIMARY))
}
