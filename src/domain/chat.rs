// ---------------------------------------------------------------------------
// Chat — enriched message received from a Twitch IRC channel
// ---------------------------------------------------------------------------

/// Default display colour used when a viewer has never set a Twitch chat colour.
/// Matches Twitch's neutral grey so the Overlay always renders a visible nick.
pub const DEFAULT_CHAT_COLOR: &str = "#9147ff";

/// One ordered piece of a [`ChatMessage`] body.
///
/// In this slice the body is always a single [`MessageFragment::Text`]; the
/// `Emote` variant exists so later slices can splice native Twitch emotes into
/// the same ordered stream the TUI and Overlay both render from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageFragment {
    /// A run of plain text.
    Text(String),
    /// A single native Twitch emote: its id and the CDN url to render.
    Emote { id: String, url: String },
}

/// One user message received from a Twitch IRC channel, enriched for parity
/// rendering on the Chat Overlay.
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
        let color = match color {
            Some(c) if !c.is_empty() => c.to_string(),
            _ => DEFAULT_CHAT_COLOR.to_string(),
        };
        Self {
            msg_id: msg_id.into(),
            username: username.into(),
            color,
            channel: channel.into(),
            fragments: vec![MessageFragment::Text(text.into())],
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_text_wraps_body_in_single_text_fragment() {
        let msg = ChatMessage::from_text("abc-123", "danielhe4rt", Some("#FF7F50"), "rustlang", "hello world");

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
}
