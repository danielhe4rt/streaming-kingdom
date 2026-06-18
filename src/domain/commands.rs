// ---------------------------------------------------------------------------
// Feature commands – TUI sends these to toggle/control feature modules
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeatureCommand {
    EnableWaybar,
    DisableWaybar,
    EnablePrivacy,
    DisablePrivacy,
    EnableAlerts,
    DisableAlerts,
    EnableLivepix,
    DisableLivepix,
    EnableOverlays,
    DisableOverlays,
}
