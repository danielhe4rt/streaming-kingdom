use axum::routing::get;
use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
struct AppState {
    client_id: String,
    client_secret: String,
    http_client: reqwest::Client,
    token: Arc<Mutex<Option<String>>>,
    elevenlabs_key: String,
}

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
    id: String,
    #[serde(default)]
    message: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    amount: i64,
    #[serde(default)]
    currency: String,
    #[serde(default)]
    proof: String,
    #[serde(default)]
    reference: String,
    #[serde(default)]
    flagged: bool,
    #[serde(default)]
    createdAt: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("\n🔐 Initializing Webhook Server...\n");

    // Load credentials from .env
    dotenv::dotenv().ok();
    let client_id = std::env::var("LIVEPIX_CLIENT_ID").expect("LIVEPIX_CLIENT_ID not set");
    let client_secret = std::env::var("LIVEPIX_CLIENT_SECRET").expect("LIVEPIX_CLIENT_SECRET not set");
    let elevenlabs_key = std::env::var("ELEVENLABS_API_KEY").unwrap_or_else(|_| {
        println!("⚠️  ELEVENLABS_API_KEY not set - TTS will be disabled");
        String::new()
    });

    println!("✅ Loaded credentials from .env");
    if !elevenlabs_key.is_empty() {
        println!("✅ ElevenLabs TTS enabled");
    }

    let http_client = reqwest::Client::new();

    // Authenticate immediately at startup
    println!("🔐 Authenticating with OAuth...");
    let token = match get_oauth_token_sync(&http_client, &client_id, &client_secret).await {
        Ok(t) => {
            println!("✅ OAuth authentication successful!");
            println!("   Token: {}...\n", &t[..40.min(t.len())]);
            t
        }
        Err(e) => {
            eprintln!("❌ Failed to authenticate: {}", e);
            std::process::exit(1);
        }
    };

    let state = AppState {
        client_id,
        client_secret,
        http_client,
        token: Arc::new(Mutex::new(Some(token))),
        elevenlabs_key,
    };

    let app = Router::new()
        .route("/webhooks", post(
            |body: axum::extract::Json<serde_json::Value>| async move {
                handle_webhook(body, state.clone()).await
            }
        ))
        .route("/webhooks", get(|| async { "Webhook endpoint ready" }))
        .route("/", axum::routing::get(|| async { "Webhook Server Running" }));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    println!("🚀 Webhook Server Started!");
    println!("📍 Listening on http://localhost:8000");
    println!("📬 POST to http://localhost:8000/webhooks to receive webhooks\n");

    axum::serve(listener, app).await.unwrap();
}

async fn get_oauth_token_sync(
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

    let token_data: OAuthToken = response.json().await
        .map_err(|e| e.to_string())?;

    Ok(token_data.access_token)
}

async fn get_oauth_token(state: &AppState) -> Result<String, String> {
    // Get the cached token (should always exist since we auth at startup)
    {
        let token = state.token.lock().await;
        if let Some(t) = token.as_ref() {
            return Ok(t.clone());
        }
    }

    // Fallback: Re-authenticate if token is missing (shouldn't happen)
    println!("⚠️ Token cache empty, re-authenticating...");
    let new_token = get_oauth_token_sync(&state.http_client, &state.client_id, &state.client_secret).await?;

    {
        let mut token = state.token.lock().await;
        *token = Some(new_token.clone());
    }

    Ok(new_token)
}

async fn text_to_speech(
    state: &AppState,
    text: &str,
) -> Result<(), String> {
    if state.elevenlabs_key.is_empty() {
        println!("⚠️  ElevenLabs API key not configured - skipping TTS");
        return Ok(());
    }

    println!("\n🔊 Converting to speech with ElevenLabs...");

    // Use ElevenLabs API directly
    // Voice ID for Rachel: 21m00Tcm4TlvDq8ikWAM
    let voice_id = "21m00Tcm4TlvDq8ikWAM";
    let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{}", voice_id);

    let payload = serde_json::json!({
        "text": text,
        "model_id": "eleven_monolingual_v1",
        "voice_settings": {
            "stability": 0.5,
            "similarity_boost": 0.75
        }
    });

    match state
        .http_client
        .post(&url)
        .header("xi-api-key", &state.elevenlabs_key)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                match response.bytes().await {
                    Ok(audio_bytes) => {
                        println!("✅ TTS Generated! ({} bytes)", audio_bytes.len());

                        // Save to file
                        let filename = format!("message_{}.mp3", chrono::Local::now().timestamp());
                        std::fs::write(&filename, audio_bytes)
                            .map_err(|e| e.to_string())?;
                        println!("💾 Saved to: {}", filename);
                        println!("🔊 Audio file ready to play!");
                    }
                    Err(e) => {
                        println!("❌ Failed to read audio bytes: {}", e);
                    }
                }
            } else {
                println!("❌ ElevenLabs API error: {}", response.status());
                if let Ok(text) = response.text().await {
                    println!("   {}", text);
                }
            }
        }
        Err(e) => {
            println!("❌ TTS Request error: {}", e);
        }
    }

    Ok(())
}

async fn fetch_message(
    state: &AppState,
    message_id: &str,
) -> Result<MessageData, String> {
    let token = get_oauth_token(state).await?;

    println!("📨 Fetching message details: {}", message_id);

    let response = state
        .http_client
        .get(&format!("https://api.livepix.gg/v2/messages/{}", message_id))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status();
    let body_text = response.text().await
        .map_err(|e| e.to_string())?;

    match serde_json::from_str::<MessageResponse>(&body_text) {
        Ok(response) => Ok(response.data),
        Err(e) => {
            println!("   ❌ Response Status: {}", status);
            println!("   ❌ Parse Error: {}", e);
            println!("   ❌ Response Body: {}", body_text);
            Err(format!("Failed to parse message: {}", e))
        }
    }
}

async fn handle_webhook(
    axum::extract::Json(webhook): axum::extract::Json<serde_json::Value>,
    state: AppState,
) -> impl IntoResponse {

    let body_str = webhook.to_string();

    println!("\n╔════════════════════════════════════════════╗");
    println!("║         🔔 WEBHOOK RECEIVED 🔔             ║");
    println!("╚════════════════════════════════════════════╝");

    println!("\n📦 PAYLOAD ({} bytes):", body_str.len());

    // Try to parse as WebhookPayload
    match serde_json::from_value::<WebhookPayload>(webhook.clone()) {
        Ok(parsed_webhook) => {
            // Print the raw JSON
            println!("{}", serde_json::to_string_pretty(&webhook).unwrap_or_else(|_| body_str.to_string()));

            // If this is a message event, fetch the full message
            if parsed_webhook.resource.resource_type == "message" && parsed_webhook.event == "new" {
                println!("\n💬 Message Event Detected!");
                println!("   Message ID: {}", parsed_webhook.resource.id);

                match fetch_message(&state, &parsed_webhook.resource.id).await {
                    Ok(message) => {
                        println!("\n✅ Message Details Retrieved:");
                        println!("   📝 Message: {}", message.message);
                        println!("   👤 Username: {}", message.username);
                        println!("   💵 Amount: {} {}", message.amount, message.currency);
                        println!("   🔖 Reference: {}", message.reference);
                        println!("   ⏰ Created: {}", message.createdAt);
                        if message.flagged {
                            println!("   ⚠️  FLAGGED: true");
                        }

                        // Generate TTS for the message
                        let _ = text_to_speech(&state, &message.message).await;
                    }
                    Err(e) => {
                        println!("❌ Failed to fetch message: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            println!("⚠️ Failed to parse JSON: {}", e);
            println!("{}", body_str);
        }
    }

    println!("\n✅ Webhook processed at {}\n", chrono::Local::now().format("%H:%M:%S%.3f"));

    (StatusCode::OK, "Webhook received and processed!")
}
