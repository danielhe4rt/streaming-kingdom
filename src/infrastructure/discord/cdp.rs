// ---------------------------------------------------------------------------
// CDP auto-injector for the `cdp` source mode (Vesktop, no Vencord build).
//
// Vesktop launched with `--remote-debugging-port=<cdp_port>` exposes the Discord
// renderer over the Chrome DevTools Protocol. We find that page target, then
// `Runtime.evaluate` the inject.js payload — which reads the already-loaded
// Vencord webpack stores, subscribes to Flux voice/speaking events, and pushes
// snapshots to the bridge WS ingress (`bridge.rs`). The payload is idempotent, so
// we simply re-inject on a fixed interval to survive Vesktop reloads/restarts.
// ---------------------------------------------------------------------------

use std::io;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use super::DiscordStatus;

/// The renderer payload, with the bridge port templated in at runtime.
const INJECT_JS: &str = include_str!("inject.js");

/// Re-inject on this cadence so a Vesktop reload (which drops the injected state)
/// is recovered automatically; injection is idempotent so this is cheap.
const REINJECT_EVERY: Duration = Duration::from_secs(10);

/// Run forever: (re)inject the bridge payload into Vesktop's renderer over CDP.
/// Never returns under normal operation; transient errors are retried. Logs only
/// on the inject reachable↔unreachable transition (not every 10s tick) so the TUI
/// shows whether Vesktop is actually attached without spamming.
pub async fn inject_loop(
    cdp_port: u16,
    bridge_port: u16,
    status_tx: &mpsc::Sender<DiscordStatus>,
) -> io::Result<()> {
    let payload = INJECT_JS.replace("__PORT__", &bridge_port.to_string());
    let mut last_ok: Option<bool> = None;
    loop {
        match inject_once(cdp_port, &payload).await {
            Ok(result) => {
                tracing::debug!("discord cdp: inject -> {result}");
                if last_ok != Some(true) {
                    let _ = status_tx.try_send(DiscordStatus::Activity(
                        "reader injected into Vesktop ✓".to_string(),
                    ));
                }
                last_ok = Some(true);
            }
            Err(e) => {
                tracing::warn!("discord cdp: inject failed ({e})");
                if last_ok != Some(false) {
                    let _ = status_tx.try_send(DiscordStatus::Activity(format!(
                        "no Vesktop on CDP :{cdp_port} (--remote-debugging-port?) — retrying"
                    )));
                }
                last_ok = Some(false);
            }
        }
        tokio::time::sleep(REINJECT_EVERY).await;
    }
}

/// One inject cycle: locate the Discord page target and evaluate the payload.
async fn inject_once(cdp_port: u16, payload: &str) -> io::Result<String> {
    let ws_url = discord_target(cdp_port).await?;
    let (mut ws, _) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .map_err(io::Error::other)?;

    let cmd = json!({
        "id": 1,
        "method": "Runtime.evaluate",
        "params": { "expression": payload, "returnByValue": true, "awaitPromise": true },
    });
    ws.send(Message::Text(cmd.to_string().into()))
        .await
        .map_err(io::Error::other)?;

    while let Some(frame) = ws.next().await {
        let text = match frame.map_err(io::Error::other)? {
            Message::Text(text) => text,
            _ => continue,
        };
        let msg: Value = serde_json::from_str(&text)?;
        if msg.get("id") == Some(&json!(1)) {
            let result = msg.pointer("/result/result/value").and_then(Value::as_str);
            return Ok(result.unwrap_or("evaluated").to_string());
        }
    }
    Err(io::Error::new(
        io::ErrorKind::UnexpectedEof,
        "CDP closed before reply",
    ))
}

/// GET `/json/list` and return the WebSocket debugger URL of the Discord page.
async fn discord_target(cdp_port: u16) -> io::Result<String> {
    let url = format!("http://127.0.0.1:{cdp_port}/json/list");
    let targets: Vec<Value> = reqwest::get(&url)
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(io::Error::other)?
        .json()
        .await
        .map_err(io::Error::other)?;

    targets
        .iter()
        .find(|t| {
            t.get("type").and_then(Value::as_str) == Some("page")
                && t.get("url")
                    .and_then(Value::as_str)
                    .is_some_and(|u| u.contains("discord.com"))
        })
        .and_then(|t| t.get("webSocketDebuggerUrl").and_then(Value::as_str))
        .map(str::to_string)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no Discord CDP page target found"))
}
