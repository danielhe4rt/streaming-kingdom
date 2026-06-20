//! Shaping a message body that interleaves text runs and native emotes, by
//! splicing the IRC emote spans into an ordered fragment list.

use super::message::resolve_color;
use super::{emote_cdn_url, ChatMessage, EmoteSpan, MessageFragment};

impl ChatMessage {
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
        Self {
            msg_id: msg_id.into(),
            username: username.into(),
            color: resolve_color(color),
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
}

#[cfg(test)]
mod tests;
