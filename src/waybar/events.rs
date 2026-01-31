use std::io;
use std::path::Path;

use serde::Serialize;

use crate::app::{StreamEvent, SubTier};

// ---------------------------------------------------------------------------
// Waybar-specific event representation (written to stream_data.json)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct WaybarEvent {
    /// Event type key matching the SE overlay naming.
    pub event_type: String,
    /// Display name of the user who triggered the event.
    pub username: String,
    /// Optional numeric amount (bits, months, donation value, viewer count).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
}

impl WaybarEvent {
    /// Convert an internal `StreamEvent` into a `WaybarEvent`.
    ///
    /// Returns `None` for events that shouldn't appear in the bar
    /// (e.g. `ViewerCountUpdate`).
    pub fn from_stream_event(ev: &StreamEvent) -> Option<Self> {
        match ev {
            StreamEvent::Follow { username } => Some(Self {
                event_type: "follow".into(),
                username: username.clone(),
                amount: None,
            }),

            StreamEvent::Sub {
                username,
                tier,
                months,
            } => {
                let event_type = classify_sub(*tier, *months);
                let amount = if *months > 1 {
                    Some(months.to_string())
                } else {
                    None
                };
                Some(Self {
                    event_type,
                    username: username.clone(),
                    amount,
                })
            }

            StreamEvent::Donation {
                username,
                amount_cents,
                ..
            } => Some(Self {
                event_type: "donation".into(),
                username: username.clone(),
                amount: Some(format!("{:.2}", *amount_cents as f64 / 100.0)),
            }),

            StreamEvent::Cheer {
                username, bits, ..
            } => Some(Self {
                event_type: "cheer".into(),
                username: username.clone(),
                amount: Some(bits.to_string()),
            }),

            StreamEvent::Raid {
                from_channel,
                viewers,
            } => Some(Self {
                event_type: "raid".into(),
                username: from_channel.clone(),
                amount: Some(viewers.to_string()),
            }),

            // Viewer count updates don't show in the event bar
            StreamEvent::ViewerCountUpdate { .. } => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Subscriber type classification (SE overlay logic)
// ---------------------------------------------------------------------------

/// Determine the SE-style subscriber event type.
///
/// From the SE overlay source:
/// - `bulkGifted` → giftSubscriber (handled at websocket layer, not here)
/// - `gifted` → directGiftSubscriber (handled at websocket layer, not here)
/// - `months > 1` → recurringSubscriber
/// - else → newSubscriber
///
/// Gift subtypes would be classified at the websocket ingest layer where
/// the raw SE payload is available. Here we use tier + months to distinguish
/// new vs recurring.
fn classify_sub(_tier: SubTier, months: u32) -> String {
    if months > 1 {
        "recurringSubscriber".into()
    } else {
        "newSubscriber".into()
    }
}

// ---------------------------------------------------------------------------
// Data file I/O
// ---------------------------------------------------------------------------

/// Write the recent events list to the JSON data file that waybar reads.
pub fn write_data_file(path: &Path, events: &[WaybarEvent]) -> io::Result<()> {
    let json = serde_json::to_string(events).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, format!("json serialize: {e}"))
    })?;
    std::fs::write(path, json)
}
