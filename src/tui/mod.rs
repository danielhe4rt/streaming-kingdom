mod input;
mod ui;

use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::event::{self, Event};
use ratatui::prelude::*;
use ratatui::Terminal;
use tokio::sync::{broadcast, mpsc};

use crate::app::{AppState, FeatureCommand, StreamEvent};

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
}

impl TuiState {
    fn new() -> Self {
        Self {
            focused_pane: Pane::Toggles,
            toggle_cursor: 0,
            event_log_scroll: 0,
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

pub async fn run(app: &mut AppState) -> io::Result<()> {
    let mut terminal = init_terminal()?;
    let mut tui = TuiState::new();
    let mut event_rx = app.subscribe_events();
    let cmd_tx = app.command_sender();

    let result = event_loop(&mut terminal, app, &mut tui, &mut event_rx, &cmd_tx).await;

    restore_terminal(&mut terminal)?;
    result
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut AppState,
    tui: &mut TuiState,
    event_rx: &mut broadcast::Receiver<StreamEvent>,
    cmd_tx: &mpsc::Sender<FeatureCommand>,
) -> io::Result<()> {
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

        // Process any pending feature commands.
        app.process_pending_commands();
    }
}
