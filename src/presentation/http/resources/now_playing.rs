//! Now-playing feed DTO — the ambient Spotify track state.
//!
//! now-playing is ambient *state* (latest value on a watch channel), not a
//! logged event — but it rides the same Overlay Feed so the Coworking Overlay's
//! Now Playing widget can render it. `status` mirrors the domain
//! [`PlaybackStatus`]; the React side falls back to the placeholder on
//! `"stopped"`. `artUrl` is the real `mpris:artUrl` album cover when present.

use serde::Serialize;

use crate::domain::{NowPlaying, PlaybackStatus};

/// The currently-playing Spotify track as the React side consumes it.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NowPlayingDto {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub art_url: Option<String>,
    pub status: &'static str,
}

impl NowPlayingDto {
    /// A "cleared" now-playing DTO — emitted when playback stops or no player is
    /// present. Empty strings + no art + `"stopped"` status drive the Now Playing
    /// widget back to its placeholder.
    pub fn cleared() -> Self {
        NowPlayingDto {
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            art_url: None,
            status: "stopped",
        }
    }
}

impl From<&NowPlaying> for NowPlayingDto {
    fn from(now_playing: &NowPlaying) -> Self {
        let status = match now_playing.status {
            PlaybackStatus::Playing => "playing",
            PlaybackStatus::Paused => "paused",
            PlaybackStatus::Stopped => "stopped",
        };
        NowPlayingDto {
            title: now_playing.title.clone(),
            artist: now_playing.artist.clone(),
            album: now_playing.album.clone(),
            art_url: now_playing.art_url.clone(),
            status,
        }
    }
}
