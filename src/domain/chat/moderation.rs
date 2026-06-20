//! Single-message moderation signals and the chat broadcast envelope.

use super::ChatMessage;

/// A single-message moderation signal: one chat message was deleted.
///
/// Carries the stable [`ChatMessage::msg_id`] of the removed message — the same
/// key the Coworking Overlay uses as its DOM node key — so the Overlay can drop just
/// that node. Produced by the IRC adapter from a Twitch `CLEARMSG`. A `CLEARCHAT`
/// (timeout/ban clearing a user's history) is a separate, out-of-scope signal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessageDeleted {
    /// The id of the deleted message (the `target-msg-id` IRC tag).
    pub msg_id: String,
}

impl ChatMessageDeleted {
    /// Build a delete signal targeting a message id.
    pub fn new(msg_id: impl Into<String>) -> Self {
        Self {
            msg_id: msg_id.into(),
        }
    }
}

/// What rides the chat broadcast channel (one producer → TUI + Overlay Feed).
///
/// The IRC adapter fans out two kinds of chat signal on the *same* broadcast so
/// neither consumer owns the other (ADR-0001): a new [`ChatMessage`] or a
/// [`ChatMessageDeleted`] moderation signal. Consumers match on the variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatSignal {
    /// A new chat message arrived.
    Message(ChatMessage),
    /// A previously sent message was deleted by a moderator (`CLEARMSG`).
    Deleted(ChatMessageDeleted),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deleted_signal_carries_target_msg_id() {
        let deleted = ChatMessageDeleted::new("abc-123");
        assert_eq!(deleted.msg_id, "abc-123");
    }

    #[test]
    fn chat_signal_wraps_message_and_delete_variants() {
        let msg = ChatMessage::from_text("m-1", "viewer", None, "chan", "hi");
        let signal = ChatSignal::Message(msg.clone());
        assert!(matches!(signal, ChatSignal::Message(m) if m.msg_id == "m-1"));

        let signal = ChatSignal::Deleted(ChatMessageDeleted::new("m-1"));
        assert!(matches!(signal, ChatSignal::Deleted(d) if d.msg_id == "m-1"));
    }
}
