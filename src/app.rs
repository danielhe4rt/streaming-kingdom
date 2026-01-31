use serde::Serialize;
use std::time::Instant;
use tokio::sync::{broadcast, mpsc};

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
// Stream stats – running counters reset per-session
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct StreamStats {
    pub viewer_count: u32,
    pub followers_today: u32,
    pub subs_today: u32,
    pub last_events: Vec<StreamEvent>,
}

const MAX_LAST_EVENTS: usize = 50;

impl StreamStats {
    fn new() -> Self {
        Self {
            viewer_count: 0,
            followers_today: 0,
            subs_today: 0,
            last_events: Vec::new(),
        }
    }

    pub fn record(&mut self, event: &StreamEvent) {
        match event {
            StreamEvent::Follow { .. } => self.followers_today += 1,
            StreamEvent::Sub { .. } => self.subs_today += 1,
            StreamEvent::ViewerCountUpdate { count } => self.viewer_count = *count,
            _ => {}
        }
        self.last_events.push(event.clone());
        if self.last_events.len() > MAX_LAST_EVENTS {
            self.last_events.remove(0);
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

    // Session start time
    pub started_at: Instant,

    // Transient status/error message shown in the TUI status bar
    pub status_message: Option<String>,
}

impl AppState {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        let (command_tx, command_rx) = mpsc::channel(COMMAND_CHANNEL_CAPACITY);

        Self {
            waybar_enabled: true,
            privacy_enabled: false,
            alerts_enabled: true,
            event_tx,
            command_tx,
            command_rx,
            stats: StreamStats::new(),
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
        // Ignore send error – means no active receivers yet.
        let _ = self.event_tx.send(event);
    }

    /// Apply a feature command, updating the corresponding toggle.
    pub fn apply_command(&mut self, cmd: &FeatureCommand) {
        match cmd {
            FeatureCommand::EnableWaybar => self.waybar_enabled = true,
            FeatureCommand::DisableWaybar => self.waybar_enabled = false,
            FeatureCommand::EnablePrivacy => self.privacy_enabled = true,
            FeatureCommand::DisablePrivacy => self.privacy_enabled = false,
            FeatureCommand::EnableAlerts => self.alerts_enabled = true,
            FeatureCommand::DisableAlerts => self.alerts_enabled = false,
        }
    }

    /// Drain pending commands from the mpsc channel (non-blocking).
    pub fn process_pending_commands(&mut self) {
        while let Ok(cmd) = self.command_rx.try_recv() {
            self.apply_command(&cmd);
        }
    }
}
