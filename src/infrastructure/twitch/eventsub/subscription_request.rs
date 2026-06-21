//! Building and sending a single EventSub subscription request to Helix.

use serde_json::Value;

use super::HELIX_SUBSCRIPTIONS_URL;
use super::client::TwitchClient;
use super::error::ClientError;
use super::subscription_catalog::SubDef;

impl TwitchClient {
    /// Send a single EventSub subscription request, returning the HTTP status.
    pub(super) async fn try_subscribe(
        &self,
        def: &SubDef,
        session_id: &str,
    ) -> Result<reqwest::StatusCode, ClientError> {
        let mut condition = serde_json::Map::new();

        if def.uses_broadcaster {
            condition.insert(
                "broadcaster_user_id".into(),
                Value::String(self.config.broadcaster_user_id.clone()),
            );
        } else {
            // channel.raid uses to_broadcaster_user_id
            condition.insert(
                "to_broadcaster_user_id".into(),
                Value::String(self.config.broadcaster_user_id.clone()),
            );
        }

        if def.needs_moderator {
            condition.insert(
                "moderator_user_id".into(),
                Value::String(self.config.broadcaster_user_id.clone()),
            );
        }

        let body = serde_json::json!({
            "type": def.sub_type,
            "version": def.version,
            "condition": condition,
            "transport": {
                "method": "websocket",
                "session_id": session_id,
            }
        });

        // WebSocket transport requires a user access token, not an app token.
        let token = &self.config.oauth_token;

        let resp = self
            .http
            .post(HELIX_SUBSCRIPTIONS_URL)
            .header("Authorization", format!("Bearer {token}"))
            .header("Client-Id", &self.config.client_id)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ClientError::Http(e.to_string()))?;

        let status = resp.status();

        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            tracing::debug!(
                sub_type = def.sub_type,
                status = %status,
                body = body_text,
                "subscription response"
            );
        }

        Ok(status)
    }
}
