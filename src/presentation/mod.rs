mod input;
pub mod state;
pub mod theme;
mod ui;

use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::event::{self, Event};
use ratatui::Terminal;
use ratatui::prelude::*;
use tokio::sync::{broadcast, mpsc};

use crate::application::{AppState, EventLogConfig};
use crate::domain::{AppEvent, ChatMessage, FeatureCommand, StreamEvent};
use crate::infrastructure::hyprland::{PrivacyCommand, PrivacyStatus};
use crate::infrastructure::livepix::{LivepixCommand, LivepixStatus};
use crate::infrastructure::waybar;

use state::{ChatEntry, LivepixIntegrationStatus, PrivacyIntegrationStatus, TuiState};
use theme::maybe_highlight;

// ---------------------------------------------------------------------------
// Terminal setup / teardown
// ---------------------------------------------------------------------------

fn init_terminal() -> io::Result<Terminal<CrosstermBackend<Stdout>>> {
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    Terminal::new(backend)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Main run loop
// ---------------------------------------------------------------------------

const TICK: Duration = Duration::from_millis(33); // ~30 fps

pub async fn run(
    app: &mut AppState,
    waybar_output: &str,
    privacy_cmd_tx: mpsc::Sender<PrivacyCommand>,
    privacy_status_rx: mpsc::Receiver<PrivacyStatus>,
    livepix_cmd_tx: mpsc::Sender<LivepixCommand>,
    livepix_status_rx: mpsc::Receiver<LivepixStatus>,
    event_log_config: &EventLogConfig,
    mut hyprland_rx: mpsc::Receiver<AppEvent>,
    mut twitch_event_rx: mpsc::Receiver<AppEvent>,
    mut chat_rx: mpsc::Receiver<ChatMessage>,
    twitch_channel: &str,
    tts_available: bool,
) -> io::Result<()> {
    let mut terminal = init_terminal()?;
    let mut tui = TuiState::new(event_log_config, twitch_channel, tts_available);
    let mut event_rx = app.subscribe_events();
    let cmd_tx = app.command_sender();

    // Enable the stream bar if waybar starts enabled
    if app.waybar_enabled
        && let Err(e) = waybar::enable(waybar_output).await
    {
        app.status_message = Some(waybar_error_message(&e));
        app.waybar_enabled = false;
        tracing::warn!("failed to enable waybar stream bar: {e}");
    }

    let mut privacy_status_rx = privacy_status_rx;
    let mut livepix_status_rx = livepix_status_rx;

    let result = event_loop(
        &mut terminal,
        app,
        &mut tui,
        &mut event_rx,
        &cmd_tx,
        waybar_output,
        &privacy_cmd_tx,
        &mut privacy_status_rx,
        &livepix_cmd_tx,
        &mut livepix_status_rx,
        &mut hyprland_rx,
        &mut twitch_event_rx,
        &mut chat_rx,
    )
    .await;

    // Send Stop to privacy monitor for clean shutdown
    let _ = privacy_cmd_tx.send(PrivacyCommand::Stop).await;

    // Send Stop to Livepix for clean shutdown
    let _ = livepix_cmd_tx.send(LivepixCommand::Stop).await;

    // Clean up stream bar from waybar config on exit
    if app.waybar_enabled
        && let Err(e) = waybar::disable().await
    {
        tracing::warn!("failed to disable waybar stream bar on exit: {e}");
    }

    restore_terminal(&mut terminal)?;
    result
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut AppState,
    tui: &mut TuiState,
    event_rx: &mut broadcast::Receiver<StreamEvent>,
    cmd_tx: &mpsc::Sender<FeatureCommand>,
    waybar_output: &str,
    privacy_cmd_tx: &mpsc::Sender<PrivacyCommand>,
    privacy_status_rx: &mut mpsc::Receiver<PrivacyStatus>,
    livepix_cmd_tx: &mpsc::Sender<LivepixCommand>,
    livepix_status_rx: &mut mpsc::Receiver<LivepixStatus>,
    hyprland_rx: &mut mpsc::Receiver<AppEvent>,
    twitch_event_rx: &mut mpsc::Receiver<AppEvent>,
    chat_rx: &mut mpsc::Receiver<ChatMessage>,
) -> io::Result<()> {
    let mut prev_waybar_enabled = app.waybar_enabled;
    let mut prev_privacy_enabled = app.privacy_enabled;
    let mut prev_livepix_enabled = app.livepix_enabled;

    loop {
        // Draw
        terminal.draw(|frame| ui::draw(frame, app, tui))?;

        // Poll for crossterm events with a short timeout so we can also
        // drain stream events from the broadcast channel.
        if event::poll(TICK)?
            && let Event::Key(key) = event::read()?
            && input::handle_key(key, app, tui, cmd_tx).await
        {
            return Ok(());
        }

        // Drain any pending stream events into stats / event log / highlights.
        while let Ok(ev) = event_rx.try_recv() {
            app.stats.record(&ev);
            if let Some(highlight) = maybe_highlight(&ev) {
                tui.push_highlight(highlight);
            }
            app.log_event(AppEvent::Stream(ev));
        }

        // Drain privacy status updates — populate structured status.
        while let Ok(status) = privacy_status_rx.try_recv() {
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

            // Log privacy events to the unified event log.
            match status {
                PrivacyStatus::Running => {
                    app.log_event(AppEvent::PrivacyStarted);
                }
                PrivacyStatus::Stopped => {
                    app.log_event(AppEvent::PrivacyStopped);
                }
                PrivacyStatus::BlurEnabled { title } => {
                    app.log_event(AppEvent::PrivacyBlurEnabled { title });
                }
                PrivacyStatus::BlurDisabled => {
                    app.log_event(AppEvent::PrivacyBlurDisabled);
                }
                PrivacyStatus::Error(msg) => {
                    app.log_event(AppEvent::PrivacyError(msg));
                }
            }
        }

        // Drain Livepix status updates — populate structured status.
        while let Ok(status) = livepix_status_rx.try_recv() {
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

            // Log to unified event log using Livepix-specific variants.
            match &status {
                LivepixStatus::Running { port } => {
                    app.log_event(AppEvent::LivepixInfo(format!(
                        "Webhook listening on 127.0.0.1:{port}"
                    )));
                }
                LivepixStatus::Stopped => {
                    app.log_event(AppEvent::LivepixInfo("Webhook stopped".into()));
                }
                LivepixStatus::OAuthSuccess => {
                    app.log_event(AppEvent::LivepixInfo("OAuth authenticated".into()));
                }
                LivepixStatus::OAuthError(msg) => {
                    app.log_event(AppEvent::LivepixError(format!("OAuth failed: {msg}")));
                }
                LivepixStatus::WebhookReceived { username, amount } => {
                    app.log_event(AppEvent::LivepixInfo(format!(
                        "{username} donated {amount}"
                    )));
                }
                LivepixStatus::Error(msg) => {
                    app.log_event(AppEvent::LivepixError(msg.clone()));
                }
            }
        }

        // Drain Hyprland events into the event log.
        while let Ok(ev) = hyprland_rx.try_recv() {
            tui.hyprland.listening = true;
            tui.hyprland.event_count += 1;
            app.log_event(ev);
        }

        // Drain Twitch background events into the event log and update status.
        while let Ok(ev) = twitch_event_rx.try_recv() {
            // Parse info/error strings to populate eventsub + chat status
            match &ev {
                AppEvent::Info(msg) => {
                    let lower = msg.to_lowercase();
                    if lower.contains("connected to eventsub")
                        || lower.contains("eventsub connecting")
                    {
                        tui.twitch_eventsub.connected = true;
                    }
                    if lower.contains("session established") || lower.contains("welcome") {
                        tui.twitch_eventsub.session_established = true;
                    }
                    if lower.contains("subscribed to") {
                        tui.twitch_eventsub.subs_registered += 1;
                    }
                    // Chat status
                    if lower.contains("twitch chat joined") {
                        tui.twitch_chat.connected = true;
                    }
                }
                AppEvent::Error(msg) => {
                    let lower = msg.to_lowercase();
                    // Chat disconnect
                    if lower.contains("twitch chat disconnected")
                        || lower.contains("twitch chat failed")
                    {
                        tui.twitch_chat.connected = false;
                    }
                    // EventSub disconnect
                    else if lower.contains("disconnected") || lower.contains("timeout") {
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

        // Drain Twitch IRC chat messages into chat buffer and event log.
        while let Ok(msg) = chat_rx.try_recv() {
            tui.twitch_chat.connected = true;
            tui.twitch_chat.message_count += 1;

            tui.push_chat(ChatEntry {
                username: msg.username.clone(),
                text: msg.text.clone(),
            });

            app.log_event(AppEvent::ChatMessage {
                username: msg.username,
                text: msg.text,
            });
        }

        // Process any pending feature commands.
        app.process_pending_commands();

        // React to waybar toggle changes
        if app.waybar_enabled != prev_waybar_enabled {
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
            prev_waybar_enabled = app.waybar_enabled;
        }

        // React to privacy toggle changes
        if app.privacy_enabled != prev_privacy_enabled {
            let cmd = if app.privacy_enabled {
                PrivacyCommand::Start
            } else {
                tui.privacy = PrivacyIntegrationStatus::default();
                PrivacyCommand::Stop
            };
            let _ = privacy_cmd_tx.send(cmd).await;
            prev_privacy_enabled = app.privacy_enabled;
        }

        // React to Livepix toggle changes
        if app.livepix_enabled != prev_livepix_enabled {
            let cmd = if app.livepix_enabled {
                LivepixCommand::Start
            } else {
                tui.livepix = LivepixIntegrationStatus::default();
                LivepixCommand::Stop
            };
            let _ = livepix_cmd_tx.send(cmd).await;
            prev_livepix_enabled = app.livepix_enabled;
        }
    }
}

fn waybar_error_message(err: &io::Error) -> String {
    if err.kind() == io::ErrorKind::NotFound {
        "waybar config not found — is Omarchy's waybar installed?".into()
    } else {
        format!("waybar error: {err}")
    }
}
