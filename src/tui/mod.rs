mod input;
mod ui;

use std::io::{self, Stdout};
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{self, Event};
use ratatui::prelude::*;
use ratatui::Terminal;
use tokio::process::Child;
use tokio::sync::{broadcast, mpsc};

use crate::app::{AppState, FeatureCommand, StreamEvent};
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
}

impl TuiState {
    fn new() -> Self {
        Self {
            focused_pane: Pane::Toggles,
            toggle_cursor: 0,
            event_log_scroll: 0,
            privacy_status: None,
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
    wb_config: &PathBuf,
    wb_style: &PathBuf,
    privacy_cmd_tx: mpsc::Sender<PrivacyCommand>,
    privacy_status_rx: mpsc::Receiver<PrivacyStatus>,
) -> io::Result<()> {
    let mut terminal = init_terminal()?;
    let mut tui = TuiState::new();
    let mut event_rx = app.subscribe_events();
    let cmd_tx = app.command_sender();

    // Spawn waybar if it starts enabled
    let mut wb_child: Option<Child> = if app.waybar_enabled {
        match waybar::spawn(wb_config, wb_style) {
            Ok(child) => Some(child),
            Err(e) => {
                app.status_message = Some(waybar_error_message(&e));
                app.waybar_enabled = false;
                tracing::warn!("failed to spawn waybar: {e}");
                None
            }
        }
    } else {
        None
    };

    let mut privacy_status_rx = privacy_status_rx;

    let result = event_loop(
        &mut terminal,
        app,
        &mut tui,
        &mut event_rx,
        &cmd_tx,
        &mut wb_child,
        wb_config,
        wb_style,
        &privacy_cmd_tx,
        &mut privacy_status_rx,
    )
    .await;

    // Send Stop to privacy monitor for clean shutdown
    let _ = privacy_cmd_tx.send(PrivacyCommand::Stop).await;

    // Clean up waybar on exit
    if let Some(child) = &mut wb_child {
        waybar::kill(child).await;
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
    wb_child: &mut Option<Child>,
    wb_config: &PathBuf,
    wb_style: &PathBuf,
    privacy_cmd_tx: &mpsc::Sender<PrivacyCommand>,
    privacy_status_rx: &mut mpsc::Receiver<PrivacyStatus>,
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
        }

        // Drain privacy status updates, keeping only the latest.
        while let Ok(status) = privacy_status_rx.try_recv() {
            tui.privacy_status = Some(match &status {
                PrivacyStatus::Running => "running".into(),
                PrivacyStatus::Stopped => "stopped".into(),
                PrivacyStatus::BlurEnabled { title } => format!("blur ON: {title}"),
                PrivacyStatus::BlurDisabled => "blur off".into(),
                PrivacyStatus::Error(msg) => format!("error: {msg}"),
            });
        }

        // Process any pending feature commands.
        app.process_pending_commands();

        // React to waybar toggle changes
        if app.waybar_enabled != prev_waybar_enabled {
            if app.waybar_enabled {
                // Spawn waybar
                if wb_child.is_none() {
                    match waybar::spawn(wb_config, wb_style) {
                        Ok(child) => {
                            *wb_child = Some(child);
                            app.status_message = None;
                        }
                        Err(e) => {
                            app.status_message = Some(waybar_error_message(&e));
                            app.waybar_enabled = false;
                            tracing::warn!("failed to spawn waybar: {e}");
                        }
                    }
                }
            } else {
                // Kill waybar
                if let Some(child) = wb_child {
                    waybar::kill(child).await;
                    *wb_child = None;
                }
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
        "waybar not found — install waybar to use the bottom bar".into()
    } else {
        format!("failed to start waybar: {err}")
    }
}
