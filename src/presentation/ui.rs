use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use super::state::{Pane, TuiState};
use super::theme::*;
use crate::application::AppState;
use crate::domain::{AppEvent, AppEventEntry, EventGroup, StreamEvent};

// Number of integration cards (for cursor bounds)
pub const INTEGRATION_COUNT: usize = 6;

// ---------------------------------------------------------------------------
// Main draw entry point
// ---------------------------------------------------------------------------

pub fn draw(frame: &mut Frame, app: &AppState, tui: &TuiState) {
    let [main_area, status_bar] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());

    // main_area: left (25%) | right_area (75%)
    let [left, right_area] =
        Layout::horizontal([Constraint::Percentage(25), Constraint::Min(0)]).areas(main_area);

    // right_area: top_right (45%) | event_log (55%)
    let [top_right, event_log_area] =
        Layout::vertical([Constraint::Percentage(45), Constraint::Min(0)]).areas(right_area);

    // top_right: stats (55%) | chat (45%)
    let [stats_area, chat_area] =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Min(0)]).areas(top_right);

    draw_integrations(frame, left, app, tui);
    draw_stats(frame, stats_area, app, tui);
    draw_chat(frame, chat_area, tui);
    draw_event_log(frame, event_log_area, app, tui);
    draw_status_bar(frame, status_bar, app);
}

// ---------------------------------------------------------------------------
// Left panel: Integration cards
// ---------------------------------------------------------------------------

fn draw_integrations(frame: &mut Frame, area: Rect, app: &AppState, tui: &TuiState) {
    let focused = tui.focused_pane == Pane::Integrations;
    let block = make_block("Integrations", focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    // Card 0: Twitch EventSub
    {
        let cursor = if focused && tui.integration_cursor == 0 {
            ">"
        } else {
            " "
        };
        let (dot, dot_color) = if tui.twitch_eventsub.connected {
            ("●", COLOR_CONNECTED)
        } else {
            ("○", COLOR_INACTIVE)
        };
        lines.push(Line::from(vec![
            Span::raw(format!("{cursor} ")),
            Span::styled(
                "Twitch EventSub",
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(dot, Style::default().fg(dot_color)),
            Span::raw(if tui.twitch_eventsub.connected {
                " Connected"
            } else {
                " Disconnected"
            }),
        ]));
        if tui.twitch_eventsub.subs_registered > 0 || tui.twitch_eventsub.subs_total > 0 {
            let subs_text = if tui.twitch_eventsub.subs_total > 0 {
                format!(
                    "    Subs: {}/{}",
                    tui.twitch_eventsub.subs_registered, tui.twitch_eventsub.subs_total
                )
            } else {
                format!("    Subs: {}", tui.twitch_eventsub.subs_registered)
            };
            lines.push(Line::from(Span::styled(
                subs_text,
                Style::default().fg(Color::DarkGray),
            )));
        }
        lines.push(Line::from(""));
    }

    // Card 1: Twitch Chat (IRC)
    {
        let cursor = if focused && tui.integration_cursor == 1 {
            ">"
        } else {
            " "
        };
        let (dot, dot_color) = if tui.twitch_chat.connected {
            ("●", COLOR_CONNECTED)
        } else {
            ("○", COLOR_INACTIVE)
        };
        lines.push(Line::from(vec![
            Span::raw(format!("{cursor} ")),
            Span::styled(
                "Twitch Chat (IRC)",
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(dot, Style::default().fg(dot_color)),
            Span::raw(if tui.twitch_chat.connected {
                " Connected"
            } else {
                " Disconnected"
            }),
        ]));
        if !tui.twitch_chat.channel.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("    #{}", tui.twitch_chat.channel),
                Style::default().fg(Color::DarkGray),
            )));
        }
        if tui.twitch_chat.message_count > 0 {
            lines.push(Line::from(Span::styled(
                format!("    Messages: {}", tui.twitch_chat.message_count),
                Style::default().fg(Color::DarkGray),
            )));
        }
        lines.push(Line::from(""));
    }

    // Card 2: Livepix Donations (toggleable index 0 → cursor 2)
    {
        let cursor = if focused && tui.integration_cursor == 2 {
            ">"
        } else {
            " "
        };
        let checkbox = if app.livepix_enabled { "[x]" } else { "[ ]" };
        let (dot, dot_color) = if tui.livepix.running {
            ("●", COLOR_CONNECTED)
        } else if app.livepix_enabled {
            ("●", COLOR_STARTING)
        } else {
            ("○", COLOR_INACTIVE)
        };
        lines.push(Line::from(vec![
            Span::raw(format!("{cursor} ")),
            Span::styled(
                format!("{checkbox} Livepix Donations"),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(dot, Style::default().fg(dot_color)),
            Span::raw(if tui.livepix.running {
                format!(" Listening :{}", tui.livepix.port)
            } else {
                " Stopped".into()
            }),
        ]));
        if tui.livepix.running {
            let oauth = if tui.livepix.oauth_ok { "✓" } else { "✗" };
            let tts = if tui.tts_available { "✓" } else { "✗" };
            lines.push(Line::from(Span::styled(
                format!("    OAuth: {oauth}  TTS: {tts}"),
                Style::default().fg(Color::DarkGray),
            )));
        }
        lines.push(Line::from(""));
    }

    // Card 3: Privacy Monitor (toggleable index 1 → cursor 3)
    {
        let cursor = if focused && tui.integration_cursor == 3 {
            ">"
        } else {
            " "
        };
        let checkbox = if app.privacy_enabled { "[x]" } else { "[ ]" };
        let (dot, dot_color) = if tui.privacy.running {
            ("●", COLOR_CONNECTED)
        } else if app.privacy_enabled {
            ("●", COLOR_STARTING)
        } else {
            ("○", COLOR_INACTIVE)
        };
        lines.push(Line::from(vec![
            Span::raw(format!("{cursor} ")),
            Span::styled(
                format!("{checkbox} Privacy Monitor"),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(dot, Style::default().fg(dot_color)),
            Span::raw(if tui.privacy.running {
                " Running"
            } else {
                " Stopped"
            }),
        ]));
        if tui.privacy.running {
            let blur_text = if tui.privacy.blur_active {
                match &tui.privacy.blur_target {
                    Some(target) => format!("    Blur: ON ({target})"),
                    None => "    Blur: ON".into(),
                }
            } else {
                "    Blur: OFF".into()
            };
            let blur_color = if tui.privacy.blur_active {
                COLOR_DISCONNECTED
            } else {
                COLOR_CONNECTED
            };
            lines.push(Line::from(Span::styled(
                blur_text,
                Style::default().fg(blur_color),
            )));
        }
        lines.push(Line::from(""));
    }

    // Card 4: Waybar (toggleable index 2 → cursor 4)
    {
        let cursor = if focused && tui.integration_cursor == 4 {
            ">"
        } else {
            " "
        };
        let checkbox = if app.waybar_enabled { "[x]" } else { "[ ]" };
        let (dot, dot_color) = if app.waybar_enabled {
            ("●", COLOR_CONNECTED)
        } else {
            ("○", COLOR_INACTIVE)
        };
        lines.push(Line::from(vec![
            Span::raw(format!("{cursor} ")),
            Span::styled(
                format!("{checkbox} Waybar"),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(dot, Style::default().fg(dot_color)),
            Span::raw(if app.waybar_enabled {
                " Active"
            } else {
                " Inactive"
            }),
        ]));
        lines.push(Line::from(""));
    }

    // Card 5: Hyprland (toggleable index 3 → cursor 5)
    {
        let cursor = if focused && tui.integration_cursor == 5 {
            ">"
        } else {
            " "
        };
        let checkbox = if app.alerts_enabled { "[x]" } else { "[ ]" };
        let (dot, dot_color) = if tui.hyprland.listening {
            ("●", COLOR_CONNECTED)
        } else {
            ("○", COLOR_INACTIVE)
        };
        lines.push(Line::from(vec![
            Span::raw(format!("{cursor} ")),
            Span::styled(
                format!("{checkbox} Hyprland"),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(dot, Style::default().fg(dot_color)),
            Span::raw(if tui.hyprland.listening {
                " Listening"
            } else {
                " Inactive"
            }),
        ]));
        if tui.hyprland.event_count > 0 {
            lines.push(Line::from(Span::styled(
                format!("    Events: {}", tui.hyprland.event_count),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, inner);
}

// ---------------------------------------------------------------------------
// Stats panel with highlights sub-panel
// ---------------------------------------------------------------------------

fn draw_stats(frame: &mut Frame, area: Rect, app: &AppState, tui: &TuiState) {
    let focused = tui.focused_pane == Pane::Stats;
    let block = make_block("Stream Stats", focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split inner into counters (top) and highlights (bottom)
    let [counters_area, highlights_area] =
        Layout::vertical([Constraint::Length(9), Constraint::Min(0)]).areas(inner);

    // Counters
    let uptime = app.started_at.elapsed();
    let hours = uptime.as_secs() / 3600;
    let mins = (uptime.as_secs() % 3600) / 60;
    let secs = uptime.as_secs() % 60;

    let counter_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Viewers:  ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{}", app.stats.viewer_count)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Followers: ", Style::default().fg(Color::Magenta)),
            Span::raw(format!("{}", app.stats.followers_today)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Subs:      ", Style::default().fg(Color::Green)),
            Span::raw(format!("{}", app.stats.subs_today)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Uptime:    ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{hours:02}:{mins:02}:{secs:02}")),
        ]),
    ];
    frame.render_widget(Paragraph::new(counter_lines), counters_area);

    // Highlights sub-panel
    let highlights_block = Block::default()
        .title(" Recent Highlights ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let highlights_inner = highlights_block.inner(highlights_area);
    frame.render_widget(highlights_block, highlights_area);

    let highlight_lines: Vec<Line> = tui
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

    frame.render_widget(
        Paragraph::new(highlight_lines).wrap(Wrap { trim: true }),
        highlights_inner,
    );
}

// ---------------------------------------------------------------------------
// Chat panel
// ---------------------------------------------------------------------------

fn draw_chat(frame: &mut Frame, area: Rect, tui: &TuiState) {
    let focused = tui.focused_pane == Pane::Chat;

    let title = if tui.twitch_chat.channel.is_empty() {
        " Chat ".to_string()
    } else {
        let status = if tui.twitch_chat.connected {
            "●"
        } else {
            "○"
        };
        format!(" Chat #{} {status} ", tui.twitch_chat.channel)
    };

    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);
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

// ---------------------------------------------------------------------------
// Event log panel
// ---------------------------------------------------------------------------

fn draw_event_log(frame: &mut Frame, area: Rect, app: &AppState, tui: &TuiState) {
    let focused = tui.focused_pane == Pane::EventLog;
    let block = make_event_log_block(focused, tui);

    let lines: Vec<Line> = app
        .event_log
        .iter()
        .rev()
        .filter(|entry| is_group_visible(entry.event.group(), tui))
        .map(|entry| format_app_event(entry))
        .collect();

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true })
        .scroll((tui.event_log_scroll, 0));

    frame.render_widget(paragraph, area);
}

fn is_group_visible(group: EventGroup, tui: &TuiState) -> bool {
    match group {
        EventGroup::Stream => tui.filter_stream,
        EventGroup::Privacy => tui.filter_privacy,
        EventGroup::System => tui.filter_system,
        EventGroup::Hyprland => tui.filter_hyprland,
        EventGroup::Livepix => tui.filter_livepix,
        EventGroup::Chat => tui.filter_chat,
    }
}

/// Build the Event Log block with filter legend in the title.
fn make_event_log_block(focused: bool, tui: &TuiState) -> Block<'static> {
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let on_style = Style::default()
        .fg(Color::Green)
        .add_modifier(Modifier::BOLD);
    let off_style = Style::default().fg(Color::DarkGray);

    let filter_labels = [
        ("S", tui.filter_stream),
        ("P", tui.filter_privacy),
        ("Sys", tui.filter_system),
        ("H", tui.filter_hyprland),
        ("L", tui.filter_livepix),
        ("C", tui.filter_chat),
    ];

    let mut title_spans: Vec<Span> = vec![Span::raw(" Event Log ")];
    for (label, active) in filter_labels {
        let style = if active { on_style } else { off_style };
        title_spans.push(Span::styled(format!("[{label}]"), style));
        title_spans.push(Span::raw(" "));
    }

    Block::default()
        .title(Line::from(title_spans))
        .borders(Borders::ALL)
        .border_style(border_style)
}

// ---------------------------------------------------------------------------
// Event formatting
// ---------------------------------------------------------------------------

fn format_app_event(entry: &AppEventEntry) -> Line<'static> {
    let secs = entry.elapsed.as_secs();
    let ts = format!(
        "{:02}:{:02}:{:02}",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    );

    let (icon, color, body) = match &entry.event {
        // --- Stream group ---
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

        // --- Privacy group ---
        AppEvent::PrivacyBlurEnabled { title } => {
            ("🔒", COLOR_BLUR_ON, format!("Blur ON: {title}"))
        }
        AppEvent::PrivacyBlurDisabled => ("🔓", COLOR_BLUR_OFF, "Blur OFF".into()),
        AppEvent::PrivacyStarted => ("▶", COLOR_TOGGLE_ON, "Privacy monitor started".into()),
        AppEvent::PrivacyStopped => ("■", COLOR_TOGGLE_OFF, "Privacy monitor stopped".into()),
        AppEvent::PrivacyError(msg) => ("⚠", COLOR_WARN, msg.to_string()),

        // --- Livepix group ---
        AppEvent::LivepixInfo(msg) => ("ℹ", COLOR_INFO, format!("Livepix: {msg}")),
        AppEvent::LivepixError(msg) => ("✖", COLOR_ERROR, format!("Livepix: {msg}")),

        // --- System group ---
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

        // --- Hyprland group ---
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

        // --- Chat group ---
        AppEvent::ChatMessage { username, text } => {
            ("💬", COLOR_CHAT, format!("{username}: {text}"))
        }
    };

    Line::from(vec![
        Span::styled(format!("[{ts}] "), Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{icon} "), Style::default().fg(color)),
        Span::raw(body),
    ])
}

// ---------------------------------------------------------------------------
// Status bar
// ---------------------------------------------------------------------------

fn draw_status_bar(frame: &mut Frame, area: Rect, app: &AppState) {
    if let Some(msg) = &app.status_message {
        let bar = Paragraph::new(format!(" {msg}"))
            .style(Style::default().fg(Color::Red).bg(Color::Black));
        frame.render_widget(bar, area);
    } else {
        let help = " q: quit | Tab: pane | j/k: scroll | Space: toggle | 1-6: filter";
        let bar = Paragraph::new(help).style(Style::default().fg(Color::DarkGray).bg(Color::Black));
        frame.render_widget(bar, area);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_block(title: &str, focused: bool) -> Block<'_> {
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_style(border_style)
}
