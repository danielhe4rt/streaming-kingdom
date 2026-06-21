// ---------------------------------------------------------------------------
// Wire contract for the Vencord "VoiceRosterBridge" plugin (source = "bridge").
//
// The plugin reads Discord's Flux voice stores and pushes one JSON snapshot per
// change (camelCase, mirroring the domain VoiceRoster). This module deserializes
// a snapshot and maps it onto the domain type the feed consumes — the bridge's
// counterpart to the RPC adapter's `mapping.rs`, so both sources converge on the
// exact same VoiceRoster.
// ---------------------------------------------------------------------------

use serde::Deserialize;

use crate::domain::{VoiceMember, VoiceRoster};

/// One roster snapshot as the plugin sends it over the WebSocket.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeSnapshot {
    #[serde(default)]
    pub channel_id: Option<String>,
    #[serde(default)]
    pub channel_name: Option<String>,
    #[serde(default)]
    pub members: Vec<BridgeMember>,
}

/// One member within a bridge snapshot. Flags default to `false` and optional
/// strings to empty/`None` so a sparse payload still deserializes.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeMember {
    pub user_id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub speaking: bool,
    #[serde(default)]
    pub self_mute: bool,
    #[serde(default)]
    pub self_deaf: bool,
    #[serde(default)]
    pub server_mute: bool,
    #[serde(default)]
    pub server_deaf: bool,
}

impl BridgeSnapshot {
    /// Convert the wire snapshot into the domain roster the feed publishes.
    pub fn into_roster(self) -> VoiceRoster {
        VoiceRoster {
            channel_id: self.channel_id,
            channel_name: self.channel_name,
            members: self
                .members
                .into_iter()
                .map(BridgeMember::into_member)
                .collect(),
        }
    }
}

impl BridgeMember {
    fn into_member(self) -> VoiceMember {
        VoiceMember {
            user_id: self.user_id,
            display_name: self.display_name,
            avatar_url: self.avatar_url,
            speaking: self.speaking,
            self_mute: self.self_mute,
            self_deaf: self.self_deaf,
            server_mute: self.server_mute,
            server_deaf: self.server_deaf,
        }
    }
}

#[cfg(test)]
#[path = "bridge_payload_tests.rs"]
mod bridge_payload_tests;
