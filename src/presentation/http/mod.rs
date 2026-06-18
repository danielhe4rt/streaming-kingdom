//! `http` renderer — serves the Overlays as OBS browser sources (ADR-0001).
//!
//! Laravel-like layout: `routes` (route table) / `controllers` (handlers) /
//! `resources` (serde Feed DTOs, M3) / `assets` (embedded React build). It
//! subscribes to the `broadcast<ChatMessage>` Overlay Feed and renders it; it
//! never owns the chat (ADR-0001, the presentation split).
//!
//! For this tracer-bullet slice the server simply starts on boot via
//! [`spawn`]; the TUI on/off toggle arrives with the nav-shell reorg slice.

pub mod assets;
pub mod controllers;
pub mod resources;
pub mod routes;

#[cfg(test)]
mod tests;

use tokio::sync::broadcast;

use crate::domain::ChatSignal;

/// Shared state handed to the Overlay controllers.
#[derive(Clone)]
pub struct OverlayState {
    /// Chat broadcast the Overlay Feed subscribes to (one feed, N Overlays).
    /// Carries both new messages and CLEARMSG deletions as [`ChatSignal`]s.
    pub chat_tx: broadcast::Sender<ChatSignal>,
}

/// Start the Overlay HTTP server, binding `127.0.0.1:<port>`.
///
/// Follows the existing `spawn()` pattern (returns a `JoinHandle`); for this
/// slice it starts immediately rather than waiting on a Start command.
pub fn spawn(
    chat_tx: broadcast::Sender<ChatSignal>,
    port: u16,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let app = routes::router(OverlayState { chat_tx });
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));

        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("overlays: failed to bind {addr}: {e}");
                return;
            }
        };

        tracing::info!("overlays: http server listening on http://{addr}/overlay/chat");

        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("overlays: server error: {e}");
        }
    })
}
