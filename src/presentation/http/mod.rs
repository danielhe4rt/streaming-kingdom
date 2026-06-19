//! `http` renderer — serves the Overlays as OBS browser sources (ADR-0001).
//!
//! Laravel-like layout: `routes` (route table) / `controllers` (handlers) /
//! `resources` (serde Feed DTOs, M3) / `assets` (embedded React build). It
//! subscribes to the `broadcast<ChatMessage>` Overlay Feed and renders it; it
//! never owns the chat (ADR-0001, the presentation split).
//!
//! As of the nav-shell reorg (#7) the server is no longer always-on at boot.
//! It is a proper **Output**: the TUI toggles it Start/Stop via the same
//! `spawn()` + `Start`/`Stop` command pattern as Livepix and Privacy. The
//! port comes from `[overlays.port]` in config.toml (default 1337).

pub mod assets;
pub mod controllers;
pub mod resources;
pub mod routes;

#[cfg(test)]
mod tests;

use tokio::sync::{broadcast, mpsc, watch};

use crate::domain::{ChatSignal, NowPlaying, StreamEvent};

/// Shared state handed to the Overlay controllers.
#[derive(Clone)]
pub struct OverlayState {
    /// Chat broadcast the Overlay Feed subscribes to (one feed, N Overlays).
    /// Carries both new messages and CLEARMSG deletions as [`ChatSignal`]s.
    pub chat_tx: broadcast::Sender<ChatSignal>,
    /// Stream-event broadcast (donation / sub / raid …). The same channel the
    /// TUI subscribes to (ADR-0001 — neither renderer owns the other); the feed
    /// fans these out so the Coworking Overlay's Footer Bar can play Alerts.
    pub event_tx: broadcast::Sender<StreamEvent>,
    /// Ambient now-playing *state* (latest value, not an event) on a watch
    /// channel. The feed turns each change into a `nowPlaying` FeedEvent; a new
    /// SSE connection immediately gets the current track because watch yields
    /// its current value first. `None` means stopped / no player.
    pub now_playing: watch::Receiver<Option<NowPlaying>>,
}

// ---------------------------------------------------------------------------
// Commands & status messages exchanged with the TUI (Output Start/Stop)
// ---------------------------------------------------------------------------

/// Start / Stop the Overlay server from the TUI, mirroring `LivepixCommand`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayCommand {
    Start,
    Stop,
}

/// Lifecycle status the server reports back to the TUI for display.
#[derive(Debug, Clone)]
pub enum OverlayStatus {
    Running { port: u16 },
    Stopped,
    Error(String),
}

/// Spawn the Overlay server task. It idles until it receives [`OverlayCommand::Start`],
/// binds `127.0.0.1:<port>`, and serves until [`OverlayCommand::Stop`] (graceful
/// shutdown) — then loops back to idle, ready to start again. This replaces the
/// boot-time always-on server from the tracer-bullet slice (#2).
pub fn spawn(
    mut cmd_rx: mpsc::Receiver<OverlayCommand>,
    status_tx: mpsc::Sender<OverlayStatus>,
    chat_tx: broadcast::Sender<ChatSignal>,
    event_tx: broadcast::Sender<StreamEvent>,
    now_playing_rx: watch::Receiver<Option<NowPlaying>>,
    port: u16,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            // Wait for a Start command before binding the port.
            match cmd_rx.recv().await {
                Some(OverlayCommand::Start) => {}
                Some(OverlayCommand::Stop) => continue,
                None => return, // channel closed
            }

            run_server(&mut cmd_rx, &status_tx, &chat_tx, &event_tx, &now_playing_rx, port).await;
        }
    })
}

async fn run_server(
    cmd_rx: &mut mpsc::Receiver<OverlayCommand>,
    status_tx: &mpsc::Sender<OverlayStatus>,
    chat_tx: &broadcast::Sender<ChatSignal>,
    event_tx: &broadcast::Sender<StreamEvent>,
    now_playing_rx: &watch::Receiver<Option<NowPlaying>>,
    port: u16,
) {
    let app = routes::router(OverlayState {
        chat_tx: chat_tx.clone(),
        event_tx: event_tx.clone(),
        now_playing: now_playing_rx.clone(),
    });
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            let msg = format!("failed to bind {addr}: {e}");
            tracing::error!("overlays: {msg}");
            let _ = status_tx.send(OverlayStatus::Error(msg)).await;
            let _ = status_tx.send(OverlayStatus::Stopped).await;
            return;
        }
    };

    tracing::info!("overlays: http server listening on http://{addr}/overlay/coworking");
    let _ = status_tx.send(OverlayStatus::Running { port }).await;

    // Graceful shutdown signal: fires when we receive a Stop command.
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
    });

    // Wait for Stop (or channel close) to tear the server down.
    loop {
        match cmd_rx.recv().await {
            Some(OverlayCommand::Stop) | None => {
                let _ = shutdown_tx.send(());
                let _ = server_handle.await;
                tracing::info!("overlays: http server stopped");
                let _ = status_tx.send(OverlayStatus::Stopped).await;
                return;
            }
            Some(OverlayCommand::Start) => {
                // Already running, ignore.
            }
        }
    }
}
