//! Feed DTOs (M3).
//!
//! The explicit serde contract serialized onto the Overlay Feed (`GET
//! /overlay/feed`). These are the stable shapes the React Overlays render; the
//! domain types stay free of presentation concerns. This slice only carries
//! chat — later slices add `streamEvent` and `chatMessageDeleted` variants to
//! [`FeedEvent`].

use serde::Serialize;

use crate::domain::{ChatMessage, MessageFragment};

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

/// A chat message as it appears on the Overlay Feed.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessageDto {
    pub msg_id: String,
    pub username: String,
    pub color: String,
    pub channel: String,
    pub fragments: Vec<FragmentDto>,
}

impl From<&ChatMessage> for ChatMessageDto {
    fn from(msg: &ChatMessage) -> Self {
        Self {
            msg_id: msg.msg_id.clone(),
            username: msg.username.clone(),
            color: msg.color.clone(),
            channel: msg.channel.clone(),
            fragments: msg.fragments.iter().map(FragmentDto::from).collect(),
        }
    }
}

/// A single Overlay Feed event. A tagged union so Overlays can switch on `kind`;
/// this slice only emits `chatMessage`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FeedEvent {
    ChatMessage(ChatMessageDto),
}

impl FeedEvent {
    /// Build a feed event from a domain chat message.
    pub fn chat(msg: &ChatMessage) -> Self {
        FeedEvent::ChatMessage(ChatMessageDto::from(msg))
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
    fn to_json_round_trips_fields() {
        let msg = ChatMessage::from_text("id2", "viewer", None, "chan", "gm");
        let json = FeedEvent::chat(&msg).to_json();
        assert!(json.contains("\"kind\":\"chatMessage\""));
        assert!(json.contains("\"msgId\":\"id2\""));
    }
}
