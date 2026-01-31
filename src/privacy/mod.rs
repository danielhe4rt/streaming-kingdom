use std::collections::HashMap;
use std::sync::Arc;

use hyprland::prelude::*;
use tokio::sync::mpsc;

use crate::config::PrivacyConfig;

// ---------------------------------------------------------------------------
// Commands & status messages exchanged with the TUI
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivacyCommand {
    Start,
    Stop,
}

#[derive(Debug, Clone)]
pub enum PrivacyStatus {
    /// Hyprland listener is running.
    Running,
    /// Listener stopped cleanly.
    Stopped,
    /// A sensitive window was detected; blur activated.
    BlurEnabled { title: String },
    /// Switched away from sensitive window; blur removed.
    BlurDisabled,
    /// Non-fatal error (e.g. OBS not reachable).
    Error(String),
}

// ---------------------------------------------------------------------------
// OBS blur helpers
// ---------------------------------------------------------------------------

const FILTER_NAME: &str = "Composite Blur";
const FILTER_KIND: &str = "obs_composite_blur";

/// Try to connect to OBS. Returns `None` (with a logged warning) when OBS
/// isn't running or the connection is refused – this is expected during normal
/// desktop use.
async fn try_connect_obs(
    host: &str,
    port: u16,
    password: &str,
) -> Option<obws::Client> {
    let pw: Option<&str> = if password.is_empty() {
        None
    } else {
        Some(password)
    };

    match obws::Client::connect(host, port, pw).await {
        Ok(client) => Some(client),
        Err(e) => {
            tracing::warn!("OBS not available: {e}");
            None
        }
    }
}

/// Ensure a blur filter exists and is **enabled** on `capture_source`.
async fn enable_blur(client: &obws::Client, capture_source: &str) {
    let source = obws::requests::sources::SourceId::Name(capture_source);

    // Check whether the filter already exists.
    match client.filters().get(source, FILTER_NAME).await {
        Ok(existing) => {
            if !existing.enabled {
                let req: obws::requests::filters::SetEnabled<'_> = obws::requests::filters::SetEnabled {
                    source,
                    filter: FILTER_NAME,
                    enabled: true,
                };
                if let Err(e) = client.filters().set_enabled(req).await {
                    tracing::error!("failed to enable blur filter: {e}");
                }
            }
        }
        Err(_) => {
            // Filter doesn't exist yet – create it enabled.
            let settings = serde_json::json!({
                "speed_x": 0.0,
                "speed_y": 0.0,
                "cx": 240.0,
                "cy": 240.0,
            });
            let req = obws::requests::filters::Create {
                source,
                filter: FILTER_NAME,
                kind: FILTER_KIND,
                settings: Some(settings),
            };
            if let Err(e) = client.filters().create(req).await {
                tracing::error!("failed to create blur filter: {e}");
            }
        }
    }
}

/// Disable (but keep) the blur filter so it can be quickly re-enabled.
async fn disable_blur(client: &obws::Client, capture_source: &str) {
    let source = obws::requests::sources::SourceId::Name(capture_source);
    let req = obws::requests::filters::SetEnabled {
        source,
        filter: FILTER_NAME,
        enabled: false,
    };
    if let Err(e) = client.filters().set_enabled(req).await {
        tracing::debug!("blur filter disable skipped: {e}");
    }
}

// ---------------------------------------------------------------------------
// Pattern matching
// ---------------------------------------------------------------------------

fn is_sensitive(title: &str, patterns: &[String]) -> bool {
    let lower = title.to_lowercase();
    patterns.iter().any(|p| lower.contains(&p.to_lowercase()))
}

// ---------------------------------------------------------------------------
// Core monitor task
// ---------------------------------------------------------------------------

/// Spawns the privacy monitor as a tokio task.
///
/// * `cmd_rx`    – receives `Start` / `Stop` from the TUI.
/// * `status_tx` – sends status updates back to the TUI.
/// * `config`    – privacy-specific configuration (patterns).
/// * `obs_host`, `obs_port`, `obs_password`, `capture_source` – OBS conn info.
///
/// The returned `JoinHandle` can be used to await or abort the task.
pub fn spawn(
    mut cmd_rx: mpsc::Receiver<PrivacyCommand>,
    status_tx: mpsc::Sender<PrivacyStatus>,
    config: Arc<PrivacyConfig>,
    obs_host: Arc<str>,
    obs_port: u16,
    obs_password: Arc<str>,
    capture_source: Arc<str>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // Wait for the first Start command before doing anything.
        loop {
            match cmd_rx.recv().await {
                Some(PrivacyCommand::Start) => break,
                Some(PrivacyCommand::Stop) => continue,
                None => return, // channel closed
            }
        }

        run_monitor(
            &mut cmd_rx,
            &status_tx,
            &config,
            &obs_host,
            obs_port,
            &obs_password,
            &capture_source,
        )
        .await;
    })
}

async fn run_monitor(
    cmd_rx: &mut mpsc::Receiver<PrivacyCommand>,
    status_tx: &mpsc::Sender<PrivacyStatus>,
    config: &PrivacyConfig,
    obs_host: &str,
    obs_port: u16,
    obs_password: &str,
    capture_source: &str,
) {
    let _ = status_tx.send(PrivacyStatus::Running).await;

    // Try connecting to OBS (non-fatal if unavailable).
    let obs = try_connect_obs(obs_host, obs_port, obs_password).await;
    if obs.is_none() {
        let _ = status_tx
            .send(PrivacyStatus::Error(
                "OBS not available – blur disabled".into(),
            ))
            .await;
    }

    // Track ALL open windows so blur stays active when a sensitive file is
    // open on any monitor, not just the focused one.
    // Key = window address (hex string), Value = window title.
    let mut windows: HashMap<String, String> = HashMap::new();

    // Seed with every window that is already open.
    match hyprland::data::Clients::get_async().await {
        Ok(clients) => {
            for client in clients {
                windows.insert(client.address.to_string(), client.title);
            }
        }
        Err(e) => {
            tracing::warn!("failed to query existing windows: {e}");
        }
    }

    // Channel carrying window lifecycle events from the Hyprland listener.
    #[derive(Debug)]
    enum WindowEvent {
        Opened { address: String, title: String },
        Closed { address: String },
        TitleChanged { address: String, title: String },
    }

    let (event_tx, mut event_rx) = mpsc::channel::<WindowEvent>(64);

    let listener_handle = tokio::spawn({
        let tx = event_tx.clone();
        async move {
            let mut listener = hyprland::event_listener::AsyncEventListener::new();

            // Track newly opened windows.
            let tx_open = tx.clone();
            listener.add_window_opened_handler(move |data| {
                let tx = tx_open.clone();
                Box::pin(async move {
                    let _ = tx
                        .send(WindowEvent::Opened {
                            address: data.window_address.to_string(),
                            title: data.window_title,
                        })
                        .await;
                })
            });

            // Track closed windows.
            let tx_close = tx.clone();
            listener.add_window_closed_handler(move |addr| {
                let tx = tx_close.clone();
                Box::pin(async move {
                    let _ = tx
                        .send(WindowEvent::Closed {
                            address: addr.to_string(),
                        })
                        .await;
                })
            });

            // Track title changes (e.g. editor opens a different file).
            let tx_title = tx.clone();
            listener.add_window_title_changed_handler(move |data| {
                let tx = tx_title.clone();
                Box::pin(async move {
                    let _ = tx
                        .send(WindowEvent::TitleChanged {
                            address: data.address.to_string(),
                            title: data.title,
                        })
                        .await;
                })
            });

            if let Err(e) = listener.start_listener_async().await {
                tracing::error!("hyprland listener error: {e}");
            }
        }
    });

    let mut blur_active = false;

    // Evaluate initial window set – a sensitive file may already be open.
    let has_sensitive = windows
        .values()
        .any(|t| is_sensitive(t, &config.sensitive_patterns));
    if has_sensitive {
        if let Some(ref client) = obs {
            enable_blur(client, capture_source).await;
        }
        blur_active = true;
        let title = windows
            .values()
            .find(|t| is_sensitive(t, &config.sensitive_patterns))
            .cloned()
            .unwrap_or_default();
        let _ = status_tx
            .send(PrivacyStatus::BlurEnabled { title })
            .await;
    }

    loop {
        tokio::select! {
            // Window lifecycle event from Hyprland.
            Some(event) = event_rx.recv() => {
                match event {
                    WindowEvent::Opened { address, title } => {
                        windows.insert(address, title);
                    }
                    WindowEvent::Closed { address } => {
                        windows.remove(&address);
                    }
                    WindowEvent::TitleChanged { address, title } => {
                        windows.insert(address, title);
                    }
                }

                let has_sensitive = windows
                    .values()
                    .any(|t| is_sensitive(t, &config.sensitive_patterns));

                if has_sensitive && !blur_active {
                    let title = windows
                        .values()
                        .find(|t| is_sensitive(t, &config.sensitive_patterns))
                        .cloned()
                        .unwrap_or_default();
                    if let Some(ref client) = obs {
                        enable_blur(client, capture_source).await;
                    }
                    blur_active = true;
                    let _ = status_tx
                        .send(PrivacyStatus::BlurEnabled { title })
                        .await;
                } else if !has_sensitive && blur_active {
                    if let Some(ref client) = obs {
                        disable_blur(client, capture_source).await;
                    }
                    blur_active = false;
                    let _ = status_tx.send(PrivacyStatus::BlurDisabled).await;
                }
            }

            // Commands from the TUI.
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(PrivacyCommand::Stop) | None => {
                        // Clean up: remove blur if active.
                        if blur_active {
                            if let Some(ref client) = obs {
                                disable_blur(client, capture_source).await;
                            }
                        }
                        listener_handle.abort();
                        let _ = status_tx.send(PrivacyStatus::Stopped).await;

                        // If the channel is still open, wait for a
                        // potential restart.
                        if cmd.is_some() {
                            loop {
                                match cmd_rx.recv().await {
                                    Some(PrivacyCommand::Start) => {
                                        // Recurse to re-enter monitor mode.
                                        return Box::pin(run_monitor(
                                            cmd_rx,
                                            status_tx,
                                            config,
                                            obs_host,
                                            obs_port,
                                            obs_password,
                                            capture_source,
                                        ))
                                        .await;
                                    }
                                    Some(PrivacyCommand::Stop) => continue,
                                    None => return,
                                }
                            }
                        }
                        return;
                    }
                    Some(PrivacyCommand::Start) => {
                        // Already running, ignore.
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pattern_matching() {
        let patterns: Vec<String> = vec![
            ".env".into(),
            "credentials".into(),
            ".secret".into(),
            ".pem".into(),
            "id_rsa".into(),
        ];

        assert!(is_sensitive(".env - nvim", &patterns));
        assert!(is_sensitive("credentials.json - code", &patterns));
        assert!(is_sensitive("server.pem - cat", &patterns));
        assert!(is_sensitive("~/.ssh/id_rsa - vim", &patterns));
        assert!(is_sensitive("MY_APP.ENV.LOCAL - nano", &patterns));

        assert!(!is_sensitive("main.rs - code", &patterns));
        assert!(!is_sensitive("README.md - nvim", &patterns));
        assert!(!is_sensitive("", &patterns));
    }
}
