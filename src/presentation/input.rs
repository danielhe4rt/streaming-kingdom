use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use tokio::sync::mpsc;

use crate::application::AppState;
use crate::domain::FeatureCommand;
use super::state::{Pane, TuiState};
use super::ui::INTEGRATION_COUNT;

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

        // Context-dependent j/Down
        KeyCode::Char('j') | KeyCode::Down => match tui.focused_pane {
            Pane::Integrations => {
                tui.integration_cursor = (tui.integration_cursor + 1).min(INTEGRATION_COUNT - 1);
            }
            Pane::EventLog => {
                tui.event_log_scroll = tui.event_log_scroll.saturating_add(1);
            }
            Pane::Chat => {
                tui.chat_scroll = tui.chat_scroll.saturating_add(1);
            }
            Pane::Stats => {}
        },

        // Context-dependent k/Up
        KeyCode::Char('k') | KeyCode::Up => match tui.focused_pane {
            Pane::Integrations => {
                tui.integration_cursor = tui.integration_cursor.saturating_sub(1);
            }
            Pane::EventLog => {
                tui.event_log_scroll = tui.event_log_scroll.saturating_sub(1);
            }
            Pane::Chat => {
                tui.chat_scroll = tui.chat_scroll.saturating_sub(1);
            }
            Pane::Stats => {}
        },

        // Scroll to bottom
        KeyCode::Char('G') | KeyCode::End => match tui.focused_pane {
            Pane::Chat => {
                tui.chat_scroll = 0;
            }
            Pane::EventLog => {
                tui.event_log_scroll = 0;
            }
            _ => {}
        },

        // Toggle selected feature (only in Integrations pane)
        KeyCode::Char(' ') | KeyCode::Enter => {
            if tui.focused_pane == Pane::Integrations {
                if let Some(cmd) = toggle_command(tui.integration_cursor, app) {
                    let _ = cmd_tx.send(cmd).await;
                }
            }
        }

        // Switch focus between panes
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                tui.focused_pane = tui.focused_pane.prev();
            } else {
                tui.focused_pane = tui.focused_pane.next();
            }
        }
        KeyCode::BackTab => {
            tui.focused_pane = tui.focused_pane.prev();
        }

        // Event log group filter toggles (1-6)
        KeyCode::Char('1') => {
            tui.filter_stream = !tui.filter_stream;
        }
        KeyCode::Char('2') => {
            tui.filter_privacy = !tui.filter_privacy;
        }
        KeyCode::Char('3') => {
            tui.filter_system = !tui.filter_system;
        }
        KeyCode::Char('4') => {
            tui.filter_hyprland = !tui.filter_hyprland;
        }
        KeyCode::Char('5') => {
            tui.filter_livepix = !tui.filter_livepix;
        }
        KeyCode::Char('6') => {
            tui.filter_chat = !tui.filter_chat;
        }

        _ => {}
    }

    false
}

/// Map integration cursor position to a feature toggle command.
/// Cursor positions 0-1 are read-only (EventSub, Chat); 2-5 are toggleable.
fn toggle_command(cursor: usize, app: &AppState) -> Option<FeatureCommand> {
    match cursor {
        // 2: Livepix
        2 => Some(if app.livepix_enabled {
            FeatureCommand::DisableLivepix
        } else {
            FeatureCommand::EnableLivepix
        }),
        // 3: Privacy
        3 => Some(if app.privacy_enabled {
            FeatureCommand::DisablePrivacy
        } else {
            FeatureCommand::EnablePrivacy
        }),
        // 4: Waybar
        4 => Some(if app.waybar_enabled {
            FeatureCommand::DisableWaybar
        } else {
            FeatureCommand::EnableWaybar
        }),
        // 5: Alerts (Hyprland card doubles as alerts toggle)
        5 => Some(if app.alerts_enabled {
            FeatureCommand::DisableAlerts
        } else {
            FeatureCommand::EnableAlerts
        }),
        _ => None,
    }
}
