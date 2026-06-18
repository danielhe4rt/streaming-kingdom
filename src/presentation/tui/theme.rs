use ratatui::prelude::*;

use super::state::HighlightEntry;
use crate::domain::StreamEvent;

// ---------------------------------------------------------------------------
// Highlight colors (Material Palenight) — used by mod.rs for maybe_highlight
// ---------------------------------------------------------------------------

pub const COLOR_FOLLOW: Color = Color::Rgb(0x67, 0x6e, 0x95);
pub const COLOR_SUB: Color = Color::Rgb(0xc7, 0x92, 0xea);
pub const COLOR_GIFTSUB: Color = Color::Rgb(0x89, 0xdd, 0xff);
pub const COLOR_CHEER: Color = Color::Rgb(0xff, 0xcb, 0x6b);
pub const COLOR_RAID: Color = Color::Rgb(0xf0, 0x71, 0x78);
pub const COLOR_DONATION: Color = Color::Rgb(0xf7, 0x8c, 0x6c);
pub const COLOR_BLUR_ON: Color = Color::Rgb(0xff, 0x53, 0x70);
pub const COLOR_BLUR_OFF: Color = Color::Rgb(0xc3, 0xe8, 0x8d);
pub const COLOR_WARN: Color = Color::Rgb(0xf7, 0x8c, 0x6c);
pub const COLOR_TOGGLE_ON: Color = Color::Rgb(0xc3, 0xe8, 0x8d);
pub const COLOR_TOGGLE_OFF: Color = Color::Rgb(0x67, 0x6e, 0x95);
pub const COLOR_INFO: Color = Color::Rgb(0x82, 0xaa, 0xff);
pub const COLOR_ERROR: Color = Color::Rgb(0xff, 0x53, 0x70);
pub const COLOR_WINDOW_OPEN: Color = Color::Rgb(0xc3, 0xe8, 0x8d);
pub const COLOR_WINDOW_CLOSE: Color = Color::Rgb(0x67, 0x6e, 0x95);
pub const COLOR_TITLE_CHANGE: Color = Color::Rgb(0x82, 0xaa, 0xff);
pub const COLOR_WORKSPACE: Color = Color::Rgb(0xc7, 0x92, 0xea);
pub const COLOR_MONITOR: Color = Color::Rgb(0xff, 0xcb, 0x6b);
pub const COLOR_CHAT: Color = Color::Rgb(0xc3, 0xe8, 0x8d);

pub const COLOR_CONNECTED: Color = Color::Rgb(0xc3, 0xe8, 0x8d);
pub const COLOR_INACTIVE: Color = Color::Rgb(0x67, 0x6e, 0x95);
pub const COLOR_STARTING: Color = Color::Rgb(0xff, 0xcb, 0x6b);

/// Extract highlights from stream events.
pub fn maybe_highlight(event: &StreamEvent) -> Option<HighlightEntry> {
    match event {
        StreamEvent::Follow { username } => Some(HighlightEntry {
            icon: "♥",
            text: format!("{username} followed"),
            color: COLOR_FOLLOW,
        }),
        StreamEvent::Sub {
            username,
            tier,
            months,
        } => Some(HighlightEntry {
            icon: "★",
            text: format!("{username} subbed ({tier:?}, {months}mo)"),
            color: COLOR_SUB,
        }),
        StreamEvent::GiftSub {
            username,
            tier,
            total,
        } => Some(HighlightEntry {
            icon: "🎁",
            text: format!("{username} gifted {total} subs ({tier:?})"),
            color: COLOR_GIFTSUB,
        }),
        StreamEvent::Raid {
            from_channel,
            viewers,
        } => Some(HighlightEntry {
            icon: "⚡",
            text: format!("{from_channel} raided with {viewers}"),
            color: COLOR_RAID,
        }),
        StreamEvent::Donation {
            username,
            amount_cents,
            message,
        } => Some(HighlightEntry {
            icon: "$",
            text: format!(
                "{username} donated ${:.2}: {message}",
                *amount_cents as f64 / 100.0
            ),
            color: COLOR_DONATION,
        }),
        _ => None,
    }
}
