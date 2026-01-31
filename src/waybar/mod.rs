mod config;
mod events;
mod style;

use std::path::PathBuf;
use std::{fs, io};

use tokio::process::{Child, Command};
use tokio::sync::broadcast;

use crate::app::StreamEvent;

// ---------------------------------------------------------------------------
// Cache directory layout
// ---------------------------------------------------------------------------

fn cache_dir() -> io::Result<PathBuf> {
    let base = dirs::cache_dir().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "no cache directory available")
    })?;
    Ok(base.join("streams-toolkit"))
}

fn waybar_dir() -> io::Result<PathBuf> {
    Ok(cache_dir()?.join("waybar"))
}

fn data_file_path() -> io::Result<PathBuf> {
    Ok(cache_dir()?.join("stream_data.json"))
}

// ---------------------------------------------------------------------------
// Config / style file generation
// ---------------------------------------------------------------------------

/// Write waybar config and style files into the cache directory.
/// Returns `(config_path, style_path)`.
pub fn write_config_files(output: &str) -> io::Result<(PathBuf, PathBuf)> {
    let dir = waybar_dir()?;
    fs::create_dir_all(&dir)?;

    let config_path = dir.join("config.jsonc");
    let style_path = dir.join("style.css");

    let config_json = config::generate(output);
    fs::write(&config_path, config_json)?;

    let style_css = style::generate();
    fs::write(&style_path, style_css)?;

    // Ensure the data file exists so waybar doesn't fail on first read
    let data_path = data_file_path()?;
    if !data_path.exists() {
        if let Some(parent) = data_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&data_path, "[]")?;
    }

    tracing::info!(
        "wrote waybar config to {} and style to {}",
        config_path.display(),
        style_path.display()
    );

    Ok((config_path, style_path))
}

// ---------------------------------------------------------------------------
// Process management
// ---------------------------------------------------------------------------

/// Spawn a waybar instance with our generated config/style.
pub fn spawn(config_path: &PathBuf, style_path: &PathBuf) -> io::Result<Child> {
    let child = Command::new("waybar")
        .arg("-c")
        .arg(config_path)
        .arg("-s")
        .arg(style_path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()?;

    tracing::info!("spawned waybar (pid {:?})", child.id());
    Ok(child)
}

/// Kill a running waybar child process.
pub async fn kill(child: &mut Child) {
    if let Some(pid) = child.id() {
        tracing::info!("killing waybar (pid {pid})");
    }
    let _ = child.kill().await;
    let _ = child.wait().await;
}

// ---------------------------------------------------------------------------
// Event listener – writes stream_data.json on each event
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
