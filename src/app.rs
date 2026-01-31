use serde::Serialize;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};

use crate::config::EventLogConfig;

// ---------------------------------------------------------------------------
// Stream events – one producer (stream listener), N consumer modules
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StreamEvent {
    Follow {
        username: String,
    },
    Sub {
        username: String,
        tier: SubTier,
        months: u32,
    },
    Donation {
        username: String,
        amount_cents: u64,
        message: String,
    },
    GiftSub {
        username: String,
        tier: SubTier,
        total: u32,
    },
    Cheer {
        username: String,
        bits: u64,
        message: String,
    },
    Raid {
        from_channel: String,
        viewers: u32,
    },
    ViewerCountUpdate {
        count: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SubTier {
    Tier1,
    Tier2,
    Tier3,
    Prime,
}

// ---------------------------------------------------------------------------
// Feature commands – TUI sends these to toggle/control feature modules
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeatureCommand {
    EnableWaybar,
    DisableWaybar,
    EnablePrivacy,
    DisablePrivacy,
    EnableAlerts,
    DisableAlerts,
}

// ---------------------------------------------------------------------------
// Unified event log – all system events in one place
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventGroup {
    Stream,
    Privacy,
    System,
    Hyprland,
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
    AlertsBrowserOpened,
    AlertsBrowserClosed,
    Info(String),
    Error(String),

    // --- Group: Hyprland ---
    WindowOpened { address: String, title: String },
    WindowClosed { address: String },
    WindowTitleChanged { address: String, title: String },
    WorkspaceChanged { name: String },
    MonitorFocused { monitor: String },
    WindowMoved { address: String, workspace: String },
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
            _ => EventGroup::System,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppEventEntry {
    pub event: AppEvent,
    pub elapsed: Duration,
}

// ---------------------------------------------------------------------------
// Stream stats – running counters reset per-session
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct StreamStats {
    pub viewer_count: u32,
    pub followers_today: u32,
    pub subs_today: u32,
}

impl StreamStats {
    fn new() -> Self {
        Self {
            viewer_count: 0,
            followers_today: 0,
            subs_today: 0,
        }
    }

    pub fn record(&mut self, event: &StreamEvent) {
        match event {
            StreamEvent::Follow { .. } => self.followers_today += 1,
            StreamEvent::Sub { .. } | StreamEvent::GiftSub { .. } => self.subs_today += 1,
            StreamEvent::ViewerCountUpdate { count } => self.viewer_count = *count,
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Central application state
// ---------------------------------------------------------------------------

const EVENT_CHANNEL_CAPACITY: usize = 256;
const COMMAND_CHANNEL_CAPACITY: usize = 64;

pub struct AppState {
    // Feature toggles
    pub waybar_enabled: bool,
    pub privacy_enabled: bool,
    pub alerts_enabled: bool,

    // broadcast: stream events (1 producer → N consumers)
    pub event_tx: broadcast::Sender<StreamEvent>,

    // mpsc: TUI commands → feature modules
    pub command_tx: mpsc::Sender<FeatureCommand>,
    pub command_rx: mpsc::Receiver<FeatureCommand>,

    // Running stats
    pub stats: StreamStats,

    // Unified event log
    pub event_log: Vec<AppEventEntry>,
    pub max_events: usize,

    // Session start time
    pub started_at: Instant,

    // Transient status/error message shown in the TUI status bar
    pub status_message: Option<String>,
}

impl AppState {
    pub fn new(event_log_config: &EventLogConfig) -> Self {
        let (event_tx, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        let (command_tx, command_rx) = mpsc::channel(COMMAND_CHANNEL_CAPACITY);

        Self {
            waybar_enabled: false,
            privacy_enabled: false,
            alerts_enabled: true,
            event_tx,
            command_tx,
            command_rx,
            stats: StreamStats::new(),
            event_log: Vec::new(),
            max_events: event_log_config.max_events,
            started_at: Instant::now(),
            status_message: None,
        }
    }

    /// Subscribe to the stream-event broadcast channel.
    pub fn subscribe_events(&self) -> broadcast::Receiver<StreamEvent> {
        self.event_tx.subscribe()
    }

    /// Get a cloneable command sender for the TUI (or tests).
    pub fn command_sender(&self) -> mpsc::Sender<FeatureCommand> {
        self.command_tx.clone()
    }

    /// Broadcast a stream event and update stats.
    pub fn dispatch_event(&mut self, event: StreamEvent) {
        self.stats.record(&event);
        self.log_event(AppEvent::Stream(event.clone()));
        // Ignore send error – means no active receivers yet.
        let _ = self.event_tx.send(event);
    }

    /// Log a unified event into the event log with timestamp.
    pub fn log_event(&mut self, event: AppEvent) {
        let elapsed = self.started_at.elapsed();
        self.event_log.push(AppEventEntry { event, elapsed });
        if self.event_log.len() > self.max_events {
            self.event_log.remove(0);
        }
    }

    /// Apply a feature command, updating the corresponding toggle.
    pub fn apply_command(&mut self, cmd: &FeatureCommand) {
        match cmd {
            FeatureCommand::EnableWaybar => {
                self.waybar_enabled = true;
                self.log_event(AppEvent::FeatureToggled {
                    feature: "Waybar".into(),
                    enabled: true,
                });
            }
            FeatureCommand::DisableWaybar => {
                self.waybar_enabled = false;
                self.log_event(AppEvent::FeatureToggled {
                    feature: "Waybar".into(),
                    enabled: false,
                });
            }
            FeatureCommand::EnablePrivacy => {
                self.privacy_enabled = true;
                self.log_event(AppEvent::FeatureToggled {
                    feature: "Privacy".into(),
                    enabled: true,
                });
            }
            FeatureCommand::DisablePrivacy => {
                self.privacy_enabled = false;
                self.log_event(AppEvent::FeatureToggled {
                    feature: "Privacy".into(),
                    enabled: false,
                });
            }
            FeatureCommand::EnableAlerts => {
                self.alerts_enabled = true;
                self.log_event(AppEvent::FeatureToggled {
                    feature: "Alerts".into(),
                    enabled: true,
                });
            }
            FeatureCommand::DisableAlerts => {
                self.alerts_enabled = false;
                self.log_event(AppEvent::FeatureToggled {
                    feature: "Alerts".into(),
                    enabled: false,
                });
            }
        }
    }

    /// Drain pending commands from the mpsc channel (non-blocking).
    pub fn process_pending_commands(&mut self) {
        while let Ok(cmd) = self.command_rx.try_recv() {
            self.apply_command(&cmd);
        }
    }
}
