use std::time::Instant;

use tokio::sync::{broadcast, mpsc, watch};

use crate::application::config::EventLogConfig;
use crate::domain::{
    AppEvent, AppEventEntry, ChatSignal, FeatureCommand, NowPlaying, StreamEvent, StreamStats,
    VoiceRoster,
};

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
    pub overlays_enabled: bool,
    pub discord_enabled: bool,

    // broadcast: stream events (1 producer → N consumers)
    pub event_tx: broadcast::Sender<StreamEvent>,

    // broadcast: chat signals — new messages and CLEARMSG deletions (IRC
    // producer → TUI + Overlay Feed consumers). Mirrors event_tx so neither the
    // TUI nor the http feed owns the chat; both moderation and messages ride the
    // same channel so a delete can never overtake the message it removes out of band.
    pub chat_tx: broadcast::Sender<ChatSignal>,

    // mpsc: TUI commands → feature modules
    pub command_tx: mpsc::Sender<FeatureCommand>,
    pub command_rx: mpsc::Receiver<FeatureCommand>,

    // watch: ambient now-playing STATE (latest value, not an event). The media
    // player observer publishes here; the Overlay Feed and the TUI Spotify row
    // both subscribe. `None` means stopped / no player. NOT logged into AppEvent.
    pub now_playing_tx: watch::Sender<Option<NowPlaying>>,

    // watch: ambient voice-roster STATE (latest value, not an event). The Discord
    // RPC adapter publishes here; the Overlay Feed subscribes. `None` means no
    // active voice channel / Discord disconnected. NOT logged into AppEvent.
    pub voice_roster_tx: watch::Sender<Option<VoiceRoster>>,

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
        // Keep the sender; the initial receiver is dropped — consumers subscribe
        // on demand via `subscribe_now_playing()`.
        let (now_playing_tx, _) = watch::channel(None);
        let (voice_roster_tx, _) = watch::channel(None);

        Self {
            waybar_enabled: false,
            privacy_enabled: false,
            alerts_enabled: true,
            livepix_enabled: false,
            overlays_enabled: false,
            discord_enabled: false,
            event_tx,
            chat_tx,
            command_tx,
            command_rx,
            now_playing_tx,
            voice_roster_tx,
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
    pub fn subscribe_chat(&self) -> broadcast::Receiver<ChatSignal> {
        self.chat_tx.subscribe()
    }

    /// Get a cloneable command sender for the TUI (or tests).
    pub fn command_sender(&self) -> mpsc::Sender<FeatureCommand> {
        self.command_tx.clone()
    }

    /// Subscribe to the ambient now-playing state (Overlay Feed + TUI).
    pub fn subscribe_now_playing(&self) -> watch::Receiver<Option<NowPlaying>> {
        self.now_playing_tx.subscribe()
    }

    /// Subscribe to the ambient voice-roster state (Overlay Feed).
    pub fn subscribe_voice_roster(&self) -> watch::Receiver<Option<VoiceRoster>> {
        self.voice_roster_tx.subscribe()
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
            FeatureCommand::EnableOverlays => {
                self.overlays_enabled = true;
                self.log_event(AppEvent::FeatureToggled {
                    feature: "Overlays".into(),
                    enabled: true,
                });
            }
            FeatureCommand::DisableOverlays => {
                self.overlays_enabled = false;
                self.log_event(AppEvent::FeatureToggled {
                    feature: "Overlays".into(),
                    enabled: false,
                });
            }
            FeatureCommand::EnableDiscord => {
                self.discord_enabled = true;
                self.log_event(AppEvent::FeatureToggled {
                    feature: "Discord".into(),
                    enabled: true,
                });
            }
            FeatureCommand::DisableDiscord => {
                self.discord_enabled = false;
                self.log_event(AppEvent::FeatureToggled {
                    feature: "Discord".into(),
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
