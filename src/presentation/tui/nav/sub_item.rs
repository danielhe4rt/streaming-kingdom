//! The sidebar sub-nav items and the registry-derived lists/labels per section.

use crate::presentation::tui::service::{self, ServiceId};

use super::{OverlayId, Section};

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
