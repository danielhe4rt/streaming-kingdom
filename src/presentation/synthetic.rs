//! Synthetic (test) event builders — the single source of truth for fabricated
//! events, shared by the TUI **Test events** pane and the `/overlay/dev` HTTP
//! panel (ADR-0002). Pure, seedable preset builders plus one shared cycle
//! counter so both trigger surfaces emit identical events that ride the real
//! channels (indistinguishable from real traffic).

use std::sync::atomic::{AtomicU64, Ordering};

use crate::domain::{
    ChatMessage, MessageFragment, NowPlaying, PlaybackStatus, StreamEvent, SubTier, emote_cdn_url,
};

/// One fireable test action. `ALL` is the display order in the Test events pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntheticKind {
    Donation,
    Sub,
    GiftSub,
    Follow,
    Raid,
    Cheer,
    ViewerCount,
    Chat,
    NowPlaying,
    /// Delete the most recently fired synthetic chat message (tests CLEARMSG).
    DeleteLast,
}

impl SyntheticKind {
    pub const ALL: &'static [SyntheticKind] = &[
        SyntheticKind::Donation,
        SyntheticKind::Sub,
        SyntheticKind::GiftSub,
        SyntheticKind::Follow,
        SyntheticKind::Raid,
        SyntheticKind::Cheer,
        SyntheticKind::ViewerCount,
        SyntheticKind::Chat,
        SyntheticKind::NowPlaying,
        SyntheticKind::DeleteLast,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SyntheticKind::Donation => "Donation",
            SyntheticKind::Sub => "Sub",
            SyntheticKind::GiftSub => "Gift Sub",
            SyntheticKind::Follow => "Follow",
            SyntheticKind::Raid => "Raid",
            SyntheticKind::Cheer => "Cheer (bits)",
            SyntheticKind::ViewerCount => "Viewer count",
            SyntheticKind::Chat => "Chat message",
            SyntheticKind::NowPlaying => "Now playing",
            SyntheticKind::DeleteLast => "Delete last message",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            SyntheticKind::Donation => "💸",
            SyntheticKind::Sub => "💜",
            SyntheticKind::GiftSub => "🎁",
            SyntheticKind::Follow => "⭐",
            SyntheticKind::Raid => "⚡",
            SyntheticKind::Cheer => "💎",
            SyntheticKind::ViewerCount => "👁",
            SyntheticKind::Chat => "💬",
            SyntheticKind::NowPlaying => "🎵",
            SyntheticKind::DeleteLast => "🗑",
        }
    }
}

/// Monotonic cycle counter shared by both trigger surfaces so fired events vary
/// (usernames / amounts / ids) without a randomness crate.
static SEQ: AtomicU64 = AtomicU64::new(0);

pub fn next_seq() -> u64 {
    SEQ.fetch_add(1, Ordering::Relaxed)
}

const USERS: &[&str] = &[
    "joaodev",
    "mariacode",
    "devzin",
    "rustacean",
    "noobmaster",
    "laravelfan",
    "annacodes",
    "pgdev",
];

const COLORS: &[&str] = &[
    "#8b2fe8", "#1ed760", "#ffcb05", "#c9a4ff", "#ff6b6b", "#4ecdc4",
];

const CHAT_TEXTS: &[&str] = &[
    "eae galera, bora codar! 💜",
    "esse Laravel tá voando",
    "PHP é vida kkk",
    "bora de rust 🦀",
    "alguém tem a doc disso?",
    "primeira vez na live, salve!",
    "da comunidade pra comunidade 💜",
    "postgres > tudo",
];

/// (title, artist, album) presets for the fake now-playing track.
const TRACKS: &[(&str, &str, &str)] = &[
    ("Get Lucky", "Daft Punk", "Random Access Memories"),
    ("Money", "Pink Floyd", "The Dark Side of the Moon"),
    ("Bohemian Rhapsody", "Queen", "A Night at the Opera"),
    ("lofi beats to code to", "He4rt Radio", "Live"),
];

fn pick<'a>(list: &'a [&'a str], n: u64) -> &'a str {
    list[(n as usize) % list.len()]
}

/// Build a synthetic [`StreamEvent`] for one of the stream-event kinds. Returns
/// `None` for kinds that are not stream events (Chat / NowPlaying / DeleteLast).
pub fn stream_event(kind: SyntheticKind, seq: u64) -> Option<StreamEvent> {
    let user = pick(USERS, seq).to_string();
    Some(match kind {
        SyntheticKind::Donation => StreamEvent::Donation {
            username: user,
            amount_cents: (seq % 50 + 1) * 100,
            message: "valeu pela live! 💜".to_string(),
        },
        SyntheticKind::Sub => StreamEvent::Sub {
            username: user,
            tier: SubTier::Tier1,
            months: (seq % 24 + 1) as u32,
        },
        SyntheticKind::GiftSub => StreamEvent::GiftSub {
            username: user,
            tier: SubTier::Tier2,
            total: (seq % 10 + 1) as u32,
        },
        SyntheticKind::Follow => StreamEvent::Follow { username: user },
        SyntheticKind::Raid => StreamEvent::Raid {
            from_channel: user,
            viewers: (seq % 90 + 10) as u32,
        },
        SyntheticKind::Cheer => StreamEvent::Cheer {
            username: user,
            bits: (seq % 9 + 1) * 100,
            message: "toma uns bits 💎".to_string(),
        },
        SyntheticKind::ViewerCount => StreamEvent::ViewerCountUpdate {
            count: (seq % 500 + 20) as u32,
        },
        SyntheticKind::Chat | SyntheticKind::NowPlaying | SyntheticKind::DeleteLast => {
            return None;
        }
    })
}

/// Build a synthetic chat message. Every third (by seq) carries an emote
/// fragment so inline emote rendering is exercised too.
pub fn chat_message(seq: u64) -> ChatMessage {
    let mut msg = ChatMessage::from_text(
        format!("dev-{seq}"),
        pick(USERS, seq),
        Some(pick(COLORS, seq)),
        "danielhe4rt",
        pick(CHAT_TEXTS, seq),
    );
    if seq % 3 == 0 {
        msg.fragments.push(MessageFragment::Emote {
            id: "25".to_string(),
            url: emote_cdn_url("25"),
        });
    }
    msg
}

/// Build a synthetic now-playing track (always Playing; no cover art — the disc
/// renders, and the real album cover is exercised with a real Spotify session).
pub fn now_playing(seq: u64) -> NowPlaying {
    let (title, artist, album) = TRACKS[(seq as usize) % TRACKS.len()];
    NowPlaying {
        title: title.to_string(),
        artist: artist.to_string(),
        album: album.to_string(),
        art_url: None,
        status: PlaybackStatus::Playing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_event_builds_the_requested_variant() {
        assert!(matches!(
            stream_event(SyntheticKind::Donation, 0),
            Some(StreamEvent::Donation { .. })
        ));
        assert!(matches!(
            stream_event(SyntheticKind::Raid, 3),
            Some(StreamEvent::Raid { .. })
        ));
    }

    #[test]
    fn non_stream_kinds_have_no_stream_event() {
        assert!(stream_event(SyntheticKind::Chat, 0).is_none());
        assert!(stream_event(SyntheticKind::NowPlaying, 0).is_none());
        assert!(stream_event(SyntheticKind::DeleteLast, 0).is_none());
    }

    #[test]
    fn donation_amount_is_positive() {
        match stream_event(SyntheticKind::Donation, 7).unwrap() {
            StreamEvent::Donation { amount_cents, .. } => assert!(amount_cents > 0),
            _ => panic!("expected donation"),
        }
    }

    #[test]
    fn chat_message_seeds_emote_every_third() {
        // seq % 3 == 0 → carries an emote fragment (text + emote).
        let with_emote = chat_message(0);
        assert!(with_emote
            .fragments
            .iter()
            .any(|f| matches!(f, MessageFragment::Emote { .. })));
        // seq % 3 != 0 → text only.
        let text_only = chat_message(1);
        assert!(text_only
            .fragments
            .iter()
            .all(|f| matches!(f, MessageFragment::Text(_))));
    }

    #[test]
    fn now_playing_is_playing_with_fields() {
        let np = now_playing(0);
        assert_eq!(np.status, PlaybackStatus::Playing);
        assert!(!np.title.is_empty());
        assert!(!np.artist.is_empty());
    }

    #[test]
    fn all_kinds_have_a_label_and_icon() {
        for k in SyntheticKind::ALL {
            assert!(!k.label().is_empty());
            assert!(!k.icon().is_empty());
        }
    }
}
