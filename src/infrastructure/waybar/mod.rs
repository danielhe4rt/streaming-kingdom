pub mod config;
mod events;
mod style;

use std::path::PathBuf;
use std::{fs, io};

use tokio::process::Command;
use tokio::sync::broadcast;

use crate::domain::StreamEvent;

// ---------------------------------------------------------------------------
// Cache directory layout (for stream_data.json — still used by event_writer)
// ---------------------------------------------------------------------------

fn cache_dir() -> io::Result<PathBuf> {
    let base = dirs::cache_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no cache directory available"))?;
    Ok(base.join("streams-toolkit"))
}

fn data_file_path() -> io::Result<PathBuf> {
    Ok(cache_dir()?.join("stream_data.json"))
}

// ---------------------------------------------------------------------------
// Scripts directory layout (for external Python scripts)
// ---------------------------------------------------------------------------

fn scripts_dir() -> io::Result<PathBuf> {
    let config = dirs::config_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no config directory available"))?;
    Ok(config.join("streams-toolkit").join("scripts"))
}

pub fn stream_events_script_path() -> io::Result<PathBuf> {
    Ok(scripts_dir()?.join("stream_events.py"))
}

/// Ensure the Python script exists for waybar exec commands.
const STREAM_EVENTS_SCRIPT: &str = include_str!("../../../scripts/stream_events.py");

pub fn ensure_scripts() -> io::Result<()> {
    let script_path = stream_events_script_path()?;
    if !script_path.exists() {
        if let Some(parent) = script_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&script_path, STREAM_EVENTS_SCRIPT)?;
        // Make executable on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script_path, perms)?;
        }
        tracing::info!(
            "installed stream_events.py script to {}",
            script_path.display()
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Ensure the data file exists so waybar custom modules don't fail
// ---------------------------------------------------------------------------

pub fn ensure_data_file() -> io::Result<()> {
    let data_path = data_file_path()?;
    if !data_path.exists() {
        if let Some(parent) = data_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&data_path, SEED_EVENTS)?;
        tracing::info!("seeded stream data file with sample events");
    }
    Ok(())
}

/// Sample events so the waybar bar has something to display before real
/// Twitch events come in. Overwritten as soon as the first real event arrives.
const SEED_EVENTS: &str = r#"[
  {"event_type":"follow","username":"rustacean42"},
  {"event_type":"newSubscriber","username":"ferris_fan"},
  {"event_type":"cheer","username":"bit_donor","amount":"500"},
  {"event_type":"giftSubscriber","username":"generous_viewer","amount":"5"},
  {"event_type":"raid","username":"other_streamer","amount":"150"},
  {"event_type":"recurringSubscriber","username":"loyal_sub","amount":"12"},
  {"event_type":"donation","username":"tipper123","amount":"25.00"},
  {"event_type":"follow","username":"new_viewer99"}
]"#;

// ---------------------------------------------------------------------------
// Toggle ON/OFF — merge config/style then restart Omarchy's waybar
// ---------------------------------------------------------------------------

/// Enable the stream bottom bar: merge config + style, restart waybar.
pub async fn enable(output: &str) -> io::Result<()> {
    ensure_data_file()?;
    ensure_scripts()?;
    config::add_stream_bar(output)?;
    style::add_stream_css()?;
    restart_waybar().await
}

/// Disable the stream bottom bar: remove config + style, restart waybar.
pub async fn disable() -> io::Result<()> {
    config::remove_stream_bar()?;
    style::remove_stream_css()?;
    restart_waybar().await
}

/// Restart Omarchy's single waybar process via `omarchy-restart-waybar`.
async fn restart_waybar() -> io::Result<()> {
    tracing::info!("restarting waybar via omarchy-restart-waybar");

    let status = Command::new("omarchy-restart-waybar")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await?;

    if status.success() {
        tracing::info!("waybar restarted successfully");
    } else {
        tracing::warn!("omarchy-restart-waybar exited with status: {status}");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Event listener — writes stream_data.json on each event
// ---------------------------------------------------------------------------

const MAX_EVENTS: usize = 15;

/// Listen for stream events and update the data file that waybar reads.
/// Runs until the broadcast channel closes.
pub async fn event_writer(mut event_rx: broadcast::Receiver<StreamEvent>) {
    let data_path = match data_file_path() {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("cannot determine data file path: {e}");
            return;
        }
    };

    let mut recent: Vec<events::WaybarEvent> = Vec::with_capacity(MAX_EVENTS);

    loop {
        match event_rx.recv().await {
            Ok(stream_event) => {
                if let Some(wb_event) = events::WaybarEvent::from_stream_event(&stream_event) {
                    recent.insert(0, wb_event);
                    recent.truncate(MAX_EVENTS);
                    if let Err(e) = events::write_data_file(&data_path, &recent) {
                        tracing::warn!("failed to write stream data: {e}");
                    }
                }
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("waybar event writer lagged by {n} events");
            }
            Err(broadcast::error::RecvError::Closed) => {
                tracing::info!("event channel closed, waybar event writer stopping");
                break;
            }
        }
    }
}
