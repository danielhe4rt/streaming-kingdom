use std::time::Duration;

use super::events::StreamEvent;

// ---------------------------------------------------------------------------
// Unified event log – all system events in one place
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventGroup {
    Stream,
    Privacy,
    System,
    Hyprland,
    Livepix,
    Chat,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    // --- Group: Stream ---
    Stream(StreamEvent),

    // --- Group: Privacy ---
    PrivacyBlurEnabled { title: String },
    PrivacyBlurDisabled,
    PrivacyStarted,
    PrivacyStopped,
    PrivacyError(String),

    // --- Group: System ---
    FeatureToggled { feature: String, enabled: bool },
    WaybarSpawned,
    WaybarKilled,
    WaybarError(String),
    Info(String),
    Error(String),

    // --- Group: Livepix ---
    LivepixInfo(String),
    LivepixError(String),

    // --- Group: Hyprland ---
    WindowOpened { title: String },
    WindowClosed { address: String },
    WindowTitleChanged { title: String },
    WorkspaceChanged { name: String },
    MonitorFocused { monitor: String },
    WindowMoved { workspace: String },

    // --- Group: Chat ---
    ChatMessage { username: String, text: String },
}

impl AppEvent {
    pub fn group(&self) -> EventGroup {
        match self {
            AppEvent::Stream(_) => EventGroup::Stream,
            AppEvent::PrivacyBlurEnabled { .. }
            | AppEvent::PrivacyBlurDisabled
            | AppEvent::PrivacyStarted
            | AppEvent::PrivacyStopped
            | AppEvent::PrivacyError(_) => EventGroup::Privacy,
            AppEvent::WindowOpened { .. }
            | AppEvent::WindowClosed { .. }
            | AppEvent::WindowTitleChanged { .. }
            | AppEvent::WorkspaceChanged { .. }
            | AppEvent::MonitorFocused { .. }
            | AppEvent::WindowMoved { .. } => EventGroup::Hyprland,
            AppEvent::ChatMessage { .. } => EventGroup::Chat,
            AppEvent::LivepixInfo(_) | AppEvent::LivepixError(_) => EventGroup::Livepix,
            _ => EventGroup::System,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppEventEntry {
    pub event: AppEvent,
    pub elapsed: Duration,
}
