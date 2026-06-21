//! `tui` renderer — the terminal control panel (ADR-0001, outbound render).
//!
//! Redesigned as a **nav shell** (prototype variant D): topbar sections →
//! sidebar sub-nav → content. The data-driven [`nav`]/[`service`] models hold
//! the pure transitions; [`panes`] render them; [`input`] drives navigation and
//! toggle resolution. The run loop drains each event source through its own
//! handler in [`drain`] rather than one monolithic block.

mod drain;
mod input;
pub mod nav;
mod panes;
pub mod service;
pub mod state;
pub mod theme;

use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::event::{self, Event};
use ratatui::Terminal;
use ratatui::prelude::*;
use tokio::sync::{broadcast, mpsc, watch};

use crate::application::{AppState, EventLogConfig};
use crate::domain::{AppEvent, ChatSignal, FeatureCommand, NowPlaying, StreamEvent};
use crate::infrastructure::discord::{DiscordCommand, DiscordStatus};
use crate::infrastructure::hyprland::{PrivacyCommand, PrivacyStatus};
use crate::infrastructure::livepix::{LivepixCommand, LivepixStatus};
use crate::infrastructure::waybar;
use crate::presentation::http::{OverlayCommand, OverlayStatus};

use state::TuiState;

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

/// All channels and config the run loop needs. Bundled into a struct to keep
/// the function signature manageable now that Overlays adds its own channel.
pub struct RunChannels {
    pub privacy_cmd_tx: mpsc::Sender<PrivacyCommand>,
    pub privacy_status_rx: mpsc::Receiver<PrivacyStatus>,
    pub livepix_cmd_tx: mpsc::Sender<LivepixCommand>,
    pub livepix_status_rx: mpsc::Receiver<LivepixStatus>,
    pub overlays_cmd_tx: mpsc::Sender<OverlayCommand>,
    pub overlays_status_rx: mpsc::Receiver<OverlayStatus>,
    pub discord_cmd_tx: mpsc::Sender<DiscordCommand>,
    pub discord_status_rx: mpsc::Receiver<DiscordStatus>,
    pub hyprland_rx: mpsc::Receiver<AppEvent>,
    pub twitch_event_rx: mpsc::Receiver<AppEvent>,
}

#[allow(clippy::too_many_arguments)]
pub async fn run(
    app: &mut AppState,
    waybar_output: &str,
    channels: RunChannels,
    event_log_config: &EventLogConfig,
    twitch_channel: &str,
    tts_available: bool,
    overlays_port: u16,
    mut now_playing_rx: watch::Receiver<Option<NowPlaying>>,
) -> io::Result<()> {
    let mut terminal = init_terminal()?;
    let mut tui = TuiState::new(
        event_log_config,
        twitch_channel,
        tts_available,
        overlays_port,
    );
    let mut event_rx = app.subscribe_events();
    let mut chat_rx = app.subscribe_chat();
    let cmd_tx = app.command_sender();

    // Enable the stream bar if waybar starts enabled
    if app.waybar_enabled
        && let Err(e) = waybar::enable(waybar_output).await
    {
        app.status_message = Some(waybar_error_message(&e));
        app.waybar_enabled = false;
        tracing::warn!("failed to enable waybar stream bar: {e}");
    }

    let RunChannels {
        privacy_cmd_tx,
        mut privacy_status_rx,
        livepix_cmd_tx,
        mut livepix_status_rx,
        overlays_cmd_tx,
        mut overlays_status_rx,
        discord_cmd_tx,
        mut discord_status_rx,
        mut hyprland_rx,
        mut twitch_event_rx,
    } = channels;

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
        &overlays_cmd_tx,
        &mut overlays_status_rx,
        &discord_cmd_tx,
        &mut discord_status_rx,
        &mut hyprland_rx,
        &mut twitch_event_rx,
        &mut chat_rx,
        &mut now_playing_rx,
    )
    .await;

    // Clean shutdown of the toggleable Outputs/Inputs.
    let _ = privacy_cmd_tx.send(PrivacyCommand::Stop).await;
    let _ = livepix_cmd_tx.send(LivepixCommand::Stop).await;
    let _ = overlays_cmd_tx.send(OverlayCommand::Stop).await;
    let _ = discord_cmd_tx.send(DiscordCommand::Stop).await;

    if app.waybar_enabled
        && let Err(e) = waybar::disable().await
    {
        tracing::warn!("failed to disable waybar stream bar on exit: {e}");
    }

    restore_terminal(&mut terminal)?;
    result
}

#[allow(clippy::too_many_arguments)]
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
    overlays_cmd_tx: &mpsc::Sender<OverlayCommand>,
    overlays_status_rx: &mut mpsc::Receiver<OverlayStatus>,
    discord_cmd_tx: &mpsc::Sender<DiscordCommand>,
    discord_status_rx: &mut mpsc::Receiver<DiscordStatus>,
    hyprland_rx: &mut mpsc::Receiver<AppEvent>,
    twitch_event_rx: &mut mpsc::Receiver<AppEvent>,
    chat_rx: &mut broadcast::Receiver<ChatSignal>,
    now_playing_rx: &mut watch::Receiver<Option<NowPlaying>>,
) -> io::Result<()> {
    let mut prev = drain::ToggleSnapshot::capture(app);

    loop {
        // Draw
        terminal.draw(|frame| panes::draw(frame, app, tui))?;

        // Poll for key input with a short timeout so we keep draining channels.
        if event::poll(TICK)?
            && let Event::Key(key) = event::read()?
            && input::handle_key(key, app, tui, cmd_tx).await
        {
            return Ok(());
        }

        // Per-source drain handlers — each owns one channel.
        drain::stream_events(app, tui, event_rx);
        drain::privacy_status(app, tui, privacy_status_rx);
        drain::livepix_status(app, tui, livepix_status_rx);
        drain::overlays_status(app, tui, overlays_status_rx);
        drain::discord_status(app, tui, discord_status_rx);
        drain::hyprland_events(app, tui, hyprland_rx);
        drain::twitch_events(app, tui, twitch_event_rx);
        drain::chat_signals(app, tui, chat_rx);
        drain::now_playing(tui, now_playing_rx);

        // Apply queued feature commands, then react to any toggle changes.
        app.process_pending_commands();
        prev = drain::react_to_toggles(
            app,
            tui,
            &prev,
            waybar_output,
            privacy_cmd_tx,
            livepix_cmd_tx,
            overlays_cmd_tx,
            discord_cmd_tx,
        )
        .await;
    }
}

pub(super) fn waybar_error_message(err: &io::Error) -> String {
    if err.kind() == io::ErrorKind::NotFound {
        "waybar config not found — is Omarchy's waybar installed?".into()
    } else {
        format!("waybar error: {err}")
    }
}
