//! Registration loop: walk the catalog, registering each subscription with one
//! 401-triggered token refresh and retry.

use crate::domain::AppEvent;

use super::client::TwitchClient;
use super::error::ClientError;
use super::subscription_catalog::SUBSCRIPTIONS;

impl TwitchClient {
    /// Create EventSub subscriptions via the Helix REST API.
    ///
    /// If a subscription returns 401, the token is refreshed once and all
    /// remaining subscriptions (including the failed one) are retried.
    pub(super) async fn subscribe_events(&mut self, session_id: &str) -> Result<(), ClientError> {
        if self.config.broadcaster_user_id.is_empty() {
            tracing::warn!("broadcaster_user_id is empty, skipping EventSub subscriptions");
            let _ = self
                .app_event_tx
                .send(AppEvent::Error(
                    "Twitch: broadcaster_user_id not set, skipping subscriptions".into(),
                ))
                .await;
            return Ok(());
        }

        let mut token_refreshed = false;

        for def in SUBSCRIPTIONS {
            let status = self.try_subscribe(def, session_id).await?;

            if status.is_success() {
                tracing::info!(sub_type = def.sub_type, "subscribed");
                let _ = self
                    .app_event_tx
                    .send(AppEvent::Info(format!(
                        "Twitch: subscribed to {}",
                        def.sub_type
                    )))
                    .await;
                continue;
            }

            // On 401, refresh the token once and retry this subscription.
            if status == reqwest::StatusCode::UNAUTHORIZED && !token_refreshed {
                tracing::warn!("got 401, attempting token refresh");
                self.refresh_token().await?;
                token_refreshed = true;

                let retry_status = self.try_subscribe(def, session_id).await?;
                if retry_status.is_success() {
                    tracing::info!(sub_type = def.sub_type, "subscribed (after refresh)");
                    let _ = self
                        .app_event_tx
                        .send(AppEvent::Info(format!(
                            "Twitch: subscribed to {} (after token refresh)",
                            def.sub_type
                        )))
                        .await;
                } else {
                    tracing::error!(
                        sub_type = def.sub_type,
                        status = %retry_status,
                        "subscription failed even after token refresh"
                    );
                    let _ = self
                        .app_event_tx
                        .send(AppEvent::Error(format!(
                            "Twitch: {} failed ({}) after token refresh",
                            def.sub_type, retry_status
                        )))
                        .await;
                }
            } else {
                tracing::error!(
                    sub_type = def.sub_type,
                    status = %status,
                    "subscription failed"
                );
                let _ = self
                    .app_event_tx
                    .send(AppEvent::Error(format!(
                        "Twitch: {} failed ({})",
                        def.sub_type, status
                    )))
                    .await;
                // Don't abort on individual subscription failures —
                // some scopes may not be authorized.
            }
        }

        Ok(())
    }
}
