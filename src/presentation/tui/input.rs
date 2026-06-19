//! Keyboard input for the nav shell. Navigation drives the pure [`NavState`]
//! transitions; toggling resolves the *selected* Service to a `FeatureCommand`
//! via the registry — no magic-number cursor mapping.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use tokio::sync::mpsc;

use super::nav::{Section, SubItem};
use super::service::{self, ServiceId};
use super::state::TuiState;
use crate::application::AppState;
use crate::domain::{ChatMessageDeleted, ChatSignal, FeatureCommand};
use crate::presentation::synthetic::{self, SyntheticKind};

/// Handle a key event. Returns `true` when the app should quit.
pub async fn handle_key(
    key: KeyEvent,
    app: &AppState,
    tui: &mut TuiState,
    cmd_tx: &mpsc::Sender<FeatureCommand>,
) -> bool {
    if key.kind != KeyEventKind::Press {
        return false;
    }

    match key.code {
        KeyCode::Char('q') => return true,

        // Section nav along the topbar.
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                tui.nav.prev_section();
            } else {
                tui.nav.next_section();
            }
        }
        KeyCode::BackTab => tui.nav.prev_section(),

        // Jump directly to a section by number row (topbar order).
        KeyCode::Char('!') => tui.nav.select_section(Section::Dashboard),
        KeyCode::Char('@') => tui.nav.select_section(Section::Services),
        KeyCode::Char('#') => tui.nav.select_section(Section::Overlays),
        KeyCode::Char('$') => tui.nav.select_section(Section::Activity),

        // Sub-nav (sidebar) with j/k or arrows; content scrolling with J/K.
        KeyCode::Char('j') | KeyCode::Down => tui.nav.next_sub(),
        KeyCode::Char('k') | KeyCode::Up => tui.nav.prev_sub(),

        KeyCode::Char('J') => scroll_down(tui),
        KeyCode::Char('K') => scroll_up(tui),

        KeyCode::Char('G') | KeyCode::End => reset_scroll(tui),

        // On the Test events pane, Enter fires the selected Synthetic Event
        // (instead of toggling the Overlays server).
        KeyCode::Enter if is_test_events(tui) => fire_test_event(app, tui),

        // Toggle the Service the sub-nav currently selects.
        KeyCode::Char(' ') | KeyCode::Enter => {
            if let Some(id) = selected_service(tui)
                && let Some(cmd) = service::toggle_command(id, app)
            {
                let _ = cmd_tx.send(cmd).await;
            }
        }

        // Event-log group filters (1-6) — preserved.
        KeyCode::Char('1') => tui.filter_stream = !tui.filter_stream,
        KeyCode::Char('2') => tui.filter_privacy = !tui.filter_privacy,
        KeyCode::Char('3') => tui.filter_system = !tui.filter_system,
        KeyCode::Char('4') => tui.filter_hyprland = !tui.filter_hyprland,
        KeyCode::Char('5') => tui.filter_livepix = !tui.filter_livepix,
        KeyCode::Char('6') => tui.filter_chat = !tui.filter_chat,

        _ => {}
    }

    false
}

/// Which Service the current section + sub-nav resolves to a toggle for.
///
/// - In **Services**, the selected sub-item *is* a Service.
/// - In **Overlays**, every sub-item (All / each Overlay / Feed) toggles the
///   one Overlays Output — so the server start/stop is reachable from anywhere
///   in the section.
/// - Other sections have nothing toggleable.
fn selected_service(tui: &TuiState) -> Option<ServiceId> {
    match tui.nav.section {
        Section::Services => match tui.nav.current_sub() {
            SubItem::Service(id) => Some(id),
            _ => None,
        },
        // Every Overlays sub-item toggles the one server — except Test events,
        // where Enter fires a Synthetic Event instead.
        Section::Overlays => match tui.nav.current_sub() {
            SubItem::TestEvents => None,
            _ => Some(ServiceId::Overlays),
        },
        _ => None,
    }
}

/// Whether the Test events pane is active (Enter fires; J/K moves the cursor).
fn is_test_events(tui: &TuiState) -> bool {
    tui.nav.section == Section::Overlays && tui.nav.current_sub() == SubItem::TestEvents
}

/// Fire the Synthetic Event the Test events cursor selects, onto the *real*
/// channels (ADR-0002) so the Overlay and the TUI react identically to a real
/// one. All senders take `&self`, so an immutable `&AppState` is enough.
fn fire_test_event(app: &AppState, tui: &mut TuiState) {
    let Some(kind) = SyntheticKind::ALL.get(tui.test_event_cursor).copied() else {
        return;
    };
    match kind {
        SyntheticKind::DeleteLast => {
            if let Some(id) = tui.last_test_msg_id.clone() {
                let _ = app
                    .chat_tx
                    .send(ChatSignal::Deleted(ChatMessageDeleted::new(id)));
            }
        }
        SyntheticKind::Chat => {
            let msg = synthetic::chat_message(synthetic::next_seq());
            tui.last_test_msg_id = Some(msg.msg_id.clone());
            let _ = app.chat_tx.send(ChatSignal::Message(msg));
        }
        SyntheticKind::NowPlaying => {
            let _ = app
                .now_playing_tx
                .send(Some(synthetic::now_playing(synthetic::next_seq())));
        }
        SyntheticKind::VoiceRoster => {
            let _ = app
                .voice_roster_tx
                .send(Some(synthetic::voice_roster(synthetic::next_seq())));
        }
        other => {
            if let Some(event) = synthetic::stream_event(other, synthetic::next_seq()) {
                let _ = app.event_tx.send(event);
            }
        }
    }
}

fn scroll_down(tui: &mut TuiState) {
    match (tui.nav.section, tui.nav.current_sub()) {
        (Section::Activity, SubItem::Events) => {
            tui.event_log_scroll = tui.event_log_scroll.saturating_add(1);
        }
        (Section::Activity, SubItem::Chat) | (Section::Overlays, SubItem::Feed) => {
            tui.chat_scroll = tui.chat_scroll.saturating_add(1);
        }
        // On the Test events pane J/K moves the event-type cursor (wraps).
        (Section::Overlays, SubItem::TestEvents) => {
            let len = SyntheticKind::ALL.len();
            tui.test_event_cursor = (tui.test_event_cursor + 1) % len;
        }
        _ => {}
    }
}

fn scroll_up(tui: &mut TuiState) {
    match (tui.nav.section, tui.nav.current_sub()) {
        (Section::Activity, SubItem::Events) => {
            tui.event_log_scroll = tui.event_log_scroll.saturating_sub(1);
        }
        (Section::Activity, SubItem::Chat) | (Section::Overlays, SubItem::Feed) => {
            tui.chat_scroll = tui.chat_scroll.saturating_sub(1);
        }
        (Section::Overlays, SubItem::TestEvents) => {
            let len = SyntheticKind::ALL.len();
            tui.test_event_cursor = (tui.test_event_cursor + len - 1) % len;
        }
        _ => {}
    }
}

fn reset_scroll(tui: &mut TuiState) {
    tui.event_log_scroll = 0;
    tui.chat_scroll = 0;
}
