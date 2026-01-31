mod input;
mod ui;

use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::event::{self, Event};
use ratatui::prelude::*;
use ratatui::Terminal;
use tokio::sync::{broadcast, mpsc};

use crate::app::{AppEvent, AppState, FeatureCommand, StreamEvent};
use crate::config::EventLogConfig;
use crate::privacy::{PrivacyCommand, PrivacyStatus};
use crate::waybar;

// ---------------------------------------------------------------------------
// TUI state that lives alongside (not inside) AppState
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Toggles,
    Stats,
    EventLog,
}

impl Pane {
    fn next(self) -> Self {
        match self {
            Pane::Toggles => Pane::Stats,
            Pane::Stats => Pane::EventLog,
            Pane::EventLog => Pane::Toggles,
        }
    }
}

pub struct TuiState {
    pub focused_pane: Pane,
    pub toggle_cursor: usize,
    pub event_log_scroll: u16,
    pub privacy_status: Option<String>,
    pub livepix_status: Option<String>,

    // Event log group filters (true = visible)
    pub filter_stream: bool,
    pub filter_privacy: bool,
    pub filter_system: bool,
    pub filter_hyprland: bool,
}

impl TuiState {
    fn new(event_log_config: &EventLogConfig) -> Self {
        Self {
            focused_pane: Pane::Toggles,
            toggle_cursor: 0,
            event_log_scroll: 0,
            privacy_status: None,
            livepix_status: None,
            filter_stream: event_log_config.show_stream,
            filter_privacy: event_log_config.show_privacy,
            filter_system: event_log_config.show_system,
            filter_hyprland: event_log_config.show_hyprland,
        }
    }
}

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
    crossterm::execute!(terminal.backend_mut(), crossterm::terminal::LeaveAlternateScreen)?;
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
    event_log_config: &EventLogConfig,
    mut hyprland_rx: mpsc::Receiver<AppEvent>,
) -> io::Result<()> {
    let mut terminal = init_terminal()?;
    let mut tui = TuiState::new(event_log_config);
    let mut event_rx = app.subscribe_events();
    let cmd_tx = app.command_sender();

    // Enable the stream bar if waybar starts enabled
    if app.waybar_enabled {
        if let Err(e) = waybar::enable(waybar_output).await {
            app.status_message = Some(waybar_error_message(&e));
            app.waybar_enabled = false;
            tracing::warn!("failed to enable waybar stream bar: {e}");
        }
    }

    let mut privacy_status_rx = privacy_status_rx;

    let result = event_loop(
        &mut terminal,
        app,
        &mut tui,
        &mut event_rx,
        &cmd_tx,
        waybar_output,
        &privacy_cmd_tx,
        &mut privacy_status_rx,
        &mut hyprland_rx,
    )
    .await;

    // Send Stop to privacy monitor for clean shutdown
    let _ = privacy_cmd_tx.send(PrivacyCommand::Stop).await;

    // Clean up stream bar from waybar config on exit
    if app.waybar_enabled {
        if let Err(e) = waybar::disable().await {
            tracing::warn!("failed to disable waybar stream bar on exit: {e}");
        }
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
    hyprland_rx: &mut mpsc::Receiver<AppEvent>,
) -> io::Result<()> {
    let mut prev_waybar_enabled = app.waybar_enabled;
    let mut prev_privacy_enabled = app.privacy_enabled;

    loop {
        // Draw
        terminal.draw(|frame| ui::draw(frame, app, tui))?;

        // Poll for crossterm events with a short timeout so we can also
        // drain stream events from the broadcast channel.
        if event::poll(TICK)? {
            if let Event::Key(key) = event::read()? {
                if input::handle_key(key, app, tui, cmd_tx).await {
                    return Ok(());
                }
            }
        }

        // Drain any pending stream events into stats / event log.
        while let Ok(ev) = event_rx.try_recv() {
            app.stats.record(&ev);
            app.log_event(AppEvent::Stream(ev));
        }

        // Drain privacy status updates — log each and keep latest for display.
        while let Ok(status) = privacy_status_rx.try_recv() {
            tui.privacy_status = Some(match &status {
                PrivacyStatus::Running => "running".into(),
                PrivacyStatus::Stopped => "stopped".into(),
                PrivacyStatus::BlurEnabled { title } => format!("blur ON: {title}"),
                PrivacyStatus::BlurDisabled => "blur off".into(),
                PrivacyStatus::Error(msg) => format!("error: {msg}"),
            });

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

        // Drain Hyprland events into the event log.
        while let Ok(ev) = hyprland_rx.try_recv() {
            app.log_event(ev);
        }

        // Process any pending feature commands.
        app.process_pending_commands();

        // React to waybar toggle changes
        if app.waybar_enabled != prev_waybar_enabled {
            if app.waybar_enabled {
                // Enable: merge config + style, restart waybar
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
                // Disable: remove config + style, restart waybar
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
                tui.privacy_status = None;
                PrivacyCommand::Stop
            };
            let _ = privacy_cmd_tx.send(cmd).await;
            prev_privacy_enabled = app.privacy_enabled;
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
