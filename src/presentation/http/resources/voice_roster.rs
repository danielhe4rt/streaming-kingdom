//! Voice-roster feed DTO — the ambient Discord voice-channel state.
//!
//! Like now-playing, the roster is ambient *state* (latest value on a watch
//! channel), not a logged event — it rides the same Overlay Feed so the React
//! Voice Roster widget can render one card per member (avatar, speaking ring,
//! mute/deaf). `channelId`/`channelName` are `null` between channels and
//! `members` empty then too (a cleared roster), so the widget can switch back to
//! empty without a separate frame. The domain [`VoiceRoster`] stays free of
//! serialization concerns; this is the camelCase wire contract.

use serde::Serialize;

use crate::domain::{VoiceMember, VoiceRoster};

/// One member of the followed Discord voice channel as the React side consumes
/// it. `displayName` was already resolved (nick → global_name → username) and
/// `avatarUrl` already derived from the avatar hash by the mapping layer.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VoiceMemberDto {
    pub user_id: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub speaking: bool,
    pub self_mute: bool,
    pub self_deaf: bool,
    pub server_mute: bool,
    pub server_deaf: bool,
}

impl From<&VoiceMember> for VoiceMemberDto {
    fn from(member: &VoiceMember) -> Self {
        Self {
            user_id: member.user_id.clone(),
            display_name: member.display_name.clone(),
            avatar_url: member.avatar_url.clone(),
            speaking: member.speaking,
            self_mute: member.self_mute,
            self_deaf: member.self_deaf,
            server_mute: member.server_mute,
            server_deaf: member.server_deaf,
        }
    }
}

/// The voice channel the streamer is currently in, as the React side consumes
/// it. `channelId`/`channelName` are `null` and `members` empty between channels
/// (a cleared roster — Discord disconnected or left voice).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VoiceRosterDto {
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub members: Vec<VoiceMemberDto>,
}

impl From<&VoiceRoster> for VoiceRosterDto {
    fn from(roster: &VoiceRoster) -> Self {
        Self {
            channel_id: roster.channel_id.clone(),
            channel_name: roster.channel_name.clone(),
            members: roster.members.iter().map(VoiceMemberDto::from).collect(),
        }
    }
}
