//! Overlays section panes: All overlays, per-Overlay detail (with OBS URL),
//! and the Feed (SSE) inspector.

use ratatui::prelude::*;
use ratatui::widgets::{Paragraph, Wrap};

use crate::application::AppState;

use super::super::nav::{self, OverlayDef, OverlayId};
use super::super::state::TuiState;
use super::super::theme::*;
use super::format;

/// "All overlays": the server's Start/Stop status, port, and each Overlay's
/// OBS browser-source URL.
pub fn draw_all(frame: &mut Frame, area: Rect, app: &AppState, tui: &TuiState) {
    let block = format::make_block("Overlays", true);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = vec![server_line(app, tui), Line::from("")];

    lines.push(format::section_label("Overlays served"));
    for o in OverlayId::ALL.iter() {
        lines.push(overlay_row(o, tui));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Each Overlay is one OBS browser source consuming the shared SSE feed.",
        Style::default().fg(COLOR_MUTED),
    )));
    lines.push(Line::from(Span::styled(
        "  Toggle the server with Space/Enter while on this section.",
        Style::default().fg(COLOR_MUTED),
    )));

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

/// Per-Overlay detail: name, enabled state, and the OBS browser source URL.
pub fn draw_detail(frame: &mut Frame, area: Rect, id: OverlayId, _app: &AppState, tui: &TuiState) {
    let def = id.def();
    let block = format::make_block(&format!("{} Overlay", def.name), true);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let url = nav::overlay_url(def, tui.overlays_port);
    let (dot, color, state) = if tui.overlays.running {
        ("●", COLOR_CONNECTED, "served")
    } else {
        ("○", COLOR_INACTIVE, "server stopped")
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(format!("  {dot} "), Style::default().fg(color)),
            Span::styled(
                def.name.to_string(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("  ({state})"), Style::default().fg(COLOR_MUTED)),
        ]),
        Line::from(""),
        format::kv("OBS source", &url),
        format::kv("Consumes", "shared SSE feed (chat + stream events)"),
        format::kv(
            "Component",
            &format!("overlays/src/components/{}Overlay", def.name),
        ),
        Line::from(""),
        Line::from(Span::styled(
            "  Paste the OBS source URL into a Browser Source in OBS.",
            Style::default().fg(COLOR_MUTED),
        )),
    ];

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

/// The Feed (SSE) inspector: endpoint + payload shape, plus recent chat.
pub fn draw_feed(frame: &mut Frame, area: Rect, tui: &TuiState) {
    let block = format::make_block("Feed (SSE)", true);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let endpoint = format!("http://127.0.0.1:{}/overlay/feed", tui.overlays_port);
    let mut lines = vec![
        format::kv("Endpoint", &endpoint),
        format::kv("Transport", "Server-Sent Events (one feed, N overlays)"),
        format::kv(
            "Payload",
            "ChatMessage (badges/emotes/colour) + StreamEvent + delete",
        ),
        Line::from(""),
        format::section_label("Recent messages on the feed"),
    ];

    for msg in tui.chat_messages.iter().rev().take(12) {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {}: ", msg.username),
                Style::default().fg(COLOR_CHAT).add_modifier(Modifier::BOLD),
            ),
            Span::raw(msg.text.clone()),
        ]));
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}

fn server_line(app: &AppState, tui: &TuiState) -> Line<'static> {
    let (dot, color, label) = if tui.overlays.running {
        ("●", COLOR_CONNECTED, "RUNNING")
    } else if app.overlays_enabled {
        ("●", COLOR_STARTING, "STARTING…")
    } else {
        ("○", COLOR_INACTIVE, "STOPPED")
    };

    Line::from(vec![
        Span::styled(format!("  {dot} "), Style::default().fg(color)),
        Span::styled(
            "Overlay server ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            label.to_string(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("   http://127.0.0.1:{}", tui.overlays_port),
            Style::default().fg(COLOR_ACCENT),
        ),
    ])
}

fn overlay_row(def: &OverlayDef, tui: &TuiState) -> Line<'static> {
    let (dot, color) = if tui.overlays.running {
        ("●", COLOR_CONNECTED)
    } else {
        ("○", COLOR_INACTIVE)
    };
    Line::from(vec![
        Span::styled(format!("  {dot} "), Style::default().fg(color)),
        Span::styled(
            format!("{:<6}", def.name),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            nav::overlay_url(def, tui.overlays_port),
            Style::default().fg(COLOR_ACCENT),
        ),
    ])
}
