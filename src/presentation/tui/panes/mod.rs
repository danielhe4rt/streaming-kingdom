//! Per-pane render modules for the nav shell. `draw` lays out the topbar →
//! sidebar → content shell (variant D) and dispatches the content area to the
//! pane for the active section + sub-nav.

pub mod activity;
pub mod dashboard;
pub mod format;
pub mod overlays;
pub mod services;
pub mod shell;

use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Padding};

use super::nav::{Section, SubItem};
use super::state::TuiState;
use super::theme::{COLOR_ACCENT, COLOR_PRIMARY};
use crate::application::AppState;

/// Main draw entry point: an outer bordered frame wrapping
/// topbar (top) → [sidebar | content] → status bar.
pub fn draw(frame: &mut Frame, app: &AppState, tui: &TuiState) {
    // Outer border + padding around the whole UI — the signature purple frame.
    let outer = Block::default()
        .title(Line::from(Span::styled(
            " ♥ streams-toolkit ",
            Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD),
        )))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(COLOR_PRIMARY))
        .padding(Padding::horizontal(1));
    let root = outer.inner(frame.area());
    frame.render_widget(outer, frame.area());

    let [topbar_area, main_area, status_area] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(root);

    shell::draw_topbar(frame, topbar_area, app, tui);

    let [sidebar_area, content_area] =
        Layout::horizontal([Constraint::Length(24), Constraint::Min(0)]).areas(main_area);

    shell::draw_sidebar(frame, sidebar_area, app, tui);
    draw_content(frame, content_area, app, tui);

    shell::draw_status_bar(frame, status_area, app);
}

fn draw_content(frame: &mut Frame, area: Rect, app: &AppState, tui: &TuiState) {
    let sub = tui.nav.current_sub();
    match tui.nav.section {
        Section::Dashboard => dashboard::draw(frame, area, app, tui),
        Section::Services => match sub {
            SubItem::Service(id) => services::draw_detail(frame, area, id, app, tui),
            _ => services::draw_all(frame, area, app, tui),
        },
        Section::Overlays => match sub {
            SubItem::Overlay(id) => overlays::draw_detail(frame, area, id, app, tui),
            SubItem::Feed => overlays::draw_feed(frame, area, tui),
            SubItem::TestEvents => overlays::draw_test_events(frame, area, tui),
            _ => overlays::draw_all(frame, area, app, tui),
        },
        Section::Activity => match sub {
            SubItem::Events => activity::draw_event_log(frame, area, app, tui),
            SubItem::Highlights => activity::draw_highlights(frame, area, tui),
            _ => activity::draw_chat(frame, area, tui),
        },
    }
}
