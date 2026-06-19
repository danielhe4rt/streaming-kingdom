//! Chat feed DTOs — `chatMessage` body (fragments + resolved badges) and the
//! `chatMessageDeleted` moderation signal. The camelCase wire contract the React
//! Chat Overlay consumes; domain types stay free of presentation concerns.

use serde::Serialize;

use crate::domain::{ChatBadge, ChatMessage, ChatMessageDeleted, MessageFragment};

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
