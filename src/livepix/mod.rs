pub mod tts;

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use tokio::sync::{broadcast, mpsc, Mutex};

use crate::app::StreamEvent;
use crate::config::LivepixConfig;

pub use tts::TtsRequest;

// ---------------------------------------------------------------------------
// Commands & status messages exchanged with the TUI
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LivepixCommand {
    Start,
    Stop,
}

#[derive(Debug, Clone)]
pub enum LivepixStatus {
    Running { port: u16 },
    Stopped,
    OAuthSuccess,
    OAuthError(String),
    WebhookReceived { username: String, amount: String },
    Error(String),
}

// ---------------------------------------------------------------------------
// Webhook deserialization types (ported from webhook-server.rs)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct WebhookPayload {
    #[serde(default)]
    event: String,
    #[serde(default)]
    resource: Resource,
}

#[derive(Debug, Deserialize, Default)]
struct Resource {
    #[serde(default)]
    id: String,
    #[serde(rename = "type", default)]
    resource_type: String,
}

#[derive(Debug, Deserialize)]
struct OAuthToken {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct MessageResponse {
    data: MessageData,
}

#[derive(Debug, Deserialize)]
struct MessageData {
    #[serde(default)]
    message: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    amount: i64,
    #[serde(default)]
    currency: String,
}

// ---------------------------------------------------------------------------
// Shared state for the Axum handlers
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct ServerState {
    client_id: String,
    client_secret: String,
    http_client: reqwest::Client,
    token: Arc<Mutex<Option<String>>>,
    event_tx: broadcast::Sender<StreamEvent>,
    status_tx: mpsc::Sender<LivepixStatus>,
    tts_tx: Option<mpsc::Sender<TtsRequest>>,
}

// ---------------------------------------------------------------------------
// OAuth – client_credentials flow
// ---------------------------------------------------------------------------

async fn get_oauth_token(
    http_client: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
) -> Result<String, String> {
    let params = [
        ("grant_type", "client_credentials"),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("scope", "account:read wallet:read webhooks messages:read"),
    ];

    let response = http_client
        .post("https://oauth.livepix.gg/oauth2/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let token_data: OAuthToken = response.json().await.map_err(|e| e.to_string())?;

    Ok(token_data.access_token)
}

/// Retrieve a cached token or re-authenticate if the cache is empty.
async fn resolve_token(state: &ServerState) -> Result<String, String> {
    {
        let token = state.token.lock().await;
        if let Some(t) = token.as_ref() {
            return Ok(t.clone());
        }
    }

    tracing::warn!("livepix token cache empty, re-authenticating");
    let new_token =
        get_oauth_token(&state.http_client, &state.client_id, &state.client_secret).await?;

    {
        let mut token = state.token.lock().await;
        *token = Some(new_token.clone());
    }

    Ok(new_token)
}

// ---------------------------------------------------------------------------
// Fetch message details from Livepix API
// ---------------------------------------------------------------------------

async fn fetch_message(state: &ServerState, message_id: &str) -> Result<MessageData, String> {
    let token = resolve_token(state).await?;

    let response = state
        .http_client
        .get(format!(
            "https://api.livepix.gg/v2/messages/{}",
            message_id
        ))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status();
    let body_text = response.text().await.map_err(|e| e.to_string())?;

    match serde_json::from_str::<MessageResponse>(&body_text) {
        Ok(resp) => Ok(resp.data),
        Err(e) => {
            tracing::error!(
                status = %status,
                body = %body_text,
                "failed to parse livepix message"
            );
            Err(format!("failed to parse message: {e}"))
        }
    }
}

// ---------------------------------------------------------------------------
// Axum handlers
// ---------------------------------------------------------------------------

async fn health_check() -> &'static str {
    "Livepix Webhook Server Running"
}

async fn handle_webhook(
    State(state): State<ServerState>,
    axum::extract::Json(webhook): axum::extract::Json<serde_json::Value>,
) -> impl IntoResponse {
    let parsed = match serde_json::from_value::<WebhookPayload>(webhook) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("failed to parse webhook payload: {e}");
            return StatusCode::OK;
        }
    };

    if parsed.event == "new" && parsed.resource.resource_type == "message" {
        let state_clone = state.clone();
        let message_id = parsed.resource.id;

        // Process asynchronously so we never block the HTTP response
        tokio::spawn(async move {
            match fetch_message(&state_clone, &message_id).await {
                Ok(message) => {
                    let amount_cents = message.amount.unsigned_abs();
                    let amount_display = format!("{} {}", message.amount, message.currency);

                    tracing::info!(
                        username = %message.username,
                        amount = %amount_display,
                        message = %message.message,
                        "livepix donation received"
                    );

                    // Broadcast StreamEvent::Donation
                    let _ = state_clone.event_tx.send(StreamEvent::Donation {
                        username: message.username.clone(),
                        amount_cents,
                        message: message.message.clone(),
                    });

                    // Notify TUI
                    let _ = state_clone
                        .status_tx
                        .send(LivepixStatus::WebhookReceived {
                            username: message.username.clone(),
                            amount: amount_display,
                        })
                        .await;

                    // Forward to TTS (non-blocking)
                    if let Some(ref tts_tx) = state_clone.tts_tx {
                        if tts_tx
                            .try_send(TtsRequest {
                                text: message.message,
                                username: message.username,
                            })
                            .is_err()
                        {
                            tracing::warn!("TTS queue full, dropping request");
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("failed to fetch livepix message: {e}");
                    let _ = state_clone
                        .status_tx
                        .send(LivepixStatus::Error(e))
                        .await;
                }
            }
        });
    }

    StatusCode::OK
}

// ---------------------------------------------------------------------------
// Spawn entry-point (follows privacy::spawn pattern)
// ---------------------------------------------------------------------------

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

            run_server(
                &mut cmd_rx,
                &status_tx,
                &event_tx,
                &tts_tx,
                &config,
            )
            .await;
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
            let _ = status_tx
                .send(LivepixStatus::OAuthError(e.clone()))
                .await;
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
