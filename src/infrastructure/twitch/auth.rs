use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::{oneshot, Mutex};

const AUTH_URL: &str = "https://id.twitch.tv/oauth2/authorize";
const TOKEN_URL: &str = "https://id.twitch.tv/oauth2/token";
const VALIDATE_URL: &str = "https://id.twitch.tv/oauth2/validate";

const REDIRECT_PORT: u16 = 3000;

/// All scopes needed by the application (EventSub + IRC).
const SCOPES: &[&str] = &[
    // IRC chat
    "chat:read",
    "chat:write",
    // EventSub
    "user:read:chat",
    "user:write:chat",
    "moderator:read:followers",
    "channel:read:subscriptions",
    "bits:read",
];

pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Deserialize)]
struct CallbackParams {
    code: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Clone)]
struct CallbackState {
    tx: Arc<Mutex<Option<oneshot::Sender<Result<String, String>>>>>,
}

/// Validate an existing OAuth token against Twitch's validate endpoint.
///
/// Returns `true` if the token is still valid.
pub async fn validate_token(token: &str) -> bool {
    if token.is_empty() {
        return false;
    }

    let http = reqwest::Client::new();
    let resp = http
        .get(VALIDATE_URL)
        .header("Authorization", format!("OAuth {token}"))
        .send()
        .await;

    match resp {
        Ok(r) => r.status().is_success(),
        Err(_) => false,
    }
}

/// Run the OAuth Authorization Code flow via a local browser redirect.
///
/// 1. Starts a temporary HTTP server on localhost:3000
/// 2. Opens the Twitch authorization URL in the user's browser
/// 3. Waits for the callback with the authorization code
/// 4. Exchanges the code for access + refresh tokens
pub async fn authenticate(
    client_id: &str,
    client_secret: &str,
) -> Result<TokenResponse, String> {
    let redirect_uri = format!("http://localhost:{REDIRECT_PORT}");
    let scope = SCOPES.join("+");

    let auth_url = format!(
        "{AUTH_URL}?response_type=code&client_id={client_id}&redirect_uri={redirect_uri}&scope={scope}&force_verify=true"
    );

    // Channel to receive the auth code from the callback handler.
    let (tx, rx) = oneshot::channel::<Result<String, String>>();
    let state = CallbackState {
        tx: Arc::new(Mutex::new(Some(tx))),
    };

    let app = Router::new()
        .route("/", get(callback_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{REDIRECT_PORT}"))
        .await
        .map_err(|e| format!("failed to bind port {REDIRECT_PORT}: {e}"))?;

    tracing::info!("auth: local callback server listening on :{REDIRECT_PORT}");

    // Graceful shutdown channel — fires once we have the auth code.
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
    });

    // Open the browser.
    tracing::info!("auth: opening browser for Twitch authorization");
    if let Err(e) = std::process::Command::new("xdg-open")
        .arg(&auth_url)
        .spawn()
    {
        // If xdg-open fails, print the URL for manual copy.
        tracing::warn!("auth: could not open browser: {e}");
        println!("\n  Open this URL in your browser to authorize:\n\n  {auth_url}\n");
    }

    // Wait for the callback.
    let code = rx
        .await
        .map_err(|_| "auth callback channel closed unexpectedly".to_string())?
        .map_err(|e| format!("authorization denied: {e}"))?;

    // Shut down the temporary server.
    let _ = shutdown_tx.send(());
    let _ = server_handle.await;

    tracing::info!("auth: exchanging authorization code for tokens");

    // Exchange the code for tokens.
    let http = reqwest::Client::new();
    let resp = http
        .post(TOKEN_URL)
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code.as_str()),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri.as_str()),
        ])
        .send()
        .await
        .map_err(|e| format!("token exchange request failed: {e}"))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("token exchange failed: {body}"));
    }

    let body: Value = resp
        .json()
        .await
        .map_err(|e| format!("failed to parse token response: {e}"))?;

    let access_token = body["access_token"]
        .as_str()
        .ok_or("no access_token in token response")?
        .to_string();

    let refresh_token = body["refresh_token"]
        .as_str()
        .ok_or("no refresh_token in token response")?
        .to_string();

    tracing::info!("auth: tokens obtained successfully");

    Ok(TokenResponse {
        access_token,
        refresh_token,
    })
}

async fn callback_handler(
    State(state): State<CallbackState>,
    Query(params): Query<CallbackParams>,
) -> Html<&'static str> {
    if let Some(sender) = state.tx.lock().await.take() {
        if let Some(error) = params.error {
            let desc = params
                .error_description
                .unwrap_or_else(|| error.clone());
            let _ = sender.send(Err(desc));
            return Html(
                "<html><body style=\"font-family:sans-serif;text-align:center;padding:60px\">\
                 <h1>Authorization Failed</h1>\
                 <p>You can close this tab.</p>\
                 </body></html>",
            );
        }

        if let Some(code) = params.code {
            let _ = sender.send(Ok(code));
            return Html(
                "<html><body style=\"font-family:sans-serif;text-align:center;padding:60px\">\
                 <h1>Authorization Successful!</h1>\
                 <p>You can close this tab and return to the terminal.</p>\
                 </body></html>",
            );
        }

        let _ = sender.send(Err("no code or error in callback".into()));
    }

    Html(
        "<html><body style=\"font-family:sans-serif;text-align:center;padding:60px\">\
         <h1>Something went wrong</h1>\
         <p>Callback already processed or missing parameters.</p>\
         </body></html>",
    )
}
