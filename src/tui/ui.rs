use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use crate::app::{AppEvent, AppEventEntry, AppState, EventGroup, StreamEvent};
use super::{Pane, TuiState};

const TOGGLE_LABELS: [&str; 4] = ["Waybar", "Privacy Monitor", "Alerts Browser", "Livepix Donations"];

pub fn draw(frame: &mut Frame, app: &AppState, tui: &TuiState) {
    let [main_area, status_bar] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    let [left, center, right] = Layout::horizontal([
        Constraint::Percentage(25),
        Constraint::Percentage(35),
        Constraint::Percentage(40),
    ])
    .areas(main_area);

    draw_toggles(frame, left, app, tui);
    draw_stats(frame, center, app, tui);
    draw_event_log(frame, right, app, tui);
    draw_status_bar(frame, status_bar, app);
}

// ---------------------------------------------------------------------------
// Left panel: feature toggles
// ---------------------------------------------------------------------------

fn draw_toggles(frame: &mut Frame, area: Rect, app: &AppState, tui: &TuiState) {
    let focused = tui.focused_pane == Pane::Toggles;
    let block = make_block("Features", focused);

    let enabled = [app.waybar_enabled, app.privacy_enabled, app.alerts_enabled, app.livepix_enabled];

    let items: Vec<ListItem> = TOGGLE_LABELS
        .iter()
        .zip(enabled.iter())
        .enumerate()
        .map(|(i, (label, &on))| {
            let checkbox = if on { "[x]" } else { "[ ]" };
            let marker = if focused && i == tui.toggle_cursor {
                ">"
            } else {
                " "
            };
            let style = if focused && i == tui.toggle_cursor {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else if on {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let suffix = match i {
                1 => {
                    // Privacy Monitor — show live status when available
                    match &tui.privacy_status {
                        Some(status) => format!("  ({status})"),
                        None => String::new(),
                    }
                }
                3 => {
                    // Livepix Donations — show listening port or error status
                    match &tui.livepix_status {
                        Some(status) => format!("  ({status})"),
                        None => String::new(),
                    }
                }
                _ => String::new(),
            };
            ListItem::new(format!("{marker} {checkbox} {label}{suffix}")).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

// ---------------------------------------------------------------------------
// Center panel: live stream stats
// ---------------------------------------------------------------------------

fn draw_stats(frame: &mut Frame, area: Rect, app: &AppState, tui: &TuiState) {
    let focused = tui.focused_pane == Pane::Stats;
    let block = make_block("Stream Stats", focused);

    let uptime = app.started_at.elapsed();
    let hours = uptime.as_secs() / 3600;
    let mins = (uptime.as_secs() % 3600) / 60;
    let secs = uptime.as_secs() % 60;

    let text = vec![
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

    let paragraph = Paragraph::new(text).block(block);
    frame.render_widget(paragraph, area);
}

// ---------------------------------------------------------------------------
// Right panel: scrolling event log
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

    let on_style = Style::default().fg(Color::Green).add_modifier(Modifier::BOLD);
    let off_style = Style::default().fg(Color::DarkGray);

    let filter_labels = [
        ("S", tui.filter_stream),
        ("P", tui.filter_privacy),
        ("Sys", tui.filter_system),
        ("H", tui.filter_hyprland),
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
// Color constants (Material Palenight palette)
// ---------------------------------------------------------------------------

const COLOR_FOLLOW: Color = Color::Rgb(0x67, 0x6e, 0x95);
const COLOR_SUB: Color = Color::Rgb(0xc7, 0x92, 0xea);
const COLOR_GIFTSUB: Color = Color::Rgb(0x89, 0xdd, 0xff);
const COLOR_CHEER: Color = Color::Rgb(0xff, 0xcb, 0x6b);
const COLOR_RAID: Color = Color::Rgb(0xf0, 0x71, 0x78);
const COLOR_DONATION: Color = Color::Rgb(0xf7, 0x8c, 0x6c);
const COLOR_BLUR_ON: Color = Color::Rgb(0xff, 0x53, 0x70);
const COLOR_BLUR_OFF: Color = Color::Rgb(0xc3, 0xe8, 0x8d);
const COLOR_WARN: Color = Color::Rgb(0xf7, 0x8c, 0x6c);
const COLOR_TOGGLE_ON: Color = Color::Rgb(0xc3, 0xe8, 0x8d);
const COLOR_TOGGLE_OFF: Color = Color::Rgb(0x67, 0x6e, 0x95);
const COLOR_INFO: Color = Color::Rgb(0x82, 0xaa, 0xff);
const COLOR_ERROR: Color = Color::Rgb(0xff, 0x53, 0x70);
const COLOR_WINDOW_OPEN: Color = Color::Rgb(0xc3, 0xe8, 0x8d);
const COLOR_WINDOW_CLOSE: Color = Color::Rgb(0x67, 0x6e, 0x95);
const COLOR_TITLE_CHANGE: Color = Color::Rgb(0x82, 0xaa, 0xff);
const COLOR_WORKSPACE: Color = Color::Rgb(0xc7, 0x92, 0xea);
const COLOR_MONITOR: Color = Color::Rgb(0xff, 0xcb, 0x6b);
const COLOR_CHAT: Color = Color::Rgb(0xc3, 0xe8, 0x8d);

fn format_app_event(entry: &AppEventEntry) -> Line<'static> {
    let secs = entry.elapsed.as_secs();
    let ts = format!("{:02}:{:02}:{:02}", secs / 3600, (secs % 3600) / 60, secs % 60);

    let (icon, color, body) = match &entry.event {
        // --- Stream group ---
        AppEvent::Stream(stream_event) => match stream_event {
            StreamEvent::Follow { username } => (
                "♥",
                COLOR_FOLLOW,
                format!("{username} followed"),
            ),
            StreamEvent::Sub { username, tier, months } => (
                "★",
                COLOR_SUB,
                format!("{username} subbed ({tier:?}, {months}mo)"),
            ),
            StreamEvent::GiftSub { username, tier, total } => (
                "🎁",
                COLOR_GIFTSUB,
                format!("{username} gifted {total} subs ({tier:?})"),
            ),
            StreamEvent::Donation { username, amount_cents, message } => (
                "$",
                COLOR_DONATION,
                format!("{username} donated ${:.2}: {message}", *amount_cents as f64 / 100.0),
            ),
            StreamEvent::Cheer { username, bits, message } => (
                "◆",
                COLOR_CHEER,
                format!("{username} cheered {bits} bits: {message}"),
            ),
            StreamEvent::Raid { from_channel, viewers } => (
                "⚡",
                COLOR_RAID,
                format!("{from_channel} raided with {viewers} viewers"),
            ),
            StreamEvent::ViewerCountUpdate { count } => (
                "👁",
                COLOR_INFO,
                format!("Viewers updated to {count}"),
            ),
        },

        // --- Privacy group ---
        AppEvent::PrivacyBlurEnabled { title } => (
            "🔒",
            COLOR_BLUR_ON,
            format!("Blur ON: {title}"),
        ),
        AppEvent::PrivacyBlurDisabled => (
            "🔓",
            COLOR_BLUR_OFF,
            "Blur OFF".into(),
        ),
        AppEvent::PrivacyStarted => (
            "▶",
            COLOR_TOGGLE_ON,
            "Privacy monitor started".into(),
        ),
        AppEvent::PrivacyStopped => (
            "■",
            COLOR_TOGGLE_OFF,
            "Privacy monitor stopped".into(),
        ),
        AppEvent::PrivacyError(msg) => (
            "⚠",
            COLOR_WARN,
            format!("{msg}"),
        ),

        // --- System group ---
        AppEvent::FeatureToggled { feature, enabled } => {
            if *enabled {
                ("●", COLOR_TOGGLE_ON, format!("{feature} enabled"))
            } else {
                ("○", COLOR_TOGGLE_OFF, format!("{feature} disabled"))
            }
        }
        AppEvent::WaybarSpawned => (
            "▶",
            COLOR_INFO,
            "Waybar started".into(),
        ),
        AppEvent::WaybarKilled => (
            "■",
            COLOR_TOGGLE_OFF,
            "Waybar stopped".into(),
        ),
        AppEvent::WaybarError(msg) => (
            "✖",
            COLOR_ERROR,
            format!("{msg}"),
        ),
        AppEvent::AlertsBrowserOpened => (
            "▶",
            COLOR_INFO,
            "Alerts browser opened".into(),
        ),
        AppEvent::AlertsBrowserClosed => (
            "■",
            COLOR_TOGGLE_OFF,
            "Alerts browser closed".into(),
        ),
        AppEvent::Info(msg) => (
            "ℹ",
            COLOR_INFO,
            msg.clone(),
        ),
        AppEvent::Error(msg) => (
            "✖",
            COLOR_ERROR,
            msg.clone(),
        ),

        // --- Hyprland group ---
        AppEvent::WindowOpened { title, .. } => (
            "＋",
            COLOR_WINDOW_OPEN,
            format!("Window: {title}"),
        ),
        AppEvent::WindowClosed { address } => (
            "✕",
            COLOR_WINDOW_CLOSE,
            format!("Window closed: {}", &address[..address.len().min(8)]),
        ),
        AppEvent::WindowTitleChanged { title, .. } => (
            "↔",
            COLOR_TITLE_CHANGE,
            format!("Title: {title}"),
        ),
        AppEvent::WorkspaceChanged { name } => (
            "⊞",
            COLOR_WORKSPACE,
            format!("Workspace: {name}"),
        ),
        AppEvent::MonitorFocused { monitor } => (
            "◉",
            COLOR_MONITOR,
            format!("Monitor: {monitor}"),
        ),
        AppEvent::WindowMoved { workspace, .. } => (
            "↔",
            COLOR_INFO,
            format!("Window moved to {workspace}"),
        ),

        // --- Chat group ---
        AppEvent::ChatMessage { username, text } => (
            "💬",
            COLOR_CHAT,
            format!("{username}: {text}"),
        ),
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
        let help = " q: quit | j/k: navigate | Space/Enter: toggle | Tab: switch pane | 1-5: filter events";
        let bar =
            Paragraph::new(help).style(Style::default().fg(Color::DarkGray).bg(Color::Black));
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
