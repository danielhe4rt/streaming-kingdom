use std::collections::VecDeque;

use ratatui::prelude::*;

use crate::application::EventLogConfig;

// ---------------------------------------------------------------------------
// Pane enum (4 panes)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Integrations,
    Stats,
    Chat,
    EventLog,
}

impl Pane {
    pub fn next(self) -> Self {
        match self {
            Pane::Integrations => Pane::Stats,
            Pane::Stats => Pane::Chat,
            Pane::Chat => Pane::EventLog,
            Pane::EventLog => Pane::Integrations,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Pane::Integrations => Pane::EventLog,
            Pane::Stats => Pane::Integrations,
            Pane::Chat => Pane::Stats,
            Pane::EventLog => Pane::Chat,
        }
    }
}

// ---------------------------------------------------------------------------
// Integration status structs
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct TwitchEventSubStatus {
    pub connected: bool,
    pub session_established: bool,
    pub subs_registered: u32,
    pub subs_total: u32,
    pub last_error: Option<String>,
}

#[derive(Debug)]
pub struct TwitchChatStatus {
    pub connected: bool,
    pub channel: String,
    pub message_count: u64,
}

impl TwitchChatStatus {
    pub fn new(channel: &str) -> Self {
        Self {
            connected: false,
            channel: channel.to_string(),
            message_count: 0,
        }
    }
}

#[derive(Debug, Default)]
pub struct LivepixIntegrationStatus {
    pub running: bool,
    pub port: u16,
    pub oauth_ok: bool,
    pub last_error: Option<String>,
}

#[derive(Debug, Default)]
pub struct PrivacyIntegrationStatus {
    pub running: bool,
    pub blur_active: bool,
    pub blur_target: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Default)]
pub struct HyprlandIntegrationStatus {
    pub listening: bool,
    pub event_count: u64,
}

// ---------------------------------------------------------------------------
// Chat & highlight buffers
// ---------------------------------------------------------------------------

const CHAT_BUFFER_CAPACITY: usize = 200;
const HIGHLIGHT_BUFFER_CAPACITY: usize = 10;

#[derive(Debug, Clone)]
pub struct ChatEntry {
    pub username: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct HighlightEntry {
    pub icon: &'static str,
    pub text: String,
    pub color: Color,
}

// ---------------------------------------------------------------------------
// TUI state that lives alongside (not inside) AppState
// ---------------------------------------------------------------------------

pub struct TuiState {
    pub focused_pane: Pane,
    pub integration_cursor: usize,
    pub event_log_scroll: u16,
    pub chat_scroll: u16,

    // Integration status structs
    pub twitch_eventsub: TwitchEventSubStatus,
    pub twitch_chat: TwitchChatStatus,
    pub livepix: LivepixIntegrationStatus,
    pub privacy: PrivacyIntegrationStatus,
    pub hyprland: HyprlandIntegrationStatus,

    // Chat ring buffer (separate from event log)
    pub chat_messages: VecDeque<ChatEntry>,

    // Highlights ring buffer
    pub highlights: VecDeque<HighlightEntry>,

    // Event log group filters (true = visible)
    pub filter_stream: bool,
    pub filter_privacy: bool,
    pub filter_system: bool,
    pub filter_hyprland: bool,
    pub filter_livepix: bool,
    pub filter_chat: bool,

    // Feature flags
    pub tts_available: bool,
}

impl TuiState {
    pub fn new(
        event_log_config: &EventLogConfig,
        twitch_channel: &str,
        tts_available: bool,
    ) -> Self {
        Self {
            focused_pane: Pane::Integrations,
            integration_cursor: 0,
            event_log_scroll: 0,
            chat_scroll: 0,
            twitch_eventsub: TwitchEventSubStatus::default(),
            twitch_chat: TwitchChatStatus::new(twitch_channel),
            livepix: LivepixIntegrationStatus::default(),
            privacy: PrivacyIntegrationStatus::default(),
            hyprland: HyprlandIntegrationStatus::default(),
            chat_messages: VecDeque::with_capacity(CHAT_BUFFER_CAPACITY),
            highlights: VecDeque::with_capacity(HIGHLIGHT_BUFFER_CAPACITY),
            filter_stream: event_log_config.show_stream,
            filter_privacy: event_log_config.show_privacy,
            filter_system: event_log_config.show_system,
            filter_hyprland: event_log_config.show_hyprland,
            filter_livepix: event_log_config.show_livepix,
            filter_chat: event_log_config.show_chat,
            tts_available,
        }
    }

    pub fn push_chat(&mut self, entry: ChatEntry) {
        if self.chat_messages.len() >= CHAT_BUFFER_CAPACITY {
            self.chat_messages.pop_front();
        }
        self.chat_messages.push_back(entry);
    }

    pub fn push_highlight(&mut self, entry: HighlightEntry) {
        if self.highlights.len() >= HIGHLIGHT_BUFFER_CAPACITY {
            self.highlights.pop_front();
        }
        self.highlights.push_back(entry);
    }
}
