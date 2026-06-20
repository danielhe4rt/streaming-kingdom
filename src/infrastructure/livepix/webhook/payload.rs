//! Deserialization shapes for Livepix webhook callbacks and API responses.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct WebhookPayload {
    #[serde(default)]
    pub event: String,
    #[serde(default)]
    pub resource: Resource,
}

#[derive(Debug, Deserialize, Default)]
pub(super) struct Resource {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "type", default)]
    pub resource_type: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct OAuthToken {
    pub access_token: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct MessageResponse {
    pub data: MessageData,
}

#[derive(Debug, Deserialize)]
pub(super) struct MessageData {
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub amount: i64,
    #[serde(default)]
    pub currency: String,
}
