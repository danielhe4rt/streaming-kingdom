// ---------------------------------------------------------------------------
// Chat — enriched message received from a Twitch IRC channel
// ---------------------------------------------------------------------------

/// Default display colour used when a viewer has never set a Twitch chat colour.
/// Matches Twitch's neutral grey so the Overlay always renders a visible nick.
pub const DEFAULT_CHAT_COLOR: &str = "#9147ff";

/// One ordered piece of a [`ChatMessage`] body.
///
/// A body is a flat, ordered stream of plain-text runs interleaved with single
/// native Twitch emotes. The TUI renders only the text runs; the Coworking Overlay
/// renders the text runs as spans and the emotes as inline `<img>`s in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageFragment {
    /// A run of plain text.
    Text(String),
    /// A single native Twitch emote: its id and the CDN url to render.
    Emote { id: String, url: String },
}

/// One native Twitch emote occurrence in a message body (M1).
///
/// Carries the emote id and the half-open `[start, end)` **character** range it
/// occupies in the original message text — exactly the shape the IRC `emotes`
/// tag yields (already char-indexed and de-bugged by `twitch-irc`). Used by
/// [`ChatMessage::split_fragments`] to splice emotes into the ordered body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmoteSpan {
    /// The Twitch emote id (e.g. `25` for Kappa, or `301512758_TK`).
    pub id: String,
    /// Inclusive char index where the emote begins.
    pub start: usize,
    /// Exclusive char index where the emote ends.
    pub end: usize,
}

impl EmoteSpan {
    /// Build a span from an emote id and its half-open char range.
    pub fn new(id: impl Into<String>, start: usize, end: usize) -> Self {
        Self {
            id: id.into(),
            start,
            end,
        }
    }
}

/// Derive the CDN url for a native Twitch emote from its id (no API call).
///
/// Uses the Helix emote CDN v2 layout, requesting the static (`default`) dark
/// 3.0 (largest) variant so the Overlay always has a crisp image to scale down.
pub fn emote_cdn_url(id: &str) -> String {
    format!("https://static-cdn.jtvnw.net/emoticons/v2/{id}/default/dark/3.0")
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

    /// Shape a [`ChatMessage`] whose body interleaves text runs and native
    /// Twitch emotes (M1).
    ///
    /// Same colour/fallback rules as [`from_text`](Self::from_text), but the body
    /// is split by the emote `char_range`s from the IRC tags via
    /// [`split_fragments`](Self::split_fragments) instead of being a single text
    /// run. Pure: it adds no badges (use [`with_badges`](Self::with_badges)).
    pub fn from_fragments(
        msg_id: impl Into<String>,
        username: impl Into<String>,
        color: Option<&str>,
        channel: impl Into<String>,
        text: &str,
        emotes: &[EmoteSpan],
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
            fragments: Self::split_fragments(text, emotes),
        }
    }

    /// Split a message body into ordered [`MessageFragment`]s by its emote spans
    /// (M1, pure).
    ///
    /// Walks the text by **character** index (the IRC ranges are char-based, not
    /// byte-based, so multi-byte text stays aligned), emitting a `Text` run for
    /// the gap before each emote and an `Emote` for the emote itself. Spans are
    /// sorted by start so callers need not pre-sort, and empty text runs (from
    /// adjacent emotes or an emote at the start/end) are dropped so the body is
    /// a clean interleave. An out-of-bounds or inverted span is skipped rather
    /// than panicking — a malformed tag never drops the whole message.
    pub fn split_fragments(text: &str, emotes: &[EmoteSpan]) -> Vec<MessageFragment> {
        let chars: Vec<char> = text.chars().collect();

        if emotes.is_empty() {
            return if chars.is_empty() {
                Vec::new()
            } else {
                vec![MessageFragment::Text(text.to_string())]
            };
        }

        let mut spans: Vec<&EmoteSpan> = emotes.iter().collect();
        spans.sort_by_key(|e| e.start);

        let mut fragments = Vec::new();
        let mut cursor = 0usize;

        let push_text = |fragments: &mut Vec<MessageFragment>, from: usize, to: usize| {
            if to > from {
                let run: String = chars[from..to].iter().collect();
                fragments.push(MessageFragment::Text(run));
            }
        };

        for span in spans {
            // Skip a span that overlaps an earlier one, is inverted, or runs past
            // the end of the text — defensive against a malformed IRC tag.
            let valid = span.start >= cursor && span.end > span.start && span.end <= chars.len();
            if !valid {
                continue;
            }

            push_text(&mut fragments, cursor, span.start);
            fragments.push(MessageFragment::Emote {
                id: span.id.clone(),
                url: emote_cdn_url(&span.id),
            });
            cursor = span.end;
        }

        push_text(&mut fragments, cursor, chars.len());
        fragments
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

    // ── M1 — emote CDN url + fragment splitting ──────────────────────────

    fn text(s: &str) -> MessageFragment {
        MessageFragment::Text(s.into())
    }

    fn emote(id: &str) -> MessageFragment {
        MessageFragment::Emote {
            id: id.into(),
            url: emote_cdn_url(id),
        }
    }

    #[test]
    fn emote_cdn_url_uses_helix_v2_layout() {
        assert_eq!(
            emote_cdn_url("25"),
            "https://static-cdn.jtvnw.net/emoticons/v2/25/default/dark/3.0"
        );
        // Channel-points modified emote ids are non-numeric but format the same.
        assert_eq!(
            emote_cdn_url("301512758_TK"),
            "https://static-cdn.jtvnw.net/emoticons/v2/301512758_TK/default/dark/3.0"
        );
    }

    #[test]
    fn split_no_emotes_yields_single_text_run() {
        assert_eq!(
            ChatMessage::split_fragments("hello world", &[]),
            vec![text("hello world")]
        );
    }

    #[test]
    fn split_empty_text_yields_no_fragments() {
        assert_eq!(ChatMessage::split_fragments("", &[]), Vec::new());
    }

    #[test]
    fn split_emote_in_the_middle_interleaves_text_emote_text() {
        // "hey Kappa there" — Kappa occupies chars [4, 9).
        let fragments =
            ChatMessage::split_fragments("hey Kappa there", &[EmoteSpan::new("25", 4, 9)]);
        assert_eq!(
            fragments,
            vec![text("hey "), emote("25"), text(" there")]
        );
    }

    #[test]
    fn split_emote_at_start_drops_leading_empty_text() {
        // "Kappa hi" — emote leads, so no empty text run before it.
        let fragments = ChatMessage::split_fragments("Kappa hi", &[EmoteSpan::new("25", 0, 5)]);
        assert_eq!(fragments, vec![emote("25"), text(" hi")]);
    }

    #[test]
    fn split_emote_at_end_drops_trailing_empty_text() {
        // "hi Kappa" — emote trails, so no empty text run after it.
        let fragments = ChatMessage::split_fragments("hi Kappa", &[EmoteSpan::new("25", 3, 8)]);
        assert_eq!(fragments, vec![text("hi "), emote("25")]);
    }

    #[test]
    fn split_adjacent_emotes_have_no_text_between_them() {
        // "KappaKeepo" — two emotes back-to-back, no gap text.
        let fragments = ChatMessage::split_fragments(
            "KappaKeepo",
            &[EmoteSpan::new("25", 0, 5), EmoteSpan::new("1902", 5, 10)],
        );
        assert_eq!(fragments, vec![emote("25"), emote("1902")]);
    }

    #[test]
    fn split_emote_only_message_is_a_single_emote() {
        let fragments = ChatMessage::split_fragments("Kappa", &[EmoteSpan::new("25", 0, 5)]);
        assert_eq!(fragments, vec![emote("25")]);
    }

    #[test]
    fn split_orders_unsorted_repeated_emotes_by_position() {
        // "Kappa test Keepo" with spans given out of order; result follows text
        // order, and a repeated emote keeps its CDN url each time.
        let fragments = ChatMessage::split_fragments(
            "Kappa test Kappa",
            &[EmoteSpan::new("25", 11, 16), EmoteSpan::new("25", 0, 5)],
        );
        assert_eq!(
            fragments,
            vec![emote("25"), text(" test "), emote("25")]
        );
    }

    #[test]
    fn split_uses_char_indices_for_multibyte_text() {
        // "👉 Kappa" — the emoji is one char but four bytes; the Kappa range is
        // char-based [2, 7), so byte-indexing would mis-slice. The leading "👉 "
        // text run must stay intact.
        let fragments = ChatMessage::split_fragments("👉 Kappa", &[EmoteSpan::new("25", 2, 7)]);
        assert_eq!(fragments, vec![text("👉 "), emote("25")]);
    }

    #[test]
    fn split_skips_out_of_bounds_span() {
        // A span past the end of the text is dropped; the text survives whole.
        let fragments = ChatMessage::split_fragments("hi", &[EmoteSpan::new("25", 0, 99)]);
        assert_eq!(fragments, vec![text("hi")]);
    }

    #[test]
    fn from_fragments_splits_body_and_sets_color() {
        let msg = ChatMessage::from_fragments(
            "m-1",
            "randers",
            Some("#19E6E6"),
            "pajlada",
            "hey Kappa",
            &[EmoteSpan::new("25", 4, 9)],
        );
        assert_eq!(msg.color, "#19E6E6");
        assert_eq!(msg.fragments, vec![text("hey "), emote("25")]);
        assert!(msg.badges.is_empty());
    }

    #[test]
    fn from_fragments_plain_text_drops_emotes() {
        // The TUI view reads plain_text(); emotes contribute no text.
        let msg = ChatMessage::from_fragments(
            "m-2",
            "viewer",
            None,
            "chan",
            "hey Kappa there",
            &[EmoteSpan::new("25", 4, 9)],
        );
        assert_eq!(msg.plain_text(), "hey  there");
    }

    #[test]
    fn with_badges_attaches_to_message() {
        let msg = ChatMessage::from_text("id", "viewer", Some("#fff"), "chan", "hi")
            .with_badges(vec![ChatBadge::new("subscriber", "3")]);
        assert_eq!(msg.badges, vec![ChatBadge::new("subscriber", "3")]);
    }

    // ── single-message moderation (CLEARMSG) ─────────────────────────────────

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
