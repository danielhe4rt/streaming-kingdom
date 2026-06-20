//! Tests for emote-span splicing and the fragmented-body constructor.

use crate::domain::chat::{emote_cdn_url, ChatMessage, EmoteSpan, MessageFragment};

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
    let fragments = ChatMessage::split_fragments("hey Kappa there", &[EmoteSpan::new("25", 4, 9)]);
    assert_eq!(fragments, vec![text("hey "), emote("25"), text(" there")]);
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
    assert_eq!(fragments, vec![emote("25"), text(" test "), emote("25")]);
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
