// ---------------------------------------------------------------------------
// Human-readable activity diff for the TUI event log.
//
// Each roster snapshot is compared with the previous one so the TUI shows only
// the *transitions* the streamer cares about while debugging — channel changes,
// joins/leaves, and who started/stopped talking — instead of the full roster on
// every frame. Pure (no I/O); `bridge.rs` sends each line as DiscordStatus.
// ---------------------------------------------------------------------------

use crate::domain::VoiceRoster;

/// Log lines describing what changed between `prev` and `next`. A channel change
/// (or first snapshot) yields a single summary line; otherwise per-member
/// join/leave and speaking transitions.
pub fn diff_lines(prev: Option<&VoiceRoster>, next: &VoiceRoster) -> Vec<String> {
    let mut lines = Vec::new();

    if prev.and_then(|r| r.channel_id.as_deref()) != next.channel_id.as_deref() {
        match &next.channel_id {
            Some(_) => {
                let names: Vec<&str> = next.members.iter().map(|m| m.display_name.as_str()).collect();
                let channel = next.channel_name.as_deref().unwrap_or("voice");
                lines.push(format!("→ in #{channel} ({}): {}", next.members.len(), names.join(", ")));
            }
            None => lines.push("← left voice (roster cleared)".to_string()),
        }
        return lines; // channel change → summary only, skip per-member noise
    }

    let prev = match prev {
        Some(prev) => prev,
        None => return lines,
    };

    for m in &next.members {
        if !prev.members.iter().any(|p| p.user_id == m.user_id) {
            lines.push(format!("+ {} joined", m.display_name));
        }
    }
    for p in &prev.members {
        if !next.members.iter().any(|m| m.user_id == p.user_id) {
            lines.push(format!("− {} left", p.display_name));
        }
    }
    for m in &next.members {
        let was_speaking = prev.members.iter().find(|p| p.user_id == m.user_id).is_some_and(|p| p.speaking);
        if m.speaking && !was_speaking {
            lines.push(format!("🎤 {} speaking", m.display_name));
        } else if !m.speaking && was_speaking {
            lines.push(format!("   {} stopped", m.display_name));
        }
    }
    lines
}

#[cfg(test)]
#[path = "activity_tests.rs"]
mod activity_tests;
