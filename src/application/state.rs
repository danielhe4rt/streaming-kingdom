use std::time::Instant;

use tokio::sync::{broadcast, mpsc};

use crate::application::config::EventLogConfig;
use crate::domain::{AppEvent, AppEventEntry, ChatMessage, FeatureCommand, StreamEvent, StreamStats};

// ---------------------------------------------------------------------------
// Central application state
// ---------------------------------------------------------------------------

const EVENT_CHANNEL_CAPACITY: usize = 256;
const CHAT_CHANNEL_CAPACITY: usize = 256;
const COMMAND_CHANNEL_CAPACITY: usize = 64;

pub struct AppState {
    // Feature toggles
    pub waybar_enabled: bool,
    pub privacy_enabled: bool,
    pub alerts_enabled: bool,
    pub livepix_enabled: bool,

    // broadcast: stream events (1 producer → N consumers)
    pub event_tx: broadcast::Sender<StreamEvent>,

    // broadcast: chat messages (IRC producer → TUI + Overlay Feed consumers).
    // Mirrors event_tx so neither the TUI nor the http feed owns the chat.
    pub chat_tx: broadcast::Sender<ChatMessage>,

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
        let (chat_tx, _) = broadcast::channel(CHAT_CHANNEL_CAPACITY);
        let (command_tx, command_rx) = mpsc::channel(COMMAND_CHANNEL_CAPACITY);

        Self {
            waybar_enabled: false,
            privacy_enabled: false,
            alerts_enabled: true,
            livepix_enabled: false,
            event_tx,
            chat_tx,
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

    /// Subscribe to the chat broadcast channel (TUI + Overlay Feed).
    pub fn subscribe_chat(&self) -> broadcast::Receiver<ChatMessage> {
        self.chat_tx.subscribe()
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
            FeatureCommand::EnableLivepix => {
                self.livepix_enabled = true;
                self.log_event(AppEvent::FeatureToggled {
                    feature: "Livepix".into(),
                    enabled: true,
                });
            }
            FeatureCommand::DisableLivepix => {
                self.livepix_enabled = false;
                self.log_event(AppEvent::FeatureToggled {
                    feature: "Livepix".into(),
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
