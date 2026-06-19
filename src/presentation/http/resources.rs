//! Feed DTOs (M3).
//!
//! The explicit serde contract serialized onto the Overlay Feed (`GET
//! /overlay/feed`). These are the stable shapes the React Overlays render; the
//! domain types stay free of presentation concerns. This slice carries chat
//! messages, `chatMessageDeleted` moderation signals, and `streamEvent`s
//! (donation / sub / raid …) that the Coworking Overlay's Footer Bar turns into
//! Alerts.

use serde::Serialize;

use crate::domain::{
    ChatBadge, ChatMessage, ChatMessageDeleted, ChatSignal, MessageFragment, NowPlaying,
    PlaybackStatus, StreamEvent, SubTier,
};

/// One ordered piece of a chat message body, as the React side consumes it.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FragmentDto {
    Text {
        text: String,
    },
    Emote {
        id: String,
        url: String,
    },
}

impl From<&MessageFragment> for FragmentDto {
    fn from(fragment: &MessageFragment) -> Self {
        match fragment {
            MessageFragment::Text(text) => FragmentDto::Text { text: text.clone() },
            MessageFragment::Emote { id, url } => FragmentDto::Emote {
                id: id.clone(),
                url: url.clone(),
            },
        }
    }
}

/// One resolved native Twitch badge as the React side consumes it. Badges that
/// the Helix resolver could not resolve (a miss) carry no `url` and are dropped
/// here so the Overlay only ever receives renderable badges.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChatBadgeDto {
    pub set_id: String,
    pub version: String,
    pub url: String,
}

/// A chat message as it appears on the Overlay Feed.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessageDto {
    pub msg_id: String,
    pub username: String,
    pub color: String,
    pub channel: String,
    pub badges: Vec<ChatBadgeDto>,
    pub fragments: Vec<FragmentDto>,
}

impl ChatBadgeDto {
    /// Build a DTO from a resolved domain badge, or `None` when it has no url
    /// (a resolver miss) so the feed only carries renderable badges.
    fn from_badge(badge: &ChatBadge) -> Option<Self> {
        badge.url.as_ref().map(|url| Self {
            set_id: badge.set.clone(),
            version: badge.version.clone(),
            url: url.clone(),
        })
    }
}

impl From<&ChatMessage> for ChatMessageDto {
    fn from(msg: &ChatMessage) -> Self {
        Self {
            msg_id: msg.msg_id.clone(),
            username: msg.username.clone(),
            color: msg.color.clone(),
            channel: msg.channel.clone(),
            badges: msg
                .badges
                .iter()
                .filter_map(ChatBadgeDto::from_badge)
                .collect(),
            fragments: msg.fragments.iter().map(FragmentDto::from).collect(),
        }
    }
}

/// A single-message moderation delete, as the React side consumes it. The Chat
/// Overlay removes the message node whose key matches `msgId`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessageDeletedDto {
    pub msg_id: String,
}

impl From<&ChatMessageDeleted> for ChatMessageDeletedDto {
    fn from(deleted: &ChatMessageDeleted) -> Self {
        Self {
            msg_id: deleted.msg_id.clone(),
        }
    }
}

/// A stream event as it appears on the Overlay Feed (donation / sub / raid …).
///
/// A presentation DTO mirroring the domain [`StreamEvent`], so the camelCase
/// wire contract the React Overlays consume lives in the presentation layer and
/// the domain type stays free of serialization concerns. The inner `type`
/// discriminant selects the Footer Bar's Alert template; the outer `kind` on
/// [`FeedEvent`] selects the stream-event branch.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StreamEventDto {
    Follow {
        username: String,
    },
    Sub {
        username: String,
        tier: SubTierDto,
        months: u32,
    },
    Donation {
        username: String,
        // serde's `rename_all` is not applied across `#[serde(flatten)]`, so the
        // camelCase wire names are spelled out explicitly on the multi-word
        // fields the Footer Bar reads.
        #[serde(rename = "amountCents")]
        amount_cents: u64,
        message: String,
    },
    GiftSub {
        username: String,
        tier: SubTierDto,
        total: u32,
    },
    Cheer {
        username: String,
        bits: u64,
        message: String,
    },
    Raid {
        #[serde(rename = "fromChannel")]
        from_channel: String,
        viewers: u32,
    },
    ViewerCountUpdate {
        count: u32,
    },
}

/// The sub tier as the React side consumes it (camelCase variants).
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SubTierDto {
    Tier1,
    Tier2,
    Tier3,
    Prime,
}

impl From<&SubTier> for SubTierDto {
    fn from(tier: &SubTier) -> Self {
        match tier {
            SubTier::Tier1 => SubTierDto::Tier1,
            SubTier::Tier2 => SubTierDto::Tier2,
            SubTier::Tier3 => SubTierDto::Tier3,
            SubTier::Prime => SubTierDto::Prime,
        }
    }
}

impl From<&StreamEvent> for StreamEventDto {
    fn from(event: &StreamEvent) -> Self {
        match event {
            StreamEvent::Follow { username } => StreamEventDto::Follow {
                username: username.clone(),
            },
            StreamEvent::Sub {
                username,
                tier,
                months,
            } => StreamEventDto::Sub {
                username: username.clone(),
                tier: tier.into(),
                months: *months,
            },
            StreamEvent::Donation {
                username,
                amount_cents,
                message,
            } => StreamEventDto::Donation {
                username: username.clone(),
                amount_cents: *amount_cents,
                message: message.clone(),
            },
            StreamEvent::GiftSub {
                username,
                tier,
                total,
            } => StreamEventDto::GiftSub {
                username: username.clone(),
                tier: tier.into(),
                total: *total,
            },
            StreamEvent::Cheer {
                username,
                bits,
                message,
            } => StreamEventDto::Cheer {
                username: username.clone(),
                bits: *bits,
                message: message.clone(),
            },
            StreamEvent::Raid {
                from_channel,
                viewers,
            } => StreamEventDto::Raid {
                from_channel: from_channel.clone(),
                viewers: *viewers,
            },
            StreamEvent::ViewerCountUpdate { count } => {
                StreamEventDto::ViewerCountUpdate { count: *count }
            }
        }
    }
}

/// The currently-playing Spotify track as the React side consumes it.
///
/// now-playing is ambient *state* (latest value on a watch channel), not a
/// logged event — but it rides the same Overlay Feed so the Coworking Overlay's
/// Now Playing widget can render it. `status` mirrors the domain
/// [`PlaybackStatus`]; the React side falls back to the placeholder on
/// `"stopped"`. `artUrl` is the real `mpris:artUrl` album cover when present.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NowPlayingDto {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub art_url: Option<String>,
    pub status: &'static str,
}

/// A single Overlay Feed event. A tagged union so Overlays can switch on `kind`;
/// this slice emits `chatMessage`, `chatMessageDeleted`, `streamEvent`, and
/// `nowPlaying`.
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
    /// `status` is mapped from the domain [`PlaybackStatus`]; the Coworking
    /// Overlay's Now Playing widget renders the track (animation reflects status).
    pub fn now_playing(now_playing: &NowPlaying) -> Self {
        let status = match now_playing.status {
            PlaybackStatus::Playing => "playing",
            PlaybackStatus::Paused => "paused",
            PlaybackStatus::Stopped => "stopped",
        };
        FeedEvent::NowPlaying(NowPlayingDto {
            title: now_playing.title.clone(),
            artist: now_playing.artist.clone(),
            album: now_playing.album.clone(),
            art_url: now_playing.art_url.clone(),
            status,
        })
    }

    /// Build a "cleared" now-playing feed event — emitted when playback stops or
    /// no player is present. Empty strings + no art + `"stopped"` status drive
    /// the Now Playing widget back to its placeholder.
    pub fn now_playing_cleared() -> Self {
        FeedEvent::NowPlaying(NowPlayingDto {
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            art_url: None,
            status: "stopped",
        })
    }

    /// Serialize to the JSON line carried in the SSE `data:` field.
    pub fn to_json(&self) -> String {
        // The DTOs are plain serializable structs, so this never fails; fall
        // back to an empty object rather than panicking on a display source.
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ChatMessage;

    #[test]
    fn chat_message_serializes_to_feed_contract() {
        let msg = ChatMessage::from_text("abc-1", "danielhe4rt", Some("#FF7F50"), "rustlang", "hello");
        let feed = FeedEvent::chat(&msg);

        let value: serde_json::Value = serde_json::to_value(&feed).unwrap();

        assert_eq!(value["kind"], "chatMessage");
        assert_eq!(value["msgId"], "abc-1");
        assert_eq!(value["username"], "danielhe4rt");
        assert_eq!(value["color"], "#FF7F50");
        assert_eq!(value["channel"], "rustlang");
        assert_eq!(value["fragments"][0]["kind"], "text");
        assert_eq!(value["fragments"][0]["text"], "hello");
    }

    #[test]
    fn emote_fragment_serializes_with_id_and_url() {
        let dto = FragmentDto::from(&MessageFragment::Emote {
            id: "25".into(),
            url: "https://cdn/emote/25.png".into(),
        });
        let value = serde_json::to_value(&dto).unwrap();
        assert_eq!(value["kind"], "emote");
        assert_eq!(value["id"], "25");
        assert_eq!(value["url"], "https://cdn/emote/25.png");
    }

    #[test]
    fn resolved_badges_serialize_and_misses_are_dropped() {
        use crate::domain::ChatBadge;

        let msg = ChatMessage::from_text("b-1", "moduser", Some("#fff"), "rustlang", "hi")
            .with_badges(vec![
                ChatBadge {
                    set: "moderator".into(),
                    version: "1".into(),
                    url: Some("https://cdn/mod.png".into()),
                },
                // Unresolved badge (a Helix miss) — must not reach the feed.
                ChatBadge::new("glitchcon2020", "1"),
            ]);

        let value: serde_json::Value = serde_json::to_value(FeedEvent::chat(&msg)).unwrap();

        let badges = value["badges"].as_array().unwrap();
        assert_eq!(badges.len(), 1, "only resolved badges reach the feed");
        assert_eq!(badges[0]["setId"], "moderator");
        assert_eq!(badges[0]["version"], "1");
        assert_eq!(badges[0]["url"], "https://cdn/mod.png");
    }

    #[test]
    fn message_without_badges_serializes_empty_array() {
        let msg = ChatMessage::from_text("b-2", "viewer", None, "chan", "gm");
        let value: serde_json::Value = serde_json::to_value(FeedEvent::chat(&msg)).unwrap();
        assert_eq!(value["badges"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn to_json_round_trips_fields() {
        let msg = ChatMessage::from_text("id2", "viewer", None, "chan", "gm");
        let json = FeedEvent::chat(&msg).to_json();
        assert!(json.contains("\"kind\":\"chatMessage\""));
        assert!(json.contains("\"msgId\":\"id2\""));
    }

    // ── M3 (extended) — single-message delete (CLEARMSG) DTO ─────────────────

    #[test]
    fn delete_serializes_to_feed_contract() {
        use crate::domain::ChatMessageDeleted;

        let feed = FeedEvent::deleted(&ChatMessageDeleted::new("abc-123"));
        let value: serde_json::Value = serde_json::to_value(&feed).unwrap();

        // The tagged union discriminates on `kind`, and the only payload field
        // is the camelCased `msgId` the Overlay keys its DOM nodes by.
        assert_eq!(value["kind"], "chatMessageDeleted");
        assert_eq!(value["msgId"], "abc-123");
        assert!(value.get("username").is_none(), "delete carries no body");
    }

    #[test]
    fn delete_to_json_emits_kind_and_msg_id() {
        use crate::domain::ChatMessageDeleted;

        let json = FeedEvent::deleted(&ChatMessageDeleted::new("del-1")).to_json();
        assert!(json.contains("\"kind\":\"chatMessageDeleted\""), "got: {json}");
        assert!(json.contains("\"msgId\":\"del-1\""), "got: {json}");
    }

    // ── stream events on the feed (Coworking Overlay Footer Bar Alerts) ──────────

    #[test]
    fn donation_serializes_to_feed_contract() {
        use crate::domain::StreamEvent;

        let feed = FeedEvent::stream(&StreamEvent::Donation {
            username: "danielhe4rt".into(),
            amount_cents: 500,
            message: "vai rust!".into(),
        });
        let value: serde_json::Value = serde_json::to_value(&feed).unwrap();

        // The domain StreamEvent is flattened inline: the outer `kind` selects
        // the Footer Bar's stream-event branch, the inner `type` the template.
        assert_eq!(value["kind"], "streamEvent");
        assert_eq!(value["type"], "donation");
        assert_eq!(value["username"], "danielhe4rt");
        assert_eq!(value["amountCents"], 500);
        assert_eq!(value["message"], "vai rust!");
    }

    #[test]
    fn raid_and_sub_carry_their_type_discriminant() {
        use crate::domain::{StreamEvent, SubTier};

        let raid = serde_json::to_value(FeedEvent::stream(&StreamEvent::Raid {
            from_channel: "ferris".into(),
            viewers: 42,
        }))
        .unwrap();
        assert_eq!(raid["kind"], "streamEvent");
        assert_eq!(raid["type"], "raid");
        assert_eq!(raid["fromChannel"], "ferris");
        assert_eq!(raid["viewers"], 42);

        let sub = serde_json::to_value(FeedEvent::stream(&StreamEvent::Sub {
            username: "viewer".into(),
            tier: SubTier::Tier1,
            months: 3,
        }))
        .unwrap();
        assert_eq!(sub["kind"], "streamEvent");
        assert_eq!(sub["type"], "sub");
        assert_eq!(sub["months"], 3);
    }

    #[test]
    fn stream_event_to_json_emits_kind_and_type() {
        use crate::domain::StreamEvent;

        let json = FeedEvent::stream(&StreamEvent::Donation {
            username: "u".into(),
            amount_cents: 100,
            message: String::new(),
        })
        .to_json();
        assert!(json.contains("\"kind\":\"streamEvent\""), "got: {json}");
        assert!(json.contains("\"type\":\"donation\""), "got: {json}");
    }

    // ── now-playing on the feed (Coworking Overlay Now Playing widget) ───────────

    #[test]
    fn now_playing_serializes_to_feed_contract() {
        use crate::domain::{NowPlaying, PlaybackStatus};

        let feed = FeedEvent::now_playing(&NowPlaying {
            title: "Money".into(),
            artist: "Pink Floyd".into(),
            album: "The Dark Side of the Moon".into(),
            art_url: Some("https://i.scdn.co/image/abc".into()),
            status: PlaybackStatus::Playing,
        });
        let value: serde_json::Value = serde_json::to_value(&feed).unwrap();

        // The outer `kind` selects the now-playing branch; the camelCase fields
        // are what the React Now Playing widget renders.
        assert_eq!(value["kind"], "nowPlaying");
        assert_eq!(value["title"], "Money");
        assert_eq!(value["artist"], "Pink Floyd");
        assert_eq!(value["album"], "The Dark Side of the Moon");
        assert_eq!(value["artUrl"], "https://i.scdn.co/image/abc");
        assert_eq!(value["status"], "playing");
    }

    #[test]
    fn now_playing_maps_paused_status() {
        use crate::domain::{NowPlaying, PlaybackStatus};

        let value = serde_json::to_value(FeedEvent::now_playing(&NowPlaying {
            title: "Time".into(),
            artist: "Pink Floyd".into(),
            album: "The Dark Side of the Moon".into(),
            art_url: None,
            status: PlaybackStatus::Paused,
        }))
        .unwrap();
        assert_eq!(value["kind"], "nowPlaying");
        assert_eq!(value["status"], "paused");
        assert!(value["artUrl"].is_null(), "no art url serializes as null");
    }

    #[test]
    fn now_playing_cleared_is_stopped_and_empty() {
        let value: serde_json::Value =
            serde_json::to_value(FeedEvent::now_playing_cleared()).unwrap();

        assert_eq!(value["kind"], "nowPlaying");
        assert_eq!(value["status"], "stopped");
        assert_eq!(value["title"], "");
        assert_eq!(value["artist"], "");
        assert_eq!(value["album"], "");
        assert!(value["artUrl"].is_null(), "cleared carries no art url");
    }

    #[test]
    fn now_playing_to_json_emits_kind() {
        use crate::domain::{NowPlaying, PlaybackStatus};

        let json = FeedEvent::now_playing(&NowPlaying {
            title: "Breathe".into(),
            artist: "Pink Floyd".into(),
            album: "The Dark Side of the Moon".into(),
            art_url: None,
            status: PlaybackStatus::Playing,
        })
        .to_json();
        assert!(json.contains("\"kind\":\"nowPlaying\""), "got: {json}");
        assert!(json.contains("\"status\":\"playing\""), "got: {json}");
    }

    #[test]
    fn from_signal_maps_both_chat_variants() {
        use crate::domain::{ChatMessageDeleted, ChatSignal};

        let msg = ChatMessage::from_text("m-1", "viewer", None, "chan", "hi");
        let feed = FeedEvent::from_signal(&ChatSignal::Message(msg));
        assert!(matches!(feed, FeedEvent::ChatMessage(_)));

        let feed = FeedEvent::from_signal(&ChatSignal::Deleted(ChatMessageDeleted::new("m-1")));
        let value = serde_json::to_value(&feed).unwrap();
        assert_eq!(value["kind"], "chatMessageDeleted");
        assert_eq!(value["msgId"], "m-1");
    }
}
