// ---------------------------------------------------------------------------
// Now Playing – observed STATE, not an event
//
// Unlike StreamEvent / ChatMessage (discrete occurrences on broadcast channels
// and logged into AppEvent history), NowPlaying is *ambient state*: at most one
// current track, latest-value-wins, never replayed as history. It rides a
// tokio::sync::watch channel and is deliberately NEVER wrapped in AppEvent.
// See src/infrastructure/docs/adr/0001-now-playing-observed-state.md.
// ---------------------------------------------------------------------------

/// Playback status of the observed media player (mapped from `playerctl`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

/// The current track as observed from the media player.
///
/// `art_url` is the `mpris:artUrl` (real album cover) when available; consumers
/// fall back to a placeholder when it is `None` or when status is `Stopped`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NowPlaying {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub art_url: Option<String>,
    pub status: PlaybackStatus,
}
