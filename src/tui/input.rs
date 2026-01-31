use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use tokio::sync::mpsc;

use crate::app::{AppState, FeatureCommand};
use super::TuiState;

const TOGGLE_COUNT: usize = 3;

/// Handle a key event. Returns `true` when the app should quit.
pub async fn handle_key(
    key: KeyEvent,
    app: &AppState,
    tui: &mut TuiState,
    cmd_tx: &mpsc::Sender<FeatureCommand>,
) -> bool {
    // Only act on key-press, ignore release / repeat
    if key.kind != KeyEventKind::Press {
        return false;
    }

    match key.code {
        // Quit
        KeyCode::Char('q') => return true,

        // Navigate toggle list
        KeyCode::Char('j') | KeyCode::Down => {
            tui.toggle_cursor = (tui.toggle_cursor + 1).min(TOGGLE_COUNT - 1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            tui.toggle_cursor = tui.toggle_cursor.saturating_sub(1);
        }

        // Toggle selected feature
        KeyCode::Char(' ') | KeyCode::Enter => {
            if let Some(cmd) = toggle_command(tui.toggle_cursor, app) {
                let _ = cmd_tx.send(cmd).await;
            }
        }

        // Switch focus between panes
        KeyCode::Tab => {
            tui.focused_pane = tui.focused_pane.next();
        }

        _ => {}
    }

    false
}

fn toggle_command(cursor: usize, app: &AppState) -> Option<FeatureCommand> {
    match cursor {
        0 => Some(if app.waybar_enabled {
            FeatureCommand::DisableWaybar
        } else {
            FeatureCommand::EnableWaybar
        }),
        1 => Some(if app.privacy_enabled {
            FeatureCommand::DisablePrivacy
        } else {
            FeatureCommand::EnablePrivacy
        }),
        2 => Some(if app.alerts_enabled {
            FeatureCommand::DisableAlerts
        } else {
            FeatureCommand::EnableAlerts
        }),
        _ => None,
    }
}
