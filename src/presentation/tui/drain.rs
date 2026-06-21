//! Per-source drain handlers for the run loop.
//!
//! The old monolithic `event_loop` body interleaved every channel's draining
//! and every toggle reaction inline. Here each event source has its own
//! function, so adding a source (or a toggle) is a localised change.

use std::io;

use tokio::sync::{broadcast, mpsc};

use crate::application::AppState;
use crate::domain::{AppEvent, ChatSignal, StreamEvent};
use crate::infrastructure::discord::{DiscordCommand, DiscordStatus};
use crate::infrastructure::hyprland::{PrivacyCommand, PrivacyStatus};
use crate::infrastructure::livepix::{LivepixCommand, LivepixStatus};
use crate::infrastructure::waybar;
use crate::presentation::http::{OverlayCommand, OverlayStatus};

use super::state::{
    ChatEntry, DiscordIntegrationStatus, LivepixIntegrationStatus, PrivacyIntegrationStatus,
    TuiState,
};
use super::theme::maybe_highlight;

// ---------------------------------------------------------------------------
// Stream events (broadcast<StreamEvent>)
// ---------------------------------------------------------------------------

pub fn stream_events(
    app: &mut AppState,
    tui: &mut TuiState,
    rx: &mut broadcast::Receiver<StreamEvent>,
) {
    while let Ok(ev) = rx.try_recv() {
        app.stats.record(&ev);
        if let Some(highlight) = maybe_highlight(&ev) {
            tui.push_highlight(highlight);
        }
        app.log_event(AppEvent::Stream(ev));
    }
}

// ---------------------------------------------------------------------------
// Now-playing (watch<Option<NowPlaying>>) — ambient state, latest-value-wins
// ---------------------------------------------------------------------------

/// Refresh the cached now-playing track from the media-player observer's
/// `watch` channel. Unlike the broadcast/mpsc drains this is not a queue: we
/// just snapshot the latest value each tick (cheap clone, deduped upstream).
pub fn now_playing(
    tui: &mut TuiState,
    rx: &mut tokio::sync::watch::Receiver<Option<crate::domain::NowPlaying>>,
) {
    tui.now_playing = rx.borrow().clone();
}

// ---------------------------------------------------------------------------
// Privacy monitor status
// ---------------------------------------------------------------------------

pub fn privacy_status(
    app: &mut AppState,
    tui: &mut TuiState,
    rx: &mut mpsc::Receiver<PrivacyStatus>,
) {
    while let Ok(status) = rx.try_recv() {
        match &status {
            PrivacyStatus::Running => {
                tui.privacy.running = true;
                tui.privacy.last_error = None;
            }
            PrivacyStatus::Stopped => {
                tui.privacy.running = false;
                tui.privacy.blur_active = false;
                tui.privacy.blur_target = None;
            }
            PrivacyStatus::BlurEnabled { title } => {
                tui.privacy.blur_active = true;
                tui.privacy.blur_target = Some(title.clone());
            }
            PrivacyStatus::BlurDisabled => {
                tui.privacy.blur_active = false;
                tui.privacy.blur_target = None;
            }
            PrivacyStatus::Error(msg) => {
                tui.privacy.last_error = Some(msg.clone());
            }
        }

        match status {
            PrivacyStatus::Running => app.log_event(AppEvent::PrivacyStarted),
            PrivacyStatus::Stopped => app.log_event(AppEvent::PrivacyStopped),
            PrivacyStatus::BlurEnabled { title } => {
                app.log_event(AppEvent::PrivacyBlurEnabled { title })
            }
            PrivacyStatus::BlurDisabled => app.log_event(AppEvent::PrivacyBlurDisabled),
            PrivacyStatus::Error(msg) => app.log_event(AppEvent::PrivacyError(msg)),
        }
    }
}

// ---------------------------------------------------------------------------
// Livepix status
// ---------------------------------------------------------------------------

pub fn livepix_status(
    app: &mut AppState,
    tui: &mut TuiState,
    rx: &mut mpsc::Receiver<LivepixStatus>,
) {
    while let Ok(status) = rx.try_recv() {
        match &status {
            LivepixStatus::Running { port } => {
                tui.livepix.running = true;
                tui.livepix.port = *port;
                tui.livepix.last_error = None;
            }
            LivepixStatus::Stopped => {
                tui.livepix.running = false;
            }
            LivepixStatus::OAuthSuccess => {
                tui.livepix.oauth_ok = true;
            }
            LivepixStatus::OAuthError(msg) => {
                tui.livepix.oauth_ok = false;
                tui.livepix.last_error = Some(msg.clone());
            }
            LivepixStatus::WebhookReceived { .. } => {}
            LivepixStatus::Error(msg) => {
                tui.livepix.last_error = Some(msg.clone());
            }
        }

        match &status {
            LivepixStatus::Running { port } => app.log_event(AppEvent::LivepixInfo(format!(
                "Webhook listening on 127.0.0.1:{port}"
            ))),
            LivepixStatus::Stopped => {
                app.log_event(AppEvent::LivepixInfo("Webhook stopped".into()))
            }
            LivepixStatus::OAuthSuccess => {
                app.log_event(AppEvent::LivepixInfo("OAuth authenticated".into()))
            }
            LivepixStatus::OAuthError(msg) => {
                app.log_event(AppEvent::LivepixError(format!("OAuth failed: {msg}")))
            }
            LivepixStatus::WebhookReceived { username, amount } => app.log_event(
                AppEvent::LivepixInfo(format!("{username} donated {amount}")),
            ),
            LivepixStatus::Error(msg) => app.log_event(AppEvent::LivepixError(msg.clone())),
        }
    }
}

// ---------------------------------------------------------------------------
// Overlays (http server) status
// ---------------------------------------------------------------------------

pub fn overlays_status(
    app: &mut AppState,
    tui: &mut TuiState,
    rx: &mut mpsc::Receiver<OverlayStatus>,
) {
    while let Ok(status) = rx.try_recv() {
        match &status {
            OverlayStatus::Running { port } => {
                tui.overlays.running = true;
                tui.overlays.port = *port;
                tui.overlays.last_error = None;
            }
            OverlayStatus::Stopped => {
                tui.overlays.running = false;
            }
            OverlayStatus::Error(msg) => {
                tui.overlays.running = false;
                tui.overlays.last_error = Some(msg.clone());
                // Surface a bind failure (e.g. port clash) in the status bar.
                app.status_message = Some(format!("Overlays: {msg}"));
                app.overlays_enabled = false;
            }
        }

        match status {
            OverlayStatus::Running { port } => app.log_event(AppEvent::Info(format!(
                "Overlays serving on http://127.0.0.1:{port}/overlay/coworking"
            ))),
            OverlayStatus::Stopped => app.log_event(AppEvent::Info("Overlays stopped".into())),
            OverlayStatus::Error(msg) => app.log_event(AppEvent::Error(format!("Overlays: {msg}"))),
        }
    }
}

// ---------------------------------------------------------------------------
// Discord voice-roster adapter status (four-state lifecycle)
// ---------------------------------------------------------------------------

pub fn discord_status(
    app: &mut AppState,
    tui: &mut TuiState,
    rx: &mut mpsc::Receiver<DiscordStatus>,
) {
    while let Ok(status) = rx.try_recv() {
        match &status {
            DiscordStatus::Stopped => {
                tui.discord.running = false;
                tui.discord.channel = None;
            }
            DiscordStatus::Running { channel } => {
                tui.discord.running = true;
                tui.discord.channel = channel.clone();
                tui.discord.last_error = None;
            }
            DiscordStatus::Activity(_) => {} // log-only; no row-state change
            DiscordStatus::Error(msg) => {
                tui.discord.running = false;
                tui.discord.last_error = Some(msg.clone());
                app.status_message = Some(format!("Discord: {msg}"));
                app.discord_enabled = false;
            }
        }

        match status {
            DiscordStatus::Stopped => app.log_event(AppEvent::Info("Discord stopped".into())),
            DiscordStatus::Activity(msg) => {
                app.log_event(AppEvent::Info(format!("Discord: {msg}")))
            }
            DiscordStatus::Running { channel } => {
                let msg = match channel.as_deref() {
                    Some(c) => format!("Discord connected · #{c}"),
                    None => "Discord bridge active (injecting into Vesktop)".into(),
                };
                app.log_event(AppEvent::Info(msg))
            }
            DiscordStatus::Error(msg) => app.log_event(AppEvent::Error(format!("Discord: {msg}"))),
        }
    }
}

// ---------------------------------------------------------------------------
// Hyprland events
// ---------------------------------------------------------------------------

pub fn hyprland_events(app: &mut AppState, tui: &mut TuiState, rx: &mut mpsc::Receiver<AppEvent>) {
    while let Ok(ev) = rx.try_recv() {
        tui.hyprland.listening = true;
        tui.hyprland.event_count += 1;
        app.log_event(ev);
    }
}

// ---------------------------------------------------------------------------
// Twitch background events (status strings → eventsub/chat indicators)
// ---------------------------------------------------------------------------

pub fn twitch_events(app: &mut AppState, tui: &mut TuiState, rx: &mut mpsc::Receiver<AppEvent>) {
    while let Ok(ev) = rx.try_recv() {
        match &ev {
            AppEvent::Info(msg) => {
                let lower = msg.to_lowercase();
                if lower.contains("connected to eventsub") || lower.contains("eventsub connecting")
                {
                    tui.twitch_eventsub.connected = true;
                }
                if lower.contains("session established") || lower.contains("welcome") {
                    tui.twitch_eventsub.session_established = true;
                }
                if lower.contains("subscribed to") {
                    tui.twitch_eventsub.subs_registered += 1;
                }
                if lower.contains("twitch chat joined") {
                    tui.twitch_chat.connected = true;
                }
            }
            AppEvent::Error(msg) => {
                let lower = msg.to_lowercase();
                if lower.contains("twitch chat disconnected")
                    || lower.contains("twitch chat failed")
                {
                    tui.twitch_chat.connected = false;
                } else if lower.contains("disconnected") || lower.contains("timeout") {
                    tui.twitch_eventsub.connected = false;
                    tui.twitch_eventsub.session_established = false;
                    tui.twitch_eventsub.subs_registered = 0;
                }
                tui.twitch_eventsub.last_error = Some(msg.clone());
            }
            _ => {}
        }
        app.log_event(ev);
    }
}

// ---------------------------------------------------------------------------
// Chat signals (broadcast<ChatSignal>) — terminal chat view preserved
// ---------------------------------------------------------------------------

pub fn chat_signals(
    app: &mut AppState,
    tui: &mut TuiState,
    rx: &mut broadcast::Receiver<ChatSignal>,
) {
    while let Ok(signal) = rx.try_recv() {
        match signal {
            ChatSignal::Message(msg) => {
                tui.twitch_chat.connected = true;
                tui.twitch_chat.message_count += 1;

                let text = msg.plain_text();
                tui.push_chat(ChatEntry {
                    username: msg.username.clone(),
                    text: text.clone(),
                });

                app.log_event(AppEvent::ChatMessage {
                    username: msg.username,
                    text,
                });
            }
            // The TUI chat view is append-only with no per-message id; a single
            // delete is a no-op here (it still reaches the Overlay Feed).
            ChatSignal::Deleted(_) => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Toggle reactions — drive the Output side-effects on toggle changes
// ---------------------------------------------------------------------------

/// Snapshot of the toggles we react to between loop iterations.
#[derive(Clone, Copy)]
pub struct ToggleSnapshot {
    waybar: bool,
    privacy: bool,
    livepix: bool,
    overlays: bool,
    discord: bool,
}

impl ToggleSnapshot {
    pub fn capture(app: &AppState) -> Self {
        Self {
            waybar: app.waybar_enabled,
            privacy: app.privacy_enabled,
            livepix: app.livepix_enabled,
            overlays: app.overlays_enabled,
            discord: app.discord_enabled,
        }
    }
}

/// React to any toggle change since `prev` and return the new snapshot.
#[allow(clippy::too_many_arguments)]
pub async fn react_to_toggles(
    app: &mut AppState,
    tui: &mut TuiState,
    prev: &ToggleSnapshot,
    waybar_output: &str,
    privacy_cmd_tx: &mpsc::Sender<PrivacyCommand>,
    livepix_cmd_tx: &mpsc::Sender<LivepixCommand>,
    overlays_cmd_tx: &mpsc::Sender<OverlayCommand>,
    discord_cmd_tx: &mpsc::Sender<DiscordCommand>,
) -> ToggleSnapshot {
    if app.waybar_enabled != prev.waybar {
        react_waybar(app, waybar_output).await;
    }

    if app.privacy_enabled != prev.privacy {
        let cmd = if app.privacy_enabled {
            PrivacyCommand::Start
        } else {
            tui.privacy = PrivacyIntegrationStatus::default();
            PrivacyCommand::Stop
        };
        let _ = privacy_cmd_tx.send(cmd).await;
    }

    if app.livepix_enabled != prev.livepix {
        let cmd = if app.livepix_enabled {
            LivepixCommand::Start
        } else {
            tui.livepix = LivepixIntegrationStatus::default();
            LivepixCommand::Stop
        };
        let _ = livepix_cmd_tx.send(cmd).await;
    }

    if app.overlays_enabled != prev.overlays {
        let cmd = if app.overlays_enabled {
            OverlayCommand::Start
        } else {
            OverlayCommand::Stop
        };
        let _ = overlays_cmd_tx.send(cmd).await;
    }

    if app.discord_enabled != prev.discord {
        let cmd = if app.discord_enabled {
            DiscordCommand::Start
        } else {
            tui.discord = DiscordIntegrationStatus::default();
            DiscordCommand::Stop
        };
        let _ = discord_cmd_tx.send(cmd).await;
    }

    ToggleSnapshot::capture(app)
}

async fn react_waybar(app: &mut AppState, waybar_output: &str) {
    if app.waybar_enabled {
        match waybar::enable(waybar_output).await {
            Ok(()) => {
                app.status_message = None;
                app.log_event(AppEvent::WaybarSpawned);
            }
            Err(e) => {
                let msg = waybar_error_message(&e);
                app.log_event(AppEvent::WaybarError(msg.clone()));
                app.status_message = Some(msg);
                app.waybar_enabled = false;
                tracing::warn!("failed to enable waybar stream bar: {e}");
            }
        }
    } else {
        if let Err(e) = waybar::disable().await {
            tracing::warn!("failed to disable waybar stream bar: {e}");
        }
        app.log_event(AppEvent::WaybarKilled);
        app.status_message = None;
    }
}

fn waybar_error_message(err: &io::Error) -> String {
    super::waybar_error_message(err)
}
