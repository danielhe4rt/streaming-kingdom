//! Feed DTOs (M3).
//!
//! The explicit serde contract serialized onto the Overlay Feed (`GET
//! /overlay/feed`). These are the stable shapes the React Overlays render; the
//! domain types stay free of presentation concerns. Split by concern: chat
//! messages + `chatMessageDeleted` ([`chat`]), `streamEvent`s ([`stream`]),
//! ambient `nowPlaying` ([`now_playing`]) and `voiceRoster` ([`voice_roster`])
//! state. This module owns the [`FeedEvent`] tagged union that ties them
//! together and the constructors the controllers call.

mod chat;
mod now_playing;
mod stream;
mod voice_roster;

#[cfg(test)]
#[path = "chat_tests.rs"]
mod chat_tests;
#[cfg(test)]
#[path = "now_playing_tests.rs"]
mod now_playing_tests;
#[cfg(test)]
#[path = "stream_tests.rs"]
mod stream_tests;
#[cfg(test)]
#[path = "voice_roster_tests.rs"]
mod voice_roster_tests;

use serde::Serialize;

use crate::domain::{
    ChatMessage, ChatMessageDeleted, ChatSignal, NowPlaying, StreamEvent, VoiceRoster,
};

pub use chat::{ChatMessageDeletedDto, ChatMessageDto};
pub use now_playing::NowPlayingDto;
pub use stream::StreamEventDto;
pub use voice_roster::VoiceRosterDto;

/// A single Overlay Feed event. A tagged union so Overlays can switch on `kind`;
/// this slice emits `chatMessage`, `chatMessageDeleted`, `streamEvent`,
/// `nowPlaying`, and `voiceRoster`.
///
/// The `streamEvent` variant flattens the [`StreamEventDto`] inline, so a
/// donation serializes as `{ "kind": "streamEvent", "type": "donation", … }` —
/// the Coworking Overlay's Footer Bar switches on `kind` first, then on the inner
/// `type` to pick an Alert template.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FeedEvent {
    ChatMessage(ChatMessageDto),
    ChatMessageDeleted(ChatMessageDeletedDto),
    StreamEvent {
        #[serde(flatten)]
        event: StreamEventDto,
    },
    NowPlaying(NowPlayingDto),
    VoiceRoster(VoiceRosterDto),
}

impl FeedEvent {
    /// Build a feed event from a domain chat message.
    pub fn chat(msg: &ChatMessage) -> Self {
        FeedEvent::ChatMessage(ChatMessageDto::from(msg))
    }

    /// Build a feed event from a single-message moderation delete (CLEARMSG).
    pub fn deleted(deleted: &ChatMessageDeleted) -> Self {
        FeedEvent::ChatMessageDeleted(ChatMessageDeletedDto::from(deleted))
    }

    /// Build a feed event from any chat broadcast signal.
    pub fn from_signal(signal: &ChatSignal) -> Self {
        match signal {
            ChatSignal::Message(msg) => Self::chat(msg),
            ChatSignal::Deleted(deleted) => Self::deleted(deleted),
        }
    }

    /// Build a feed event from a domain stream event (donation / sub / raid …).
    /// The Coworking Overlay's Footer Bar turns this into a queued Alert.
    pub fn stream(event: &StreamEvent) -> Self {
        FeedEvent::StreamEvent {
            event: StreamEventDto::from(event),
        }
    }

    /// Build a feed event from the current now-playing state (Playing/Paused).
    /// `status` is mapped from the domain `PlaybackStatus`; the Coworking
    /// Overlay's Now Playing widget renders the track (animation reflects status).
    pub fn now_playing(now_playing: &NowPlaying) -> Self {
        FeedEvent::NowPlaying(NowPlayingDto::from(now_playing))
    }

    /// Build a "cleared" now-playing feed event — emitted when playback stops or
    /// no player is present. Empty strings + no art + `"stopped"` status drive
    /// the Now Playing widget back to its placeholder.
    pub fn now_playing_cleared() -> Self {
        FeedEvent::NowPlaying(NowPlayingDto::cleared())
    }

    /// Build a feed event from the current voice-roster state. The React Voice
    /// Roster widget renders one card per member (avatar, speaking ring,
    /// mute/deaf). A cleared roster (`channelId: null`, `members: []`) drives the
    /// widget back to empty — see [`VoiceRoster::default`].
    pub fn voice_roster(roster: &VoiceRoster) -> Self {
        FeedEvent::VoiceRoster(VoiceRosterDto::from(roster))
    }

    /// Serialize to the JSON line carried in the SSE `data:` field.
    pub fn to_json(&self) -> String {
        // The DTOs are plain serializable structs, so this never fails; fall
        // back to an empty object rather than panicking on a display source.
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}
