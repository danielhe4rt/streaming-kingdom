//! Driving the OBS blur on/off as the set of open windows changes.

use std::collections::HashMap;

use tokio::sync::mpsc;

use crate::application::PrivacyConfig;
use crate::infrastructure::obs;

use super::messages::PrivacyStatus;
use super::sensitive_match::is_sensitive;

/// Re-evaluate the current windows and toggle blur if the sensitivity changed.
///
/// Works for both the initial evaluation (with `blur_active = false`) and every
/// subsequent window event: it enables blur on the first sensitive window and
/// disables it once none remain, emitting the matching status each time.
pub(super) async fn apply_blur_transition(
    windows: &HashMap<String, String>,
    config: &PrivacyConfig,
    obs_client: &Option<obws::Client>,
    capture_source: &str,
    status_tx: &mpsc::Sender<PrivacyStatus>,
    blur_active: &mut bool,
) {
    let has_sensitive = windows
        .values()
        .any(|t| is_sensitive(t, &config.sensitive_patterns));

    if has_sensitive && !*blur_active {
        let title = windows
            .values()
            .find(|t| is_sensitive(t, &config.sensitive_patterns))
            .cloned()
            .unwrap_or_default();
        if let Some(client) = obs_client {
            obs::enable_blur(client, capture_source).await;
        }
        *blur_active = true;
        let _ = status_tx.send(PrivacyStatus::BlurEnabled { title }).await;
    } else if !has_sensitive && *blur_active {
        if let Some(client) = obs_client {
            obs::disable_blur(client, capture_source).await;
        }
        *blur_active = false;
        let _ = status_tx.send(PrivacyStatus::BlurDisabled).await;
    }
}

/// Remove any active blur on shutdown so OBS isn't left blurred after stop.
pub(super) async fn teardown_blur(
    obs_client: &Option<obws::Client>,
    capture_source: &str,
    blur_active: bool,
) {
    if blur_active
        && let Some(client) = obs_client
    {
        obs::disable_blur(client, capture_source).await;
    }
}
