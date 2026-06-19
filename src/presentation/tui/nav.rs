//! M4 — the two-level nav model (prototype variant **D**).
//!
//! Topbar picks a **Section**; the sidebar shows that section's contextual
//! **sub-nav**, and the content panel renders the selected sub-item. All
//! transitions here are **pure** functions of the current [`NavState`] — no I/O,
//! no rendering — so they're unit-testable in isolation, and the sub-nav lists
//! are derived from the data-driven Service registry (no fixed counts, no magic
//! numbers).

use super::service::{self, ServiceId};

/// Primary topbar sections (variant D).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Dashboard,
    Services,
    Overlays,
    Activity,
}

impl Section {
    /// The sections in topbar order.
    pub const ALL: [Section; 4] = [
        Section::Dashboard,
        Section::Services,
        Section::Overlays,
        Section::Activity,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Section::Dashboard => "Dashboard",
            Section::Services => "Services",
            Section::Overlays => "Overlays",
            Section::Activity => "Activity",
        }
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|s| *s == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let idx = Self::ALL.iter().position(|s| *s == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// One sub-nav item: a contextual entry in the sidebar for the active section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubItem {
    // Dashboard
    Overview,
    // Services — `All` then one per Service in the registry.
    AllServices,
    Service(ServiceId),
    // Overlays — `All`, one per Overlay, the Feed, then the test-event dispatch.
    AllOverlays,
    Overlay(OverlayId),
    Feed,
    TestEvents,
    // Activity
    Chat,
    Events,
    Highlights,
}

/// The Overlays served by the `http` renderer. Adding an Overlay = one entry in
/// [`OverlayId::ALL`] and its [`OverlayDef`] — no layout code changes (the
/// sidebar and panels iterate this list).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayId {
    Coworking,
}

/// Static description of one Overlay: its display name and OBS browser-source path.
#[derive(Debug, Clone, Copy)]
pub struct OverlayDef {
    pub id: OverlayId,
    pub name: &'static str,
    /// The path appended to `http://127.0.0.1:<port>` for the OBS browser source.
    pub path: &'static str,
}

impl OverlayId {
    pub const ALL: [OverlayDef; 1] = [OverlayDef {
        id: OverlayId::Coworking,
        name: "Coworking",
        path: "/overlay/coworking",
    }];

    pub fn def(self) -> &'static OverlayDef {
        OverlayId::ALL.iter().find(|o| o.id == self).unwrap()
    }
}

/// Build the data-driven OBS browser-source URL for an Overlay at the given port.
pub fn overlay_url(def: &OverlayDef, port: u16) -> String {
    format!("http://127.0.0.1:{port}{}", def.path)
}

/// The sub-nav items for a section, derived from the Service / Overlay registries.
pub fn sub_items(section: Section) -> Vec<SubItem> {
    match section {
        Section::Dashboard => vec![SubItem::Overview],
        Section::Services => {
            let mut v = vec![SubItem::AllServices];
            v.extend(service::registry().iter().map(|s| SubItem::Service(s.id)));
            v
        }
        Section::Overlays => {
            let mut v = vec![SubItem::AllOverlays];
            v.extend(OverlayId::ALL.iter().map(|o| SubItem::Overlay(o.id)));
            v.push(SubItem::Feed);
            v.push(SubItem::TestEvents);
            v
        }
        Section::Activity => vec![SubItem::Chat, SubItem::Events, SubItem::Highlights],
    }
}

/// The default sub-item when a section becomes active.
pub fn default_sub(section: Section) -> SubItem {
    sub_items(section)[0]
}

/// The human label for a sub-item (used in the sidebar).
pub fn sub_label(item: SubItem) -> String {
    match item {
        SubItem::Overview => "Overview".into(),
        SubItem::AllServices => "All services".into(),
        SubItem::Service(id) => service::registry()
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.name.to_string())
            .unwrap_or_default(),
        SubItem::AllOverlays => "All overlays".into(),
        SubItem::Overlay(id) => format!("{} overlay", id.def().name),
        SubItem::Feed => "Feed (SSE)".into(),
        SubItem::TestEvents => "Test events".into(),
        SubItem::Chat => "Live Chat".into(),
        SubItem::Events => "Event Log".into(),
        SubItem::Highlights => "Highlights".into(),
    }
}

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

// ---------------------------------------------------------------------------
// Tests — external behaviour of the pure nav transitions.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_cycles_both_ways() {
        assert_eq!(Section::Dashboard.next(), Section::Services);
        assert_eq!(Section::Activity.next(), Section::Dashboard);
        assert_eq!(Section::Dashboard.prev(), Section::Activity);
    }

    #[test]
    fn overlays_sub_nav_lists_all_coworking_feed() {
        let items = sub_items(Section::Overlays);
        assert_eq!(items[0], SubItem::AllOverlays);
        assert_eq!(items[1], SubItem::Overlay(OverlayId::Coworking));
        assert_eq!(items[2], SubItem::Feed);
        assert_eq!(items[3], SubItem::TestEvents);
        assert_eq!(items.len(), 4);
    }

    #[test]
    fn services_sub_nav_is_derived_from_registry() {
        let items = sub_items(Section::Services);
        // "All services" + one entry per registered Service — no fixed count.
        assert_eq!(items.len(), 1 + service::registry().len());
        assert_eq!(items[0], SubItem::AllServices);
    }

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

    #[test]
    fn overlay_url_is_data_driven() {
        let def = OverlayId::Coworking.def();
        assert_eq!(
            overlay_url(def, 1337),
            "http://127.0.0.1:1337/overlay/coworking"
        );
        assert_eq!(
            overlay_url(def, 8080),
            "http://127.0.0.1:8080/overlay/coworking"
        );
    }
}
