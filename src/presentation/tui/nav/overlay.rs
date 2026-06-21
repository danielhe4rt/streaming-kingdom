//! The Overlay registry and the data-driven OBS browser-source URL.

/// The Overlays served by the `http` renderer. Adding an Overlay = one entry in
/// [`OverlayId::ALL`] and its [`OverlayDef`] — no layout code changes (the
/// sidebar and panels iterate this list).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayId {
    Coworking,
    Voice,
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
    pub const ALL: [OverlayDef; 2] = [
        OverlayDef {
            id: OverlayId::Coworking,
            name: "Coworking",
            path: "/overlay/coworking",
        },
        OverlayDef {
            id: OverlayId::Voice,
            name: "Voice",
            path: "/overlay/voice",
        },
    ];

    pub fn def(self) -> &'static OverlayDef {
        OverlayId::ALL
            .iter()
            .find(|o| o.id == self)
            .expect("every OverlayId has a matching OverlayDef in OverlayId::ALL")
    }
}

/// Build the data-driven OBS browser-source URL for an Overlay at the given port.
pub fn overlay_url(def: &OverlayDef, port: u16) -> String {
    format!("http://127.0.0.1:{port}{}", def.path)
}

#[cfg(test)]
mod tests {
    use super::*;

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
