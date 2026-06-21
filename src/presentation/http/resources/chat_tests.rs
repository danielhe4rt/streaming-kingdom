//! Serde-contract tests for the chat / `chatMessageDeleted` Feed DTOs. Sliced
//! out of the former aggregate `tests.rs` (by-concern, file-size rule); wired in
//! by `#[path = "chat_tests.rs"] mod chat_tests;` in `mod.rs`. Asserting on the
//! *wire shape* (camelCase keys, `kind` discriminant) keeps the React Overlays
//! and the Rust feed in lock-step.

use super::chat::FragmentDto;
use super::*;
use crate::domain::{ChatMessage, MessageFragment};

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
fn resolved_badges_serialize_and_misses_are_dropped() {
    use crate::domain::ChatBadge;

    let msg =
        ChatMessage::from_text("b-1", "moduser", Some("#fff"), "rustlang", "hi").with_badges(vec![
            ChatBadge {
                set: "moderator".into(),
                version: "1".into(),
                url: Some("https://cdn/mod.png".into()),
            },
            // Unresolved badge (a Helix miss) — must not reach the feed.
            ChatBadge::new("glitchcon2020", "1"),
        ]);

    let value: serde_json::Value = serde_json::to_value(FeedEvent::chat(&msg)).unwrap();

    let badges = value["badges"].as_array().unwrap();
    assert_eq!(badges.len(), 1, "only resolved badges reach the feed");
    assert_eq!(badges[0]["setId"], "moderator");
    assert_eq!(badges[0]["version"], "1");
    assert_eq!(badges[0]["url"], "https://cdn/mod.png");
}

#[test]
fn message_without_badges_serializes_empty_array() {
    let msg = ChatMessage::from_text("b-2", "viewer", None, "chan", "gm");
    let value: serde_json::Value = serde_json::to_value(FeedEvent::chat(&msg)).unwrap();
    assert_eq!(value["badges"].as_array().unwrap().len(), 0);
}

#[test]
fn to_json_round_trips_fields() {
    let msg = ChatMessage::from_text("id2", "viewer", None, "chan", "gm");
    let json = FeedEvent::chat(&msg).to_json();
    assert!(json.contains("\"kind\":\"chatMessage\""));
    assert!(json.contains("\"msgId\":\"id2\""));
}

// ── M3 (extended) — single-message delete (CLEARMSG) DTO ─────────────────

#[test]
fn delete_serializes_to_feed_contract() {
    use crate::domain::ChatMessageDeleted;

    let feed = FeedEvent::deleted(&ChatMessageDeleted::new("abc-123"));
    let value: serde_json::Value = serde_json::to_value(&feed).unwrap();

    // The tagged union discriminates on `kind`, and the only payload field
    // is the camelCased `msgId` the Overlay keys its DOM nodes by.
    assert_eq!(value["kind"], "chatMessageDeleted");
    assert_eq!(value["msgId"], "abc-123");
    assert!(value.get("username").is_none(), "delete carries no body");
}

#[test]
fn delete_to_json_emits_kind_and_msg_id() {
    use crate::domain::ChatMessageDeleted;

    let json = FeedEvent::deleted(&ChatMessageDeleted::new("del-1")).to_json();
    assert!(
        json.contains("\"kind\":\"chatMessageDeleted\""),
        "got: {json}"
    );
    assert!(json.contains("\"msgId\":\"del-1\""), "got: {json}");
}

#[test]
fn from_signal_maps_both_chat_variants() {
    use crate::domain::{ChatMessageDeleted, ChatSignal};

    let msg = ChatMessage::from_text("m-1", "viewer", None, "chan", "hi");
    let feed = FeedEvent::from_signal(&ChatSignal::Message(msg));
    assert!(matches!(feed, FeedEvent::ChatMessage(_)));

    let feed = FeedEvent::from_signal(&ChatSignal::Deleted(ChatMessageDeleted::new("m-1")));
    let value = serde_json::to_value(&feed).unwrap();
    assert_eq!(value["kind"], "chatMessageDeleted");
    assert_eq!(value["msgId"], "m-1");
}
