//! Webhook server lifecycle: the spawn entry-point and the per-session runner.

use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tokio::sync::{broadcast, mpsc, Mutex};

use crate::application::LivepixConfig;
use crate::domain::StreamEvent;

use super::super::{LivepixCommand, LivepixStatus, TtsRequest};
use super::handlers::{handle_webhook, health_check};
use super::oauth::get_oauth_token;
use super::server_state::ServerState;

/// Spawns the Livepix webhook server as a tokio task.
///
/// * `cmd_rx`    -- receives `Start` / `Stop` from the TUI.
/// * `status_tx` -- sends status updates back to the TUI.
/// * `event_tx`  -- broadcast channel for `StreamEvent::Donation`.
/// * `tts_tx`    -- optional channel for forwarding TTS requests.
/// * `config`    -- Livepix-specific configuration.
///
/// The returned `JoinHandle` can be used to await or abort the task.
pub fn spawn(
    mut cmd_rx: mpsc::Receiver<LivepixCommand>,
    status_tx: mpsc::Sender<LivepixStatus>,
    event_tx: broadcast::Sender<StreamEvent>,
    tts_tx: Option<mpsc::Sender<TtsRequest>>,
    config: LivepixConfig,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            // Wait for a Start command before doing anything.
            match cmd_rx.recv().await {
                Some(LivepixCommand::Start) => {}
                Some(LivepixCommand::Stop) => continue,
                None => return, // channel closed
            }

            run_server(&mut cmd_rx, &status_tx, &event_tx, &tts_tx, &config).await;
        }
    })
}

async fn run_server(
    cmd_rx: &mut mpsc::Receiver<LivepixCommand>,
    status_tx: &mpsc::Sender<LivepixStatus>,
    event_tx: &broadcast::Sender<StreamEvent>,
    tts_tx: &Option<mpsc::Sender<TtsRequest>>,
    config: &LivepixConfig,
) {
    let http_client = reqwest::Client::new();

    // Authenticate via OAuth
    tracing::info!("livepix: authenticating via OAuth");
    let token = match get_oauth_token(&http_client, &config.client_id, &config.client_secret).await
    {
        Ok(t) => {
            tracing::info!("livepix: OAuth authentication successful");
            let _ = status_tx.send(LivepixStatus::OAuthSuccess).await;
            t
        }
        Err(e) => {
            tracing::error!("livepix: OAuth authentication failed: {e}");
            let _ = status_tx.send(LivepixStatus::OAuthError(e.clone())).await;
            let _ = status_tx.send(LivepixStatus::Stopped).await;
            return;
        }
    };

    let state = ServerState {
        client_id: config.client_id.clone(),
        client_secret: config.client_secret.clone(),
        http_client,
        token: Arc::new(Mutex::new(Some(token))),
        event_tx: event_tx.clone(),
        status_tx: status_tx.clone(),
        tts_tx: tts_tx.clone(),
    };

    let app = Router::new()
        .route("/webhooks", post(handle_webhook))
        .route("/", get(health_check))
        .with_state(state);

    let port = config.webhook_port;
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            let msg = format!("failed to bind to {addr}: {e}");
            tracing::error!("livepix: {msg}");
            let _ = status_tx.send(LivepixStatus::Error(msg)).await;
            let _ = status_tx.send(LivepixStatus::Stopped).await;
            return;
        }
    };

    tracing::info!("livepix: webhook server listening on {addr}");
    let _ = status_tx.send(LivepixStatus::Running { port }).await;

    // Graceful shutdown signal: fires when we receive a Stop command.
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // Spawn the server with graceful shutdown
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
    });

    // Wait for Stop command (or channel close)
    loop {
        match cmd_rx.recv().await {
            Some(LivepixCommand::Stop) | None => {
                let _ = shutdown_tx.send(());
                let _ = server_handle.await;
                tracing::info!("livepix: webhook server stopped");
                let _ = status_tx.send(LivepixStatus::Stopped).await;
                return;
            }
            Some(LivepixCommand::Start) => {
                // Already running, ignore.
            }
        }
    }
}
