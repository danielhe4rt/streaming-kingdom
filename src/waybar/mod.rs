pub mod config;
mod events;
mod style;

use std::path::PathBuf;
use std::{fs, io};

use tokio::process::Command;
use tokio::sync::broadcast;

use crate::app::StreamEvent;

// ---------------------------------------------------------------------------
// Cache directory layout (for stream_data.json — still used by event_writer)
// ---------------------------------------------------------------------------

fn cache_dir() -> io::Result<PathBuf> {
    let base = dirs::cache_dir().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "no cache directory available")
    })?;
    Ok(base.join("streams-toolkit"))
}

fn data_file_path() -> io::Result<PathBuf> {
    Ok(cache_dir()?.join("stream_data.json"))
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
        fs::write(&data_path, "[]")?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Toggle ON/OFF — merge config/style then restart Omarchy's waybar
// ---------------------------------------------------------------------------

/// Enable the stream bottom bar: merge config + style, restart waybar.
pub async fn enable(output: &str) -> io::Result<()> {
    ensure_data_file()?;
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

/// Restart Omarchy's single waybar process.
/// Uses the standard pattern: `pkill -x waybar && setsid uwsm-app -- waybar`.
async fn restart_waybar() -> io::Result<()> {
    tracing::info!("restarting waybar");

    // Kill existing waybar(s)
    let _ = Command::new("pkill")
        .arg("-x")
        .arg("waybar")
        .output()
        .await;

    // Small delay to let the process fully exit
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Launch waybar through uwsm-app (Omarchy's pattern) detached via setsid
    let status = Command::new("setsid")
        .arg("uwsm-app")
        .arg("--")
        .arg("waybar")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await?;

    if status.success() {
        tracing::info!("waybar restarted successfully");
    } else {
        tracing::warn!("waybar restart exited with status: {status}");
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
