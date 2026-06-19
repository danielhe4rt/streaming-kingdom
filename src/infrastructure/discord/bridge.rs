// ---------------------------------------------------------------------------
// Bridge ingress: a local WebSocket server the injected reader (`inject.js`,
// pushed into Vesktop by `cdp.rs`) connects to and pushes JSON roster snapshots.
// Each snapshot is deserialized into a domain VoiceRoster and published on the
// watch channel the Overlay Feed subscribes to. A dropped connection clears the
// roster (publish `None`). Localhost-only; the only client is our own injected
// reader. See `docs/arch/14-infra-discord.md`.
// ---------------------------------------------------------------------------

use std::io;
use std::net::SocketAddr;

use futures_util::StreamExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::tungstenite::Message;

use crate::domain::VoiceRoster;

use super::DiscordStatus;
use super::activity::diff_lines;
use super::bridge_payload::BridgeSnapshot;

/// Send a debugging line to the TUI event log (best-effort; dropped if full so
/// the snapshot loop never blocks on a slow TUI).
fn log(status_tx: &mpsc::Sender<DiscordStatus>, line: impl Into<String>) {
    let _ = status_tx.try_send(DiscordStatus::Activity(line.into()));
}

/// Bind the bridge WS server on `127.0.0.1:port` and serve roster snapshots until
/// cancelled (the `spawn()` task races this against `Stop`). Accepts one reader at
/// a time, clearing the roster whenever a connection ends. Returns `Err` only when
/// the port cannot be bound.
pub async fn serve(
    port: u16,
    status_tx: &mpsc::Sender<DiscordStatus>,
    roster_tx: &watch::Sender<Option<VoiceRoster>>,
) -> io::Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("discord bridge: listening on ws://{addr}");
    log(status_tx, format!("ingress listening on :{port}, waiting for Vesktop reader…"));
    accept_loop(listener, status_tx, roster_tx).await
}

/// Accept connections forever, clearing the roster between them. Split out from
/// [`serve`] so tests can drive a pre-bound listener.
async fn accept_loop(
    listener: TcpListener,
    status_tx: &mpsc::Sender<DiscordStatus>,
    roster_tx: &watch::Sender<Option<VoiceRoster>>,
) -> io::Result<()> {
    loop {
        let (stream, _peer) = listener.accept().await?;
        handle_conn(stream, status_tx, roster_tx).await;
        // The reader disconnected → clear the roster until it reconnects.
        let _ = roster_tx.send(None);
        log(status_tx, "reader disconnected — roster cleared");
    }
}

/// Serve one reader connection: publish a fresh roster for every snapshot frame
/// and log the diff vs. the previous one, until the socket closes or errors.
async fn handle_conn(
    stream: TcpStream,
    status_tx: &mpsc::Sender<DiscordStatus>,
    roster_tx: &watch::Sender<Option<VoiceRoster>>,
) {
    let mut ws = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            tracing::warn!("discord bridge: handshake failed: {e}");
            return;
        }
    };
    log(status_tx, "reader connected ✓");
    let mut prev: Option<VoiceRoster> = None;
    while let Some(frame) = ws.next().await {
        let text = match frame {
            Ok(Message::Text(text)) => text,
            Ok(Message::Close(_)) | Err(_) => break,
            Ok(_) => continue, // ignore pings / binary
        };
        match serde_json::from_str::<BridgeSnapshot>(&text) {
            Ok(snapshot) => {
                let roster = snapshot.into_roster();
                for line in diff_lines(prev.as_ref(), &roster) {
                    log(status_tx, line);
                }
                let _ = roster_tx.send(Some(roster.clone()));
                prev = Some(roster);
            }
            Err(e) => tracing::warn!("discord bridge: ignoring bad snapshot: {e}"),
        }
    }
}

#[cfg(test)]
#[path = "bridge_tests.rs"]
mod bridge_tests;
