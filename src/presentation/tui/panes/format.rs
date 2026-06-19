//! Shared, layout-agnostic render helpers for the nav-shell panes.
//!
//! These are the "component builders" the prototype's NOTES call out: every
//! pane renders Service rows, event lines, etc. from the same functions, so a
//! new Service/Overlay flows through one path.

use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders};

use crate::application::AppState;
use crate::domain::{AppEvent, AppEventEntry, EventGroup, StreamEvent};

use super::super::service::{self, ServiceDef, ServiceId};
use super::super::state::TuiState;
use super::super::theme::*;

/// A rounded, optionally-focused block with a lavender title. Focused blocks
/// take the vivid primary border; secondary blocks the muted one.
pub fn make_block(title: &str, focused: bool) -> Block<'static> {
    let border = if focused { COLOR_PRIMARY } else { COLOR_BORDER };
    Block::default()
        .title(Line::from(Span::styled(
            format!(" {title} "),
            Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD),
        )))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border))
}

/// An uppercase section label (e.g. `▸ INPUTS`) in lavender — the consistent
/// group header across panes.
pub fn section_label(label: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!(" ▸ {} ", label.to_uppercase()),
        Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD),
    ))
}

/// A key/value detail line: muted key, plain value. Used by detail panes.
pub fn kv(key: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {key:<11} "), Style::default().fg(COLOR_MUTED)),
        Span::raw(value.to_string()),
    ])
}

/// Resolve the status dot + colour + short status text for a Service, reading
/// live status from `TuiState`/`AppState`. Pure formatting, registry-driven.
pub fn service_status(def: &ServiceDef, app: &AppState, tui: &TuiState) -> (&'static str, Color, String) {
    match def.id {
        ServiceId::TwitchEventSub => {
            if tui.twitch_eventsub.connected {
                let subs = if tui.twitch_eventsub.subs_total > 0 {
                    format!(
                        "Connected · subs {}/{}",
                        tui.twitch_eventsub.subs_registered, tui.twitch_eventsub.subs_total
                    )
                } else if tui.twitch_eventsub.subs_registered > 0 {
                    format!("Connected · {} subs", tui.twitch_eventsub.subs_registered)
                } else {
                    "Connected".into()
                };
                ("●", COLOR_CONNECTED, subs)
            } else {
                ("○", COLOR_INACTIVE, "Disconnected".into())
            }
        }
        ServiceId::TwitchChat => {
            if tui.twitch_chat.connected {
                ("●", COLOR_CONNECTED, format!("Connected · {} msgs", tui.twitch_chat.message_count))
            } else {
                ("○", COLOR_INACTIVE, "Disconnected".into())
            }
        }
        ServiceId::Spotify => {
            // Ambient now-playing state: a filled dot while a track is loaded
            // (green = Playing, amber = Paused), hollow when nothing is playing.
            use crate::domain::PlaybackStatus;
            match &tui.now_playing {
                Some(np) => {
                    let track = format!("{} — {}", np.title, np.artist);
                    match np.status {
                        PlaybackStatus::Playing => ("●", COLOR_CONNECTED, format!("Playing · {track}")),
                        PlaybackStatus::Paused => ("●", COLOR_STARTING, format!("Paused · {track}")),
                        // Defensive: the observer clears to None on Stopped, but
                        // render a sensible state if a Stopped value ever arrives.
                        PlaybackStatus::Stopped => ("○", COLOR_INACTIVE, "Nothing playing".into()),
                    }
                }
                None => ("○", COLOR_INACTIVE, "Nothing playing".into()),
            }
        }
        ServiceId::Livepix => {
            if tui.livepix.running {
                let oauth = if tui.livepix.oauth_ok { "✓" } else { "✗" };
                let tts = if tui.tts_available { "✓" } else { "✗" };
                (
                    "●",
                    COLOR_CONNECTED,
                    format!(
                        "Listening :{} · OAuth {oauth} · TTS {tts}",
                        tui.livepix.port
                    ),
                )
            } else if app.livepix_enabled {
                ("●", COLOR_STARTING, "Starting…".into())
            } else {
                ("○", COLOR_INACTIVE, "Stopped".into())
            }
        }
        ServiceId::Hyprland => {
            if tui.hyprland.listening {
                ("●", COLOR_CONNECTED, format!("Listening · {} events", tui.hyprland.event_count))
            } else {
                ("○", COLOR_INACTIVE, "Inactive".into())
            }
        }
        ServiceId::Privacy => {
            if tui.privacy.running {
                let blur = if tui.privacy.blur_active {
                    match &tui.privacy.blur_target {
                        Some(t) => format!("Running · Blur ON ({t})"),
                        None => "Running · Blur ON".into(),
                    }
                } else {
                    "Running · Blur OFF".into()
                };
                ("●", COLOR_CONNECTED, blur)
            } else if app.privacy_enabled {
                ("●", COLOR_STARTING, "Starting…".into())
            } else {
                ("○", COLOR_INACTIVE, "Stopped".into())
            }
        }
        ServiceId::Waybar => {
            if app.waybar_enabled {
                ("●", COLOR_CONNECTED, "Active".into())
            } else {
                ("○", COLOR_INACTIVE, "Inactive".into())
            }
        }
        ServiceId::Overlays => {
            if tui.overlays.running {
                ("●", COLOR_CONNECTED, format!("Serving :{}", tui.overlays.port))
            } else if app.overlays_enabled {
                ("●", COLOR_STARTING, "Starting…".into())
            } else {
                ("○", COLOR_INACTIVE, "Stopped".into())
            }
        }
    }
}

/// One Service row as a TUI line: cursor, status dot, name, kind tag, toggle.
pub fn service_line(def: &ServiceDef, app: &AppState, tui: &TuiState, selected: bool) -> Line<'static> {
    let (dot, dot_color, status) = service_status(def, app, tui);

    // Selection affordance: a bright-purple left bar + bold purple name.
    let (marker, name_style) = if selected {
        (
            "▌ ",
            Style::default().fg(COLOR_PRIMARY).add_modifier(Modifier::BOLD),
        )
    } else {
        ("  ", Style::default().add_modifier(Modifier::BOLD))
    };

    let mut spans = vec![
        Span::styled(marker, Style::default().fg(COLOR_PRIMARY)),
        Span::styled(format!("{dot} "), Style::default().fg(dot_color)),
        Span::styled(def.name.to_string(), name_style),
    ];

    if def.toggleable {
        let enabled = service::is_enabled(def.id, app);
        let (tag, color) = if enabled {
            (" [ON]", COLOR_TOGGLE_ON)
        } else {
            (" [OFF]", COLOR_TOGGLE_OFF)
        };
        spans.push(Span::styled(tag.to_string(), Style::default().fg(color)));
    } else {
        spans.push(Span::styled(
            " (monitor)".to_string(),
            Style::default().fg(COLOR_MUTED),
        ));
    }

    spans.push(Span::styled(
        format!("  {status}"),
        Style::default().fg(COLOR_MUTED),
    ));

    Line::from(spans)
}

/// Whether an event group is currently visible per the filter toggles.
pub fn is_group_visible(group: EventGroup, tui: &TuiState) -> bool {
    match group {
        EventGroup::Stream => tui.filter_stream,
        EventGroup::Privacy => tui.filter_privacy,
        EventGroup::System => tui.filter_system,
        EventGroup::Hyprland => tui.filter_hyprland,
        EventGroup::Livepix => tui.filter_livepix,
        EventGroup::Chat => tui.filter_chat,
    }
}

/// Format a single AppEvent into a styled line for the event log.
pub fn format_app_event(entry: &AppEventEntry) -> Line<'static> {
    let secs = entry.elapsed.as_secs();
    let ts = format!(
        "{:02}:{:02}:{:02}",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    );

    let (icon, color, body) = match &entry.event {
        AppEvent::Stream(stream_event) => match stream_event {
            StreamEvent::Follow { username } => ("♥", COLOR_FOLLOW, format!("{username} followed")),
            StreamEvent::Sub {
                username,
                tier,
                months,
            } => (
                "★",
                COLOR_SUB,
                format!("{username} subbed ({tier:?}, {months}mo)"),
            ),
            StreamEvent::GiftSub {
                username,
                tier,
                total,
            } => (
                "🎁",
                COLOR_GIFTSUB,
                format!("{username} gifted {total} subs ({tier:?})"),
            ),
            StreamEvent::Donation {
                username,
                amount_cents,
                message,
            } => (
                "$",
                COLOR_DONATION,
                format!(
                    "{username} donated ${:.2}: {message}",
                    *amount_cents as f64 / 100.0
                ),
            ),
            StreamEvent::Cheer {
                username,
                bits,
                message,
            } => (
                "◆",
                COLOR_CHEER,
                format!("{username} cheered {bits} bits: {message}"),
            ),
            StreamEvent::Raid {
                from_channel,
                viewers,
            } => (
                "⚡",
                COLOR_RAID,
                format!("{from_channel} raided with {viewers} viewers"),
            ),
            StreamEvent::ViewerCountUpdate { count } => {
                ("👁", COLOR_INFO, format!("Viewers updated to {count}"))
            }
        },

        AppEvent::PrivacyBlurEnabled { title } => {
            ("🔒", COLOR_BLUR_ON, format!("Blur ON: {title}"))
        }
        AppEvent::PrivacyBlurDisabled => ("🔓", COLOR_BLUR_OFF, "Blur OFF".into()),
        AppEvent::PrivacyStarted => ("▶", COLOR_TOGGLE_ON, "Privacy monitor started".into()),
        AppEvent::PrivacyStopped => ("■", COLOR_TOGGLE_OFF, "Privacy monitor stopped".into()),
        AppEvent::PrivacyError(msg) => ("⚠", COLOR_WARN, msg.to_string()),

        AppEvent::LivepixInfo(msg) => ("ℹ", COLOR_INFO, format!("Livepix: {msg}")),
        AppEvent::LivepixError(msg) => ("✖", COLOR_ERROR, format!("Livepix: {msg}")),

        AppEvent::FeatureToggled { feature, enabled } => {
            if *enabled {
                ("●", COLOR_TOGGLE_ON, format!("{feature} enabled"))
            } else {
                ("○", COLOR_TOGGLE_OFF, format!("{feature} disabled"))
            }
        }
        AppEvent::WaybarSpawned => ("▶", COLOR_INFO, "Waybar started".into()),
        AppEvent::WaybarKilled => ("■", COLOR_TOGGLE_OFF, "Waybar stopped".into()),
        AppEvent::WaybarError(msg) => ("✖", COLOR_ERROR, msg.to_string()),
        AppEvent::AlertsBrowserOpened => ("▶", COLOR_INFO, "Alerts browser opened".into()),
        AppEvent::AlertsBrowserClosed => ("■", COLOR_TOGGLE_OFF, "Alerts browser closed".into()),
        AppEvent::Info(msg) => ("ℹ", COLOR_INFO, msg.clone()),
        AppEvent::Error(msg) => ("✖", COLOR_ERROR, msg.clone()),

        AppEvent::WindowOpened { title, .. } => {
            ("＋", COLOR_WINDOW_OPEN, format!("Window: {title}"))
        }
        AppEvent::WindowClosed { address } => (
            "✕",
            COLOR_WINDOW_CLOSE,
            format!("Window closed: {}", &address[..address.len().min(8)]),
        ),
        AppEvent::WindowTitleChanged { title, .. } => {
            ("↔", COLOR_TITLE_CHANGE, format!("Title: {title}"))
        }
        AppEvent::WorkspaceChanged { name } => ("⊞", COLOR_WORKSPACE, format!("Workspace: {name}")),
        AppEvent::MonitorFocused { monitor } => ("◉", COLOR_MONITOR, format!("Monitor: {monitor}")),
        AppEvent::WindowMoved { workspace, .. } => {
            ("↔", COLOR_INFO, format!("Window moved to {workspace}"))
        }

        AppEvent::ChatMessage { username, text } => {
            ("💬", COLOR_CHAT, format!("{username}: {text}"))
        }
    };

    Line::from(vec![
        Span::styled(format!("[{ts}] "), Style::default().fg(COLOR_MUTED)),
        Span::styled(format!("{icon} "), Style::default().fg(color)),
        Span::raw(body),
    ])
}
