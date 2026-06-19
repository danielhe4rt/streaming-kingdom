//! EventSub session handshake — awaiting the `session_welcome` and extracting the session id.

use std::time::Duration;

use futures_util::StreamExt;
use serde_json::Value;
use tokio::time;
use tokio_tungstenite::tungstenite::Message;

use super::error::ClientError;

/// Wait for the welcome message and extract the session_id.
pub(super) async fn wait_for_welcome(
    stream: &mut futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
) -> Result<String, ClientError> {
    let deadline = time::Instant::now() + Duration::from_secs(15);

    loop {
        let remaining = deadline.saturating_duration_since(time::Instant::now());
        if remaining.is_zero() {
            return Err(ClientError::Connect("welcome timeout".into()));
        }

        match time::timeout(remaining, stream.next()).await {
            Ok(Some(Ok(Message::Text(text)))) => {
                let msg: Value = serde_json::from_str(&text)
                    .map_err(|e| ClientError::Connect(format!("parse welcome: {e}")))?;

                if msg["metadata"]["message_type"].as_str() == Some("session_welcome") {
                    let session_id = msg["payload"]["session"]["id"]
                        .as_str()
                        .ok_or_else(|| ClientError::Connect("no session_id in welcome".into()))?
                        .to_string();
                    return Ok(session_id);
                }
            }
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(e))) => {
                return Err(ClientError::WebSocket(e.to_string()));
            }
            Ok(None) => {
                return Err(ClientError::Connect("stream ended before welcome".into()));
            }
            Err(_) => {
                return Err(ClientError::Connect("welcome timeout".into()));
            }
        }
    }
}
