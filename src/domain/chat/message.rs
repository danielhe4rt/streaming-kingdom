//! The enriched `ChatMessage` value and its text-only shaping path.

use super::{ChatBadge, MessageFragment, DEFAULT_CHAT_COLOR};

/// One user message received from a Twitch IRC channel, enriched for parity
/// rendering on the Coworking Overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    /// Stable Twitch message id (`id` IRC tag); used as the Overlay DOM key and
    /// the target of a future `ChatMessageDeleted`.
    pub msg_id: String,
    pub username: String,
    /// Author display colour as a hex string (e.g. `#FF7F50`), with a fallback
    /// when the author never set one.
    pub color: String,
    pub channel: String,
    /// The native Twitch badges worn by the author, in the tag's order. Their
    /// `url`s are resolved by the Helix badge resolver before fan-out.
    pub badges: Vec<ChatBadge>,
    /// The message body as ordered fragments (text-only for this slice).
    pub fragments: Vec<MessageFragment>,
}

impl ChatMessage {
    /// Shape a text-only [`ChatMessage`] (M1, this slice).
    ///
    /// Pure function from the raw IRC pieces to the enriched domain type:
    /// derives the display colour (falling back to [`DEFAULT_CHAT_COLOR`] when
    /// the author never set one) and wraps the body as a single ordered text
    /// fragment. Badges/emote-splitting arrive in later slices; the signature is
    /// the seam they thicken.
    pub fn from_text(
        msg_id: impl Into<String>,
        username: impl Into<String>,
        color: Option<&str>,
        channel: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        Self {
            msg_id: msg_id.into(),
            username: username.into(),
            color: resolve_color(color),
            channel: channel.into(),
            badges: Vec::new(),
            fragments: vec![MessageFragment::Text(text.into())],
        }
    }

    /// Attach the author's badges (builder-style). Used by the IRC adapter after
    /// [`from_text`](Self::from_text) shapes the body, keeping the text path the
    /// single source of the colour/fragment logic.
    pub fn with_badges(mut self, badges: Vec<ChatBadge>) -> Self {
        self.badges = badges;
        self
    }

    /// The full message body as plain text (fragments concatenated). Used by the
    /// TUI chat view, which renders text only.
    pub fn plain_text(&self) -> String {
        self.fragments
            .iter()
            .map(|f| match f {
                MessageFragment::Text(t) => t.as_str(),
                MessageFragment::Emote { .. } => "",
            })
            .collect()
    }
}

/// Resolve the author colour, falling back to [`DEFAULT_CHAT_COLOR`] when the
/// viewer never set one (or set an empty value). Shared by every constructor.
pub(super) fn resolve_color(color: Option<&str>) -> String {
    match color {
        Some(c) if !c.is_empty() => c.to_string(),
        _ => DEFAULT_CHAT_COLOR.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_text_wraps_body_in_single_text_fragment() {
        let msg =
            ChatMessage::from_text("abc-123", "danielhe4rt", Some("#FF7F50"), "rustlang", "hello world");

        assert_eq!(msg.msg_id, "abc-123");
        assert_eq!(msg.username, "danielhe4rt");
        assert_eq!(msg.channel, "rustlang");
        assert_eq!(msg.color, "#FF7F50");
        assert_eq!(msg.fragments, vec![MessageFragment::Text("hello world".into())]);
    }

    #[test]
    fn missing_color_falls_back_to_default() {
        let msg = ChatMessage::from_text("id", "viewer", None, "chan", "hi");
        assert_eq!(msg.color, DEFAULT_CHAT_COLOR);
    }

    #[test]
    fn empty_color_falls_back_to_default() {
        let msg = ChatMessage::from_text("id", "viewer", Some(""), "chan", "hi");
        assert_eq!(msg.color, DEFAULT_CHAT_COLOR);
    }

    #[test]
    fn plain_text_concatenates_fragments() {
        let msg = ChatMessage::from_text("id", "viewer", Some("#fff"), "chan", "gm everyone");
        assert_eq!(msg.plain_text(), "gm everyone");
    }

    #[test]
    fn with_badges_attaches_to_message() {
        let msg = ChatMessage::from_text("id", "viewer", Some("#fff"), "chan", "hi")
            .with_badges(vec![ChatBadge::new("subscriber", "3")]);
        assert_eq!(msg.badges, vec![ChatBadge::new("subscriber", "3")]);
    }
}
