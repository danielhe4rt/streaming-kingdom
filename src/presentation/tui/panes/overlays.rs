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
    lines.push(Line::from(Span::styled(
        "  Each Overlay is one OBS browser source consuming the shared SSE feed.",
        Style::default().fg(COLOR_MUTED),
    )));

    lines.push(Line::from(""));
    lines.push(format::section_label("Live feed"));
    lines.push(format::kv(
        "Endpoint",
        &format!("http://127.0.0.1:{}/overlay/feed", tui.overlays_port),
    ));
    lines.push(format::kv(
        "Payload",
        "ChatMessage (badges/emotes/colour) + StreamEvent + delete",
    ));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  RECENT ON FEED",
        Style::default()
            .fg(COLOR_MUTED)
            .add_modifier(Modifier::BOLD),
    )));
    if tui.chat_messages.is_empty() {
        lines.push(Line::from(Span::styled(
            "  no messages yet…",
            Style::default()
                .fg(COLOR_MUTED)
                .add_modifier(Modifier::ITALIC),
        )));
    } else {
        for msg in tui.chat_messages.iter().rev().take(10) {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {}: ", msg.username),
                    Style::default().fg(COLOR_CHAT).add_modifier(Modifier::BOLD),
                ),
                Span::raw(msg.text.clone()),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
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
            &format!("overlays/src/overlays/{}Overlay.tsx", def.name),
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

/// The Test events dispatch pane: a selectable list of Synthetic Events. `J/K`
/// moves the cursor; `Enter` fires the selected one onto the real channels, so
/// the Overlay and the TUI log react exactly as to a real event (ADR-0002).
pub fn draw_test_events(frame: &mut Frame, area: Rect, tui: &TuiState) {
    use crate::presentation::synthetic::SyntheticKind;

    let block = format::make_block("Test events", true);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = vec![
        Line::from(Span::styled(
            "  Dispara eventos sintéticos nos canais reais — a Overlay e o log reagem como a um evento real.",
            Style::default().fg(COLOR_MUTED),
        )),
        Line::from(""),
    ];

    for (i, kind) in SyntheticKind::ALL.iter().enumerate() {
        let selected = i == tui.test_event_cursor;
        let label = format!("{} {}", kind.icon(), kind.label());
        if selected {
            lines.push(Line::from(vec![
                Span::styled("  ▌ ", Style::default().fg(COLOR_ACCENT)),
                Span::styled(
                    label,
                    Style::default()
                        .fg(COLOR_ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        } else {
            lines.push(Line::from(Span::styled(
                format!("    {label}"),
                Style::default().fg(COLOR_PRIMARY),
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  J/K move · Enter dispara",
        Style::default().fg(COLOR_MUTED),
    )));
    let last = match &tui.last_test_msg_id {
        Some(id) => format!("  última msg de teste: {id}"),
        None => "  nenhuma msg de teste enviada ainda".to_string(),
    };
    lines.push(Line::from(Span::styled(
        last,
        Style::default()
            .fg(COLOR_MUTED)
            .add_modifier(Modifier::ITALIC),
    )));

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
