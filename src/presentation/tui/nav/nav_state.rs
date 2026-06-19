//! The whole nav state and the pure transitions over it.

use super::{default_sub, sub_items, Section, SubItem};

/// The whole nav state: which section is active and the *remembered* sub-item
/// per section, plus the sidebar cursor. Switching sections restores that
/// section's last sub-item rather than resetting — a small UX nicety, and it
/// keeps section/sub-nav selection independent (a pure product of transitions).
#[derive(Debug, Clone)]
pub struct NavState {
    pub section: Section,
    sub_dashboard: SubItem,
    sub_services: SubItem,
    sub_overlays: SubItem,
    sub_activity: SubItem,
}

impl Default for NavState {
    fn default() -> Self {
        Self {
            section: Section::Overlays,
            sub_dashboard: default_sub(Section::Dashboard),
            sub_services: default_sub(Section::Services),
            sub_overlays: default_sub(Section::Overlays),
            sub_activity: default_sub(Section::Activity),
        }
    }
}

impl NavState {
    pub fn new() -> Self {
        Self::default()
    }

    /// The currently-selected sub-item for the active section.
    pub fn current_sub(&self) -> SubItem {
        match self.section {
            Section::Dashboard => self.sub_dashboard,
            Section::Services => self.sub_services,
            Section::Overlays => self.sub_overlays,
            Section::Activity => self.sub_activity,
        }
    }

    fn set_current_sub(&mut self, item: SubItem) {
        match self.section {
            Section::Dashboard => self.sub_dashboard = item,
            Section::Services => self.sub_services = item,
            Section::Overlays => self.sub_overlays = item,
            Section::Activity => self.sub_activity = item,
        }
    }

    /// Pure transition: move to a section (remembers each section's sub-item).
    pub fn select_section(&mut self, section: Section) {
        self.section = section;
    }

    /// Pure transition: next/prev section along the topbar.
    pub fn next_section(&mut self) {
        self.section = self.section.next();
    }
    pub fn prev_section(&mut self) {
        self.section = self.section.prev();
    }

    /// Pure transition: move the sidebar cursor down/up within the active
    /// section's sub-nav, clamped to the list bounds.
    pub fn next_sub(&mut self) {
        let items = sub_items(self.section);
        let cur = self.current_sub();
        let idx = items.iter().position(|i| *i == cur).unwrap_or(0);
        let next = (idx + 1).min(items.len() - 1);
        self.set_current_sub(items[next]);
    }

    pub fn prev_sub(&mut self) {
        let items = sub_items(self.section);
        let cur = self.current_sub();
        let idx = items.iter().position(|i| *i == cur).unwrap_or(0);
        let prev = idx.saturating_sub(1);
        self.set_current_sub(items[prev]);
    }
}

#[cfg(test)]
mod tests {
    use super::super::OverlayId;
    use super::*;

    #[test]
    fn switching_section_remembers_each_subnav() {
        let mut nav = NavState::new();
        nav.select_section(Section::Overlays);
        nav.next_sub(); // Overlays: AllOverlays -> Coworking overlay
        assert_eq!(nav.current_sub(), SubItem::Overlay(OverlayId::Coworking));

        // Go to Activity, move its cursor.
        nav.select_section(Section::Activity);
        assert_eq!(nav.current_sub(), SubItem::Chat);
        nav.next_sub();
        assert_eq!(nav.current_sub(), SubItem::Events);

        // Returning to Overlays restores its remembered sub-item.
        nav.select_section(Section::Overlays);
        assert_eq!(nav.current_sub(), SubItem::Overlay(OverlayId::Coworking));
    }

    #[test]
    fn sub_nav_cursor_clamps_at_bounds() {
        let mut nav = NavState::new();
        nav.select_section(Section::Overlays);
        // Walk past the end — should clamp at the last sub-item (Test events).
        for _ in 0..10 {
            nav.next_sub();
        }
        assert_eq!(nav.current_sub(), SubItem::TestEvents);
        // Walk past the start — should clamp at AllOverlays.
        for _ in 0..10 {
            nav.prev_sub();
        }
        assert_eq!(nav.current_sub(), SubItem::AllOverlays);
    }
}
