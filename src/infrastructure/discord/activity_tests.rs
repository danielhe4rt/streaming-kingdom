use super::*;
use crate::domain::{VoiceMember, VoiceRoster};

fn member(id: &str, name: &str, speaking: bool) -> VoiceMember {
    VoiceMember {
        user_id: id.into(),
        display_name: name.into(),
        avatar_url: None,
        speaking,
        self_mute: false,
        self_deaf: false,
        server_mute: false,
        server_deaf: false,
    }
}

fn roster(channel: Option<&str>, members: Vec<VoiceMember>) -> VoiceRoster {
    VoiceRoster {
        channel_id: channel.map(|c| c.to_string()),
        channel_name: channel.map(|_| "coworking".to_string()),
        members,
    }
}

#[test]
fn first_snapshot_summarizes_the_channel() {
    let next = roster(
        Some("1"),
        vec![member("a", "nina", false), member("b", "bob", false)],
    );
    let lines = diff_lines(None, &next, false);
    assert_eq!(lines.len(), 1);
    assert!(
        lines[0].contains("#coworking (2)")
            && lines[0].contains("nina")
            && lines[0].contains("bob")
    );
}

#[test]
fn channel_change_is_a_single_summary() {
    let prev = roster(Some("1"), vec![member("a", "nina", true)]);
    let next = roster(Some("2"), vec![member("a", "nina", false)]);
    let lines = diff_lines(Some(&prev), &next, true);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].starts_with("→ in #"));
}

#[test]
fn leaving_voice_logs_cleared() {
    let prev = roster(Some("1"), vec![member("a", "nina", false)]);
    let next = roster(None, vec![]);
    assert_eq!(
        diff_lines(Some(&prev), &next, true),
        vec!["← left voice (roster cleared)"]
    );
}

#[test]
fn join_and_leave_are_diffed() {
    let prev = roster(Some("1"), vec![member("a", "nina", false)]);
    let next = roster(Some("1"), vec![member("b", "bob", false)]);
    let lines = diff_lines(Some(&prev), &next, false);
    assert!(lines.iter().any(|l| l == "+ bob joined"));
    assert!(lines.iter().any(|l| l == "− nina left"));
}

#[test]
fn speaking_transitions_are_logged_when_enabled() {
    let quiet = roster(Some("1"), vec![member("a", "nina", false)]);
    let talking = roster(Some("1"), vec![member("a", "nina", true)]);
    assert!(
        diff_lines(Some(&quiet), &talking, true)
            .iter()
            .any(|l| l.contains("🎤 nina"))
    );
    assert!(
        diff_lines(Some(&talking), &quiet, true)
            .iter()
            .any(|l| l.contains("nina stopped"))
    );
}

#[test]
fn speaking_is_suppressed_when_disabled() {
    let quiet = roster(Some("1"), vec![member("a", "nina", false)]);
    let talking = roster(Some("1"), vec![member("a", "nina", true)]);
    assert!(diff_lines(Some(&quiet), &talking, false).is_empty());
}

#[test]
fn no_change_yields_no_lines() {
    let r = roster(Some("1"), vec![member("a", "nina", false)]);
    assert!(diff_lines(Some(&r), &r, true).is_empty());
}
