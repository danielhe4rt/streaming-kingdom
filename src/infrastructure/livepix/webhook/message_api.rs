//! Fetching the full donation message detail from the Livepix REST API.

use super::oauth::resolve_token;
use super::payload::{MessageData, MessageResponse};
use super::server_state::ServerState;

pub(super) async fn fetch_message(
    state: &ServerState,
    message_id: &str,
) -> Result<MessageData, String> {
    let token = resolve_token(state).await?;

    let response = state
        .http_client
        .get(format!("https://api.livepix.gg/v2/messages/{}", message_id))
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
