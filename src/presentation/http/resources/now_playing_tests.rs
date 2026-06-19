//! Serde-contract tests for the `nowPlaying` Feed DTO (Coworking Overlay Now
//! Playing widget). Sliced out of the former aggregate `tests.rs` (by-concern,
//! file-size rule); wired in by `#[path = "now_playing_tests.rs"] mod
//! now_playing_tests;` in `mod.rs`.

use super::*;

#[test]
fn now_playing_serializes_to_feed_contract() {
    use crate::domain::{NowPlaying, PlaybackStatus};

    let feed = FeedEvent::now_playing(&NowPlaying {
        title: "Money".into(),
        artist: "Pink Floyd".into(),
        album: "The Dark Side of the Moon".into(),
        art_url: Some("https://i.scdn.co/image/abc".into()),
        status: PlaybackStatus::Playing,
    });
    let value: serde_json::Value = serde_json::to_value(&feed).unwrap();

    // The outer `kind` selects the now-playing branch; the camelCase fields
    // are what the React Now Playing widget renders.
    assert_eq!(value["kind"], "nowPlaying");
    assert_eq!(value["title"], "Money");
    assert_eq!(value["artist"], "Pink Floyd");
    assert_eq!(value["album"], "The Dark Side of the Moon");
    assert_eq!(value["artUrl"], "https://i.scdn.co/image/abc");
    assert_eq!(value["status"], "playing");
}

#[test]
fn now_playing_maps_paused_status() {
    use crate::domain::{NowPlaying, PlaybackStatus};

    let value = serde_json::to_value(FeedEvent::now_playing(&NowPlaying {
        title: "Time".into(),
        artist: "Pink Floyd".into(),
        album: "The Dark Side of the Moon".into(),
        art_url: None,
        status: PlaybackStatus::Paused,
    }))
    .unwrap();
    assert_eq!(value["kind"], "nowPlaying");
    assert_eq!(value["status"], "paused");
    assert!(value["artUrl"].is_null(), "no art url serializes as null");
}

#[test]
fn now_playing_cleared_is_stopped_and_empty() {
    let value: serde_json::Value =
        serde_json::to_value(FeedEvent::now_playing_cleared()).unwrap();

    assert_eq!(value["kind"], "nowPlaying");
    assert_eq!(value["status"], "stopped");
    assert_eq!(value["title"], "");
    assert_eq!(value["artist"], "");
    assert_eq!(value["album"], "");
    assert!(value["artUrl"].is_null(), "cleared carries no art url");
}

#[test]
fn now_playing_to_json_emits_kind() {
    use crate::domain::{NowPlaying, PlaybackStatus};

    let json = FeedEvent::now_playing(&NowPlaying {
        title: "Breathe".into(),
        artist: "Pink Floyd".into(),
        album: "The Dark Side of the Moon".into(),
        art_url: None,
        status: PlaybackStatus::Playing,
    })
    .to_json();
    assert!(json.contains("\"kind\":\"nowPlaying\""), "got: {json}");
    assert!(json.contains("\"status\":\"playing\""), "got: {json}");
}
