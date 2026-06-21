//! Services section panes: the grouped Inputs/Outputs list ("All services")
//! and a per-Service detail view. Both render from the data-driven registry.

use ratatui::prelude::*;
use ratatui::widgets::{Paragraph, Wrap};

use crate::application::AppState;

use super::super::nav::SubItem;
use super::super::service::{self, ServiceDef, ServiceId, ServiceKind};
use super::super::state::TuiState;
use super::super::theme::*;
use super::format;

/// "All services": grouped Inputs then Outputs, the selected row marked.
pub fn draw_all(frame: &mut Frame, area: Rect, app: &AppState, tui: &TuiState) {
    let block = format::make_block("Services — Inputs & Outputs", true);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let selected = match tui.nav.current_sub() {
        SubItem::Service(id) => Some(id),
        _ => None,
    };

    let mut lines: Vec<Line> = Vec::new();
    lines.push(format::section_label("Inputs"));
    for def in service::inputs() {
        lines.push(format::service_line(
            def,
            app,
            tui,
            selected == Some(def.id),
        ));
    }
    lines.push(Line::from(""));
    lines.push(format::section_label("Outputs"));
    for def in service::outputs() {
        lines.push(format::service_line(
            def,
            app,
            tui,
            selected == Some(def.id),
        ));
    }

    // Live summary footer. Spotify counts as "live" whenever a track is loaded
    // (Playing or Paused) — the observer clears to None when nothing is playing.
    let spotify_live = tui.now_playing.is_some();
    let inputs_live = tui.twitch_eventsub.connected as u32
        + tui.twitch_chat.connected as u32
        + spotify_live as u32
        + tui.livepix.running as u32
        + tui.hyprland.listening as u32
        + tui.privacy.running as u32;
    let outputs_on =
        app.waybar_enabled as u32 + tui.overlays.running as u32 + tui.discord.running as u32;
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Inputs ", Style::default().fg(COLOR_MUTED)),
        Span::styled(
            format!("{inputs_live}/6 live"),
            Style::default().fg(COLOR_CONNECTED),
        ),
        Span::styled("   Outputs ", Style::default().fg(COLOR_MUTED)),
        Span::styled(
            format!("{outputs_on}/3 active"),
            Style::default().fg(COLOR_CONNECTED),
        ),
        Span::styled(
            "   ·  Space/Enter to toggle the selected Output",
            Style::default().fg(COLOR_MUTED),
        ),
    ]));

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

/// Per-Service detail: status, classification, and toggle hint.
pub fn draw_detail(frame: &mut Frame, area: Rect, id: ServiceId, app: &AppState, tui: &TuiState) {
    let def = service::registry()
        .iter()
        .find(|s| s.id == id)
        .copied()
        .unwrap_or(ServiceDef {
            id,
            name: "?",
            kind: ServiceKind::Input,
            toggleable: false,
        });

    let block = format::make_block(def.name, true);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (dot, dot_color, status) = format::service_status(&def, app, tui);
    let kind = match def.kind {
        ServiceKind::Input => "input (monitor)",
        ServiceKind::Output => "output (toggleable)",
    };

    let mut lines = vec![
        format::kv("Kind", kind),
        Line::from(vec![
            Span::styled("  Status      ", Style::default().fg(COLOR_MUTED)),
            Span::styled(format!("{dot} "), Style::default().fg(dot_color)),
            Span::raw(status),
        ]),
    ];

    if def.toggleable {
        let enabled = service::is_enabled(def.id, app);
        lines.push(Line::from(vec![
            Span::styled("  Control     ", Style::default().fg(COLOR_MUTED)),
            Span::styled(
                if enabled { "ON" } else { "OFF" }.to_string(),
                Style::default().fg(if enabled {
                    COLOR_TOGGLE_ON
                } else {
                    COLOR_TOGGLE_OFF
                }),
            ),
            Span::styled(
                "  (Space/Enter to toggle)",
                Style::default().fg(COLOR_MUTED),
            ),
        ]));
    } else {
        lines.push(format::kv("Control", "read-only (monitor)"));
    }

    if id == ServiceId::Overlays {
        lines.push(Line::from(""));
        lines.push(format::section_label("OBS browser sources"));
        for o in super::super::nav::OverlayId::ALL.iter() {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("{}: ", o.name),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    super::super::nav::overlay_url(o, tui.overlays_port),
                    Style::default().fg(COLOR_ACCENT),
                ),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
