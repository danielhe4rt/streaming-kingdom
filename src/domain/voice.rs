// ---------------------------------------------------------------------------
// Voice — observed STATE of a Discord voice channel, not an event
//
// Like NowPlaying (see media_player.rs), VoiceRoster is *ambient state*: at most
// one current roster for the channel the streamer is in, latest-value-wins, never
// replayed as history. It rides a tokio::sync::watch channel and is deliberately
// NEVER wrapped in AppEvent. `None` on that channel = not in a channel / Discord
// disconnected. The Discord bridge (infra/discord) builds a full snapshot per
// change from Vesktop's voice stores, so the toolkit replaces the roster wholesale
// rather than mutating it incrementally.
// ---------------------------------------------------------------------------

/// One member present in the followed Discord voice channel.
///
/// `display_name` resolves nick → global_name → username at mapping time, so the
/// reducer only ever sees a final string. `avatar_url` is the CDN url derived from
/// the user's avatar hash, or `None` to fall back to a default avatar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceMember {
    pub user_id: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub speaking: bool,
    pub self_mute: bool,
    pub self_deaf: bool,
    pub server_mute: bool,
    pub server_deaf: bool,
}

/// Ambient state of the voice channel the streamer is currently in.
///
/// `channel_id`/`channel_name` are `None` between channels; `members` is empty
/// then too. A fresh full snapshot is published after every mutation, so the
/// struct is `Clone` and compared by value (the watch channel dedupes on `Eq`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VoiceRoster {
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub members: Vec<VoiceMember>,
}
