//! Livepix OAuth `client_credentials` flow and token caching.

use super::payload::OAuthToken;
use super::server_state::ServerState;

pub(super) async fn get_oauth_token(
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

    let status = response.status();
    let body = response.text().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(format!("OAuth returned {status}: {body}"));
    }

    let token_data: OAuthToken = serde_json::from_str(&body)
        .map_err(|e| format!("failed to parse OAuth response: {e} -- body: {body}"))?;

    Ok(token_data.access_token)
}

/// Retrieve a cached token or re-authenticate if the cache is empty.
pub(super) async fn resolve_token(state: &ServerState) -> Result<String, String> {
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
