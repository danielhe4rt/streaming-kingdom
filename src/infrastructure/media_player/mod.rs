// ---------------------------------------------------------------------------
// Media player observer – `playerctl --follow` -> watch<Option<NowPlaying>>
//
// WHY: Now Playing is ambient STATE, not an event (see
// src/infrastructure/docs/adr/0001-now-playing-observed-state.md). We acquire it
// by spawning `playerctl --follow -p spotify metadata` once and reading its
// stdout line by line: `--follow` emits a line on every track/status change (and
// the current track immediately on attach), so this is
// event-driven with no polling and no per-tick subprocess. Each line is parsed
// into a domain::NowPlaying and published on a watch channel (latest-value).
//
// Graceful degradation is the whole point: if `playerctl` is missing, exits, or
// has no player (Spotify not running), we publish `None` so consumers fall back
// to their placeholder, then back off and retry. Never panics.
// ---------------------------------------------------------------------------

use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::watch;

use crate::domain::{NowPlaying, PlaybackStatus};

/// Backoff between respawn attempts when `playerctl` is absent or exits.
const RETRY_BACKOFF: Duration = Duration::from_secs(3);

/// `playerctl --format` template. Field order is fixed and parsed positionally
/// by `parse_line`; keep the two in sync.
const FORMAT: &str = "{{status}}|{{title}}|{{artist}}|{{album}}|{{mpris:artUrl}}";

/// Spawn the media-player observer. While Spotify is Playing/Paused it publishes
/// `Some(NowPlaying)` (carrying that status); when the player is Stopped, absent,
/// or `playerctl` exits, it publishes `None`. Only publishes on change (dedupe).
pub fn spawn(
    now_playing_tx: watch::Sender<Option<NowPlaying>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // Last value we sent, so we only push to the watch on a real change.
        let mut last_sent: Option<NowPlaying> = None;

        loop {
            // (Re)spawn `playerctl --follow`. stdout piped; stderr inherited so
            // playerctl's own diagnostics stay visible in the host logs.
            // NOTE: `--follow` REQUIRES a command (here `metadata`) — without it
            // playerctl just prints its usage and exits. `metadata --follow` emits
            // the current track immediately on attach, then on each change.
            let mut child = match Command::new("playerctl")
                .args(["--follow", "-p", "spotify", "metadata", "--format", FORMAT])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::inherit())
                .spawn()
            {
                Ok(child) => child,
                Err(err) => {
                    // Most commonly: `playerctl` not installed. Degrade to the
                    // placeholder and retry after a backoff.
                    tracing::debug!(error = %err, "playerctl failed to spawn; now-playing unavailable");
                    send_if_changed(&now_playing_tx, &mut last_sent, None);
                    tokio::time::sleep(RETRY_BACKOFF).await;
                    continue;
                }
            };

            let stdout = match child.stdout.take() {
                Some(stdout) => stdout,
                None => {
                    tracing::debug!("playerctl stdout unavailable; retrying");
                    send_if_changed(&now_playing_tx, &mut last_sent, None);
                    let _ = child.kill().await;
                    tokio::time::sleep(RETRY_BACKOFF).await;
                    continue;
                }
            };

            tracing::info!("media_player observer attached to playerctl --follow -p spotify metadata");

            let mut lines = BufReader::new(stdout).lines();
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }
                        // A malformed line yields None and is simply skipped —
                        // we do NOT clear the current track over a parse glitch.
                        if let Some(now_playing) = parse_line(line) {
                            let next = match now_playing.status {
                                PlaybackStatus::Stopped => None,
                                _ => Some(now_playing),
                            };
                            send_if_changed(&now_playing_tx, &mut last_sent, next);
                        }
                    }
                    // stdout closed (Spotify quit / playerctl errored out).
                    Ok(None) => {
                        tracing::debug!("playerctl stdout closed; player gone");
                        break;
                    }
                    Err(err) => {
                        tracing::debug!(error = %err, "error reading playerctl stdout");
                        break;
                    }
                }
            }

            // The follow stream ended: no player. Fall back to placeholder,
            // reap the child, and retry after a backoff.
            send_if_changed(&now_playing_tx, &mut last_sent, None);
            let _ = child.kill().await;
            let _ = child.wait().await;
            tokio::time::sleep(RETRY_BACKOFF).await;
        }
    })
}

/// Send `next` on the watch only when it differs from the last sent value, and
/// update `last_sent`. Keeps consumers from re-rendering on duplicate frames.
fn send_if_changed(
    tx: &watch::Sender<Option<NowPlaying>>,
    last_sent: &mut Option<NowPlaying>,
    next: Option<NowPlaying>,
) {
    if *last_sent == next {
        return;
    }
    *last_sent = next.clone();
    // A send error means all receivers dropped; the task will keep running
    // harmlessly until the process exits.
    let _ = tx.send(next);
}

/// Parse one `playerctl --format` line into a `NowPlaying`.
///
/// Expected shape: `status|title|artist|album|artUrl` (see `FORMAT`). Returns
/// `None` for an empty or malformed line (fewer than the 5 expected fields).
/// Status maps "Playing"/"Paused" to their variants, anything else to Stopped.
/// An empty `artUrl` becomes `None`.
fn parse_line(line: &str) -> Option<NowPlaying> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    // splitn(5) so a `|` inside an album/title never eats later fields; the
    // last field (artUrl) keeps any literal `|` it might contain.
    let mut parts = line.splitn(5, '|');
    let status = parts.next()?;
    let title = parts.next()?;
    let artist = parts.next()?;
    let album = parts.next()?;
    let art_url_raw = parts.next()?;

    let status = match status.trim() {
        "Playing" => PlaybackStatus::Playing,
        "Paused" => PlaybackStatus::Paused,
        _ => PlaybackStatus::Stopped,
    };

    let art_url_trimmed = art_url_raw.trim();
    let art_url = if art_url_trimmed.is_empty() {
        None
    } else {
        Some(art_url_trimmed.to_string())
    };

    Some(NowPlaying {
        title: title.trim().to_string(),
        artist: artist.trim().to_string(),
        album: album.trim().to_string(),
        art_url,
        status,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_playing_line() {
        let np = parse_line(
            "Playing|Bohemian Rhapsody|Queen|A Night at the Opera|https://i.scdn.co/image/abc",
        )
        .expect("playing line should parse");
        assert_eq!(np.status, PlaybackStatus::Playing);
        assert_eq!(np.title, "Bohemian Rhapsody");
        assert_eq!(np.artist, "Queen");
        assert_eq!(np.album, "A Night at the Opera");
        assert_eq!(np.art_url.as_deref(), Some("https://i.scdn.co/image/abc"));
    }

    #[test]
    fn parses_a_paused_line() {
        let np = parse_line("Paused|Clocks|Coldplay|A Rush of Blood to the Head|https://art")
            .expect("paused line should parse");
        assert_eq!(np.status, PlaybackStatus::Paused);
        assert_eq!(np.title, "Clocks");
    }

    #[test]
    fn empty_art_url_becomes_none() {
        let np = parse_line("Playing|Song|Artist|Album|")
            .expect("line with empty artUrl should parse");
        assert_eq!(np.art_url, None);
    }

    #[test]
    fn unknown_status_maps_to_stopped() {
        let np = parse_line("Stopped|||| ").expect("stopped line should parse");
        assert_eq!(np.status, PlaybackStatus::Stopped);
    }

    #[test]
    fn malformed_line_returns_none() {
        // Fewer than 5 fields -> None.
        assert_eq!(parse_line("Playing|Song|Artist"), None);
        // Empty line -> None.
        assert_eq!(parse_line("   "), None);
    }
}
