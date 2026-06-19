//! M4 — data-driven **Service** registry.
//!
//! A Service is one integration row, classified **Input** (feeds the toolkit,
//! read-only/monitor) or **Output** (driven by the toolkit, toggleable). This
//! replaces the old hardcoded six cards and the magic-number `cursor → toggle`
//! mapping in `input.rs`: adding a Service is a single entry in [`registry`],
//! and toggle resolution is a pure function of the Service id + current state.
//!
//! Nothing here does I/O — it is a pure description of *what* the TUI shows and
//! *which* `FeatureCommand` a toggle resolves to, so it is unit-testable in
//! isolation (mirroring `privacy_monitor::is_sensitive`).

use crate::application::AppState;
use crate::domain::FeatureCommand;

/// Which side of the toolkit a Service sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceKind {
    /// Feeds the toolkit (EventSub, Chat, Livepix, Hyprland). Read-only monitor.
    Input,
    /// Driven by the toolkit (Waybar, Overlays). Toggleable on/off.
    Output,
}

/// A stable identifier for each Service. Adding a Service = one variant here
/// plus one [`ServiceDef`] entry in [`registry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceId {
    TwitchEventSub,
    TwitchChat,
    Spotify,
    Livepix,
    Hyprland,
    Privacy,
    Waybar,
    Overlays,
    Discord,
}

/// The static description of a Service — its identity and classification. The
/// live status (connected, running, counters) is read from `TuiState` /
/// `AppState` at render time, never stored here.
#[derive(Debug, Clone, Copy)]
pub struct ServiceDef {
    pub id: ServiceId,
    pub name: &'static str,
    pub kind: ServiceKind,
    /// `true` for Outputs the TUI can Start/Stop; `false` for read-only Inputs.
    pub toggleable: bool,
}

/// The complete Service registry, in display order. Inputs first, then Outputs.
/// This single list is the source of truth for both the sidebar sub-nav and the
/// grouped Services pane — no layout code knows a fixed count.
pub const REGISTRY: &[ServiceDef] = &[
    ServiceDef {
        id: ServiceId::TwitchEventSub,
        name: "Twitch EventSub",
        kind: ServiceKind::Input,
        toggleable: false,
    },
    ServiceDef {
        id: ServiceId::TwitchChat,
        name: "Twitch Chat",
        kind: ServiceKind::Input,
        toggleable: false,
    },
    ServiceDef {
        // Now-playing observer (MPRIS / playerctl). Read-only ambient state —
        // its live track is shown in the detail pane, never toggled here.
        id: ServiceId::Spotify,
        name: "Spotify",
        kind: ServiceKind::Input,
        toggleable: false,
    },
    ServiceDef {
        id: ServiceId::Livepix,
        name: "Livepix",
        kind: ServiceKind::Input,
        toggleable: true,
    },
    ServiceDef {
        id: ServiceId::Hyprland,
        name: "Hyprland",
        kind: ServiceKind::Input,
        toggleable: true,
    },
    ServiceDef {
        id: ServiceId::Privacy,
        name: "Privacy Monitor",
        kind: ServiceKind::Input,
        toggleable: true,
    },
    ServiceDef {
        id: ServiceId::Waybar,
        name: "Waybar",
        kind: ServiceKind::Output,
        toggleable: true,
    },
    ServiceDef {
        id: ServiceId::Overlays,
        name: "Overlays",
        kind: ServiceKind::Output,
        toggleable: true,
    },
    ServiceDef {
        id: ServiceId::Discord,
        name: "Discord",
        kind: ServiceKind::Output,
        toggleable: true,
    },
];

/// The Service registry as a slice — the data-driven source for nav + panes.
pub fn registry() -> &'static [ServiceDef] {
    REGISTRY
}

/// Inputs, in registry order.
pub fn inputs() -> impl Iterator<Item = &'static ServiceDef> {
    REGISTRY.iter().filter(|s| s.kind == ServiceKind::Input)
}

/// Outputs, in registry order.
pub fn outputs() -> impl Iterator<Item = &'static ServiceDef> {
    REGISTRY.iter().filter(|s| s.kind == ServiceKind::Output)
}

/// Whether a Service is currently enabled, read from the central toggles.
/// Inputs that aren't toggleable report their "feed is wired" intent (Livepix,
/// Hyprland map to their enable flags; the read-only Twitch services are always
/// considered "on" once configured — their live status is shown separately).
pub fn is_enabled(id: ServiceId, app: &AppState) -> bool {
    match id {
        ServiceId::TwitchEventSub | ServiceId::TwitchChat | ServiceId::Spotify => true,
        ServiceId::Livepix => app.livepix_enabled,
        ServiceId::Hyprland => app.alerts_enabled,
        ServiceId::Privacy => app.privacy_enabled,
        ServiceId::Waybar => app.waybar_enabled,
        ServiceId::Overlays => app.overlays_enabled,
        ServiceId::Discord => app.discord_enabled,
    }
}

/// Pure transition: resolve a toggle of the given Service into the
/// `FeatureCommand` that flips it, based on the current enabled state.
/// Returns `None` for read-only Services (no command). No magic numbers — the
/// id selects the command, the current state selects the direction.
pub fn toggle_command(id: ServiceId, app: &AppState) -> Option<FeatureCommand> {
    match id {
        ServiceId::Livepix => Some(if app.livepix_enabled {
            FeatureCommand::DisableLivepix
        } else {
            FeatureCommand::EnableLivepix
        }),
        ServiceId::Hyprland => Some(if app.alerts_enabled {
            FeatureCommand::DisableAlerts
        } else {
            FeatureCommand::EnableAlerts
        }),
        ServiceId::Privacy => Some(if app.privacy_enabled {
            FeatureCommand::DisablePrivacy
        } else {
            FeatureCommand::EnablePrivacy
        }),
        ServiceId::Waybar => Some(if app.waybar_enabled {
            FeatureCommand::DisableWaybar
        } else {
            FeatureCommand::EnableWaybar
        }),
        ServiceId::Overlays => Some(if app.overlays_enabled {
            FeatureCommand::DisableOverlays
        } else {
            FeatureCommand::EnableOverlays
        }),
        ServiceId::Discord => Some(if app.discord_enabled {
            FeatureCommand::DisableDiscord
        } else {
            FeatureCommand::EnableDiscord
        }),
        // Read-only Inputs have no toggle.
        ServiceId::TwitchEventSub | ServiceId::TwitchChat | ServiceId::Spotify => None,
    }
}

// ---------------------------------------------------------------------------
// Tests — external behaviour of the pure registry / toggle resolution.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{AppState, EventLogConfig};

    fn app() -> AppState {
        AppState::new(&EventLogConfig::default())
    }

    #[test]
    fn registry_is_grouped_inputs_then_outputs() {
        assert_eq!(
            inputs().count(),
            6,
            "EventSub, Chat, Spotify, Livepix, Hyprland, Privacy"
        );
        assert_eq!(outputs().count(), 3, "Waybar, Overlays, Discord");

        // Every Input is classified Input, every Output is classified Output.
        assert!(inputs().all(|s| s.kind == ServiceKind::Input));
        assert!(outputs().all(|s| s.kind == ServiceKind::Output));
    }

    #[test]
    fn read_only_inputs_have_no_toggle_command() {
        let app = app();
        assert!(toggle_command(ServiceId::TwitchEventSub, &app).is_none());
        assert!(toggle_command(ServiceId::TwitchChat, &app).is_none());
    }

    #[test]
    fn toggle_resolves_to_enable_when_off() {
        let app = app(); // all outputs default off
        assert_eq!(
            toggle_command(ServiceId::Overlays, &app),
            Some(FeatureCommand::EnableOverlays)
        );
        assert_eq!(
            toggle_command(ServiceId::Waybar, &app),
            Some(FeatureCommand::EnableWaybar)
        );
        assert_eq!(
            toggle_command(ServiceId::Livepix, &app),
            Some(FeatureCommand::EnableLivepix)
        );
    }

    #[test]
    fn toggle_resolves_to_disable_when_on() {
        let mut app = app();
        app.overlays_enabled = true;
        app.waybar_enabled = true;
        app.livepix_enabled = true;
        app.alerts_enabled = true;

        assert_eq!(
            toggle_command(ServiceId::Overlays, &app),
            Some(FeatureCommand::DisableOverlays)
        );
        assert_eq!(
            toggle_command(ServiceId::Waybar, &app),
            Some(FeatureCommand::DisableWaybar)
        );
        assert_eq!(
            toggle_command(ServiceId::Livepix, &app),
            Some(FeatureCommand::DisableLivepix)
        );
        assert_eq!(
            toggle_command(ServiceId::Hyprland, &app),
            Some(FeatureCommand::DisableAlerts)
        );
    }

    #[test]
    fn is_enabled_tracks_central_toggles() {
        let mut app = app();
        assert!(!is_enabled(ServiceId::Overlays, &app));
        app.overlays_enabled = true;
        assert!(is_enabled(ServiceId::Overlays, &app));

        // Read-only Twitch services are always "on".
        assert!(is_enabled(ServiceId::TwitchEventSub, &app));
        assert!(is_enabled(ServiceId::TwitchChat, &app));
    }
}
