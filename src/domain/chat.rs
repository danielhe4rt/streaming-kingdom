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

/// One native Twitch chat badge worn by a message author (e.g.
/// `subscriber/12`, `moderator/1`).
///
/// The IRC adapter captures the `set`/`version` pair from the message `badges`
/// tag (M1). The CDN `url` is filled in once by the Helix badge resolver (M2);
/// it stays `None` when the resolver has no entry for the pair (a miss), which
/// the Overlay renders as no image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatBadge {
    /// The badge set id (the `name` in the IRC tag), e.g. `subscriber`.
    pub set: String,
    /// The badge version within the set, e.g. `12`.
    pub version: String,
    /// The resolved CDN image url, or `None` when unresolved (a resolver miss).
    pub url: Option<String>,
}

impl ChatBadge {
    /// Build an unresolved badge from a `set`/`version` pair (M1, IRC tags).
    pub fn new(set: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            set: set.into(),
            version: version.into(),
            url: None,
        }
    }
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
        let color = match color {
            Some(c) if !c.is_empty() => c.to_string(),
            _ => DEFAULT_CHAT_COLOR.to_string(),
        };
        Self {
            msg_id: msg_id.into(),
            username: username.into(),
            color,
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

    /// Parse the raw IRC `badges` tag into `set/version` pairs (M1, pure).
    ///
    /// The tag is a comma-separated list of `set/version` entries (e.g.
    /// `moderator/1,subscriber/12`); an empty tag yields no badges. Malformed
    /// entries (missing the `/`, or an empty set/version) are skipped so a
    /// single bad pair never drops the whole message. The returned badges are
    /// unresolved — the Helix resolver fills their urls afterwards.
    pub fn parse_badges(tag: &str) -> Vec<ChatBadge> {
        tag.split(',')
            .filter_map(|entry| {
                let entry = entry.trim();
                if entry.is_empty() {
                    return None;
                }
                let (set, version) = entry.split_once('/')?;
                if set.is_empty() || version.is_empty() {
                    return None;
                }
                Some(ChatBadge::new(set, version))
            })
            .collect()
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

    // ── M1 — badge parsing ────────────────────────────────────────────────

    #[test]
    fn parse_badges_splits_set_and_version_pairs() {
        let badges = ChatMessage::parse_badges("moderator/1,subscriber/12");
        assert_eq!(
            badges,
            vec![
                ChatBadge::new("moderator", "1"),
                ChatBadge::new("subscriber", "12"),
            ]
        );
        // Parsed pairs start unresolved; the resolver fills urls later.
        assert!(badges.iter().all(|b| b.url.is_none()));
    }

    #[test]
    fn parse_badges_empty_tag_yields_none() {
        assert!(ChatMessage::parse_badges("").is_empty());
    }

    #[test]
    fn parse_badges_skips_malformed_entries() {
        // `broadcaster` has no `/version`, `/2` has no set, `subscriber/` has no
        // version — all skipped — leaving only the well-formed `vip/1`.
        let badges = ChatMessage::parse_badges("broadcaster,/2,subscriber/,vip/1");
        assert_eq!(badges, vec![ChatBadge::new("vip", "1")]);
    }

    #[test]
    fn with_badges_attaches_to_message() {
        let msg = ChatMessage::from_text("id", "viewer", Some("#fff"), "chan", "hi")
            .with_badges(vec![ChatBadge::new("subscriber", "3")]);
        assert_eq!(msg.badges, vec![ChatBadge::new("subscriber", "3")]);
    }
}
