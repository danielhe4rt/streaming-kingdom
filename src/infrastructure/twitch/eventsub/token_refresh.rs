//! OAuth token refresh flow and persistence of the renewed credentials.

use serde_json::Value;

use crate::domain::AppEvent;

use super::client::TwitchClient;
use super::error::ClientError;
use super::TOKEN_REFRESH_URL;

impl TwitchClient {
    /// Attempt to refresh the OAuth token using the refresh_token.
    ///
    /// On success, updates `self.config.oauth_token` and `self.config.refresh_token`
    /// in memory and persists them to config.toml.
    pub(super) async fn refresh_token(&mut self) -> Result<(), ClientError> {
        if self.config.refresh_token.is_empty() {
            return Err(ClientError::Http("no refresh_token configured".into()));
        }

        tracing::info!("refreshing OAuth token...");
        let _ = self
            .app_event_tx
            .send(AppEvent::Info("Twitch: refreshing OAuth token...".into()))
            .await;

        let resp = self
            .http
            .post(TOKEN_REFRESH_URL)
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", self.config.refresh_token.as_str()),
                ("client_id", self.config.client_id.as_str()),
                ("client_secret", self.config.client_secret.as_str()),
            ])
            .send()
            .await
            .map_err(|e| ClientError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            let _ = self
                .app_event_tx
                .send(AppEvent::Error(format!(
                    "Twitch: token refresh failed: {body}"
                )))
                .await;
            return Err(ClientError::Http(format!("token refresh failed: {body}")));
        }

        let body: Value = resp
            .json()
            .await
            .map_err(|e| ClientError::Http(e.to_string()))?;

        let new_access = body["access_token"]
            .as_str()
            .ok_or_else(|| ClientError::Http("no access_token in refresh response".into()))?
            .to_string();

        let new_refresh = body["refresh_token"]
            .as_str()
            .ok_or_else(|| ClientError::Http("no refresh_token in refresh response".into()))?
            .to_string();

        self.config.oauth_token = new_access.clone();
        self.config.refresh_token = new_refresh.clone();

        // Persist to disk (best-effort — don't fail the refresh if disk write fails)
        if let Err(e) = crate::application::config::save_twitch_tokens(&new_access, &new_refresh) {
            tracing::warn!("failed to persist refreshed tokens: {e}");
        }

        tracing::info!("OAuth token refreshed successfully");
        let _ = self
            .app_event_tx
            .send(AppEvent::Info(
                "Twitch: token refreshed successfully".into(),
            ))
            .await;

        Ok(())
    }
}
