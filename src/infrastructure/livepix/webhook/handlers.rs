//! Axum route handlers: health check and the donation webhook ingest + fan-out.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::domain::StreamEvent;

use super::super::{LivepixStatus, TtsRequest};
use super::message_api::fetch_message;
use super::payload::WebhookPayload;
use super::server_state::ServerState;

pub(super) async fn health_check() -> &'static str {
    "Livepix Webhook Server Running"
}

pub(super) async fn handle_webhook(
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
                Ok(message) => process_donation(&state_clone, message).await,
                Err(e) => {
                    tracing::error!("failed to fetch livepix message: {e}");
                    let _ = state_clone.status_tx.send(LivepixStatus::Error(e)).await;
                }
            }
        });
    }

    StatusCode::OK
}

/// Broadcast the donation, notify the TUI + desktop, and forward to TTS.
async fn process_donation(state: &ServerState, message: super::payload::MessageData) {
    let amount_cents = message.amount.unsigned_abs();
    let amount_display = format!("{} {}", message.amount, message.currency);

    tracing::info!(
        username = %message.username,
        amount = %amount_display,
        message = %message.message,
        "livepix donation received"
    );

    // Broadcast StreamEvent::Donation
    let _ = state.event_tx.send(StreamEvent::Donation {
        username: message.username.clone(),
        amount_cents,
        message: message.message.clone(),
    });

    // Notify TUI
    let _ = state
        .status_tx
        .send(LivepixStatus::WebhookReceived {
            username: message.username.clone(),
            amount: amount_display.clone(),
        })
        .await;

    // Desktop notification via notify-send
    let title = format!("Donation — {}", amount_display);
    let body = format!(
        "<b>{}</b> donated <b>{}</b>\n\"{}\"",
        message.username, amount_display, message.message
    );
    let _ = std::process::Command::new("notify-send")
        .arg("--app-name=streams-toolkit")
        .arg("--urgency=critical")
        .arg("--expire-time=12000")
        .arg("--category=stream.donate")
        .arg(&title)
        .arg(&body)
        .spawn();

    // Forward to TTS (non-blocking)
    if let Some(ref tts_tx) = state.tts_tx
        && tts_tx
            .try_send(TtsRequest {
                text: message.message,
                username: message.username,
            })
            .is_err()
    {
        tracing::warn!("TTS queue full, dropping request");
    }
}
