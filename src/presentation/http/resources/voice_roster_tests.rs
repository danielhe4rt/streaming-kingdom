//! Serde-contract tests for the `voiceRoster` Feed DTO (Discord Voice Roster
//! widget). Sliced out of the former aggregate `tests.rs` (by-concern, file-size
//! rule); wired in by `#[path = "voice_roster_tests.rs"] mod voice_roster_tests;`
//! in `mod.rs`.

use super::*;

#[test]
fn voice_roster_serializes_to_feed_contract() {
    use crate::domain::{VoiceMember, VoiceRoster};

    let feed = FeedEvent::voice_roster(&VoiceRoster {
        channel_id: Some("123".into()),
        channel_name: Some("coworking".into()),
        members: vec![VoiceMember {
            user_id: "42".into(),
            display_name: "nina".into(),
            avatar_url: Some("https://cdn.discordapp.com/avatars/42/abc.png".into()),
            speaking: true,
            self_mute: false,
            self_deaf: false,
            server_mute: false,
            server_deaf: false,
        }],
    });
    let value: serde_json::Value = serde_json::to_value(&feed).unwrap();

    // The outer `kind` selects the voice-roster branch; the camelCase member
    // fields are exactly what the React Voice Roster widget renders.
    assert_eq!(value["kind"], "voiceRoster");
    assert_eq!(value["channelId"], "123");
    assert_eq!(value["channelName"], "coworking");
    let member = &value["members"][0];
    assert_eq!(member["userId"], "42");
    assert_eq!(member["displayName"], "nina");
    assert_eq!(
        member["avatarUrl"],
        "https://cdn.discordapp.com/avatars/42/abc.png"
    );
    assert_eq!(member["speaking"], true);
    assert_eq!(member["selfMute"], false);
    assert_eq!(member["selfDeaf"], false);
    assert_eq!(member["serverMute"], false);
    assert_eq!(member["serverDeaf"], false);
}

#[test]
fn voice_roster_cleared_has_null_channel_and_empty_members() {
    use crate::domain::VoiceRoster;

    // The default roster (between channels / Discord disconnected) carries a
    // null channelId/channelName and no members, so the widget clears itself.
    let value: serde_json::Value =
        serde_json::to_value(FeedEvent::voice_roster(&VoiceRoster::default())).unwrap();

    assert_eq!(value["kind"], "voiceRoster");
    assert!(
        value["channelId"].is_null(),
        "no channel serializes as null"
    );
    assert!(
        value["channelName"].is_null(),
        "no channel serializes as null"
    );
    assert_eq!(value["members"].as_array().unwrap().len(), 0);
}

#[test]
fn voice_member_without_avatar_serializes_null() {
    use crate::domain::{VoiceMember, VoiceRoster};

    let value = serde_json::to_value(FeedEvent::voice_roster(&VoiceRoster {
        channel_id: Some("9".into()),
        channel_name: None,
        members: vec![VoiceMember {
            user_id: "7".into(),
            display_name: "bob".into(),
            avatar_url: None,
            speaking: false,
            self_mute: true,
            self_deaf: true,
            server_mute: false,
            server_deaf: false,
        }],
    }))
    .unwrap();
    let member = &value["members"][0];
    assert!(
        member["avatarUrl"].is_null(),
        "no avatar serializes as null"
    );
    assert_eq!(member["selfMute"], true);
    assert_eq!(member["selfDeaf"], true);
}

#[test]
fn voice_roster_to_json_emits_kind() {
    use crate::domain::VoiceRoster;

    let json = FeedEvent::voice_roster(&VoiceRoster::default()).to_json();
    assert!(json.contains("\"kind\":\"voiceRoster\""), "got: {json}");
}
