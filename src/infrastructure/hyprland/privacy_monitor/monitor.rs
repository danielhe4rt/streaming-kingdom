//! The monitor task: connect OBS, watch windows, and drive blur until stopped.

use std::sync::Arc;

use tokio::sync::mpsc;

use crate::application::PrivacyConfig;
use crate::infrastructure::obs;

use super::blur_state::{apply_blur_transition, teardown_blur};
use super::messages::{PrivacyCommand, PrivacyStatus};
use super::window_events::{WindowEvent, snapshot_open_windows, spawn_window_listener};

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
    let obs_client = obs::try_connect_obs(obs_host, obs_port, obs_password).await;
    if obs_client.is_none() {
        let _ = status_tx
            .send(PrivacyStatus::Error(
                "OBS not available – blur disabled".into(),
            ))
            .await;
    }

    let mut windows = snapshot_open_windows().await;

    let (event_tx, mut event_rx) = mpsc::channel::<WindowEvent>(64);
    let listener_handle = spawn_window_listener(event_tx);

    let mut blur_active = false;

    // Evaluate initial window set – a sensitive file may already be open.
    apply_blur_transition(
        &windows,
        config,
        &obs_client,
        capture_source,
        status_tx,
        &mut blur_active,
    )
    .await;

    loop {
        tokio::select! {
            // Window lifecycle event from Hyprland.
            Some(event) = event_rx.recv() => {
                match event {
                    WindowEvent::Opened { address, title } => { windows.insert(address, title); }
                    WindowEvent::Closed { address } => { windows.remove(&address); }
                    WindowEvent::TitleChanged { address, title } => { windows.insert(address, title); }
                }

                apply_blur_transition(
                    &windows, config, &obs_client, capture_source, status_tx, &mut blur_active,
                )
                .await;
            }

            // Commands from the TUI.
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(PrivacyCommand::Stop) | None => {
                        // Clean up: remove blur if active.
                        teardown_blur(&obs_client, capture_source, blur_active).await;
                        listener_handle.abort();
                        let _ = status_tx.send(PrivacyStatus::Stopped).await;

                        // If the channel is still open, wait for a restart.
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
