mod application;
mod domain;
mod infrastructure;
mod presentation;

use std::io;
use std::path::Path;
use std::sync::Arc;

use tokio::sync::mpsc;

use domain::AppEvent;

/// Sync Python files from project to ~/.config/streams-toolkit/
fn sync_python_files() -> io::Result<()> {
    let project_dir = Path::new("/home/danielhe4rt/dev/lives/streams-toolkit");
    let config_dir = Path::new("/home/danielhe4rt/.config/streams-toolkit");

    // Files to sync: (source_relative_path, dest_relative_path)
    let files_to_sync = vec![("scripts/stream_events.py", "scripts/stream_events.py")];

    for (src_rel, dst_rel) in files_to_sync {
        let src = project_dir.join(src_rel);
        let dst = config_dir.join(dst_rel);

        // Check if files differ or destination doesn't exist
        let needs_sync = if !dst.exists() {
            true
        } else {
            // Compare file contents
            let src_content = std::fs::read(&src)?;
            let dst_content = std::fs::read(&dst)?;
            src_content != dst_content
        };

        if needs_sync {
            println!("⚠️  {} differs - syncing...", src_rel);
            // Ensure parent directory exists
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&src, &dst)?;
            println!("✅ Synced {}", src_rel);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // Sync Python files on startup (project files are source of truth)
    if let Err(e) = sync_python_files() {
        eprintln!("Warning: Failed to sync Python files: {}", e);
    }

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let mut cfg = application::config::load().map_err(|e| io::Error::other(e.to_string()))?;

    // Ensure the stream data file exists so waybar custom modules don't fail
    infrastructure::waybar::ensure_data_file()?;

    // ── Twitch token validation / OAuth flow ──────────────────────────────
    // If client_id + client_secret are configured, ensure we have a valid token.
    // Flow: validate existing token → try refresh → browser OAuth as last resort.
    if !cfg.twitch.client_id.is_empty() && !cfg.twitch.client_secret.is_empty() {
        let token_valid =
            infrastructure::twitch::auth::validate_token(&cfg.twitch.oauth_token).await;

        if !token_valid {
            tracing::info!("Twitch token missing or invalid, attempting to obtain a valid token");

            // Try refreshing first if we have a refresh_token.
            let mut refreshed = false;
            if !cfg.twitch.refresh_token.is_empty() {
                tracing::info!("attempting token refresh...");
                match refresh_twitch_token(&cfg.twitch).await {
                    Ok((access, refresh)) => {
                        cfg.twitch.oauth_token = access;
                        cfg.twitch.refresh_token = refresh;
                        refreshed = true;
                        tracing::info!("token refreshed successfully");
                    }
                    Err(e) => {
                        tracing::warn!("token refresh failed: {e}");
                    }
                }
            }

            // If refresh didn't work, run the browser OAuth flow.
            if !refreshed {
                tracing::info!("opening browser for Twitch authorization...");
                match infrastructure::twitch::auth::authenticate(
                    &cfg.twitch.client_id,
                    &cfg.twitch.client_secret,
                )
                .await
                {
                    Ok(tokens) => {
                        cfg.twitch.oauth_token = tokens.access_token;
                        cfg.twitch.refresh_token = tokens.refresh_token;
                        tracing::info!("Twitch authorization completed");
                    }
                    Err(e) => {
                        tracing::error!("Twitch authorization failed: {e}");
                        eprintln!("Warning: Twitch authorization failed: {e}");
                    }
                }
            }

            // Persist whatever tokens we obtained.
            if !cfg.twitch.oauth_token.is_empty()
                && let Err(e) = application::config::save_twitch_tokens(
                    &cfg.twitch.oauth_token,
                    &cfg.twitch.refresh_token,
                )
            {
                tracing::warn!("failed to save tokens: {e}");
            }
        }
    }

    let mut app = application::AppState::new(&cfg.event_log);
    app.log_event(AppEvent::Info("streams-toolkit started".into()));
    app.log_event(AppEvent::Info("Config loaded".into()));

    // Start the waybar event-writer task (writes stream_data.json on each event)
    let event_rx = app.subscribe_events();
    tokio::spawn(infrastructure::waybar::event_writer(event_rx));

    // Start the Twitch EventSub WebSocket client (runs in background)
    // WebSocket transport requires a user access token — app tokens won't work.
    let (twitch_event_tx, twitch_event_rx) = mpsc::channel::<AppEvent>(64);

    let chat_event_tx = twitch_event_tx.clone();

    if !cfg.twitch.oauth_token.is_empty() && !cfg.twitch.client_id.is_empty() {
        let twitch = infrastructure::twitch::TwitchClient::new(
            cfg.twitch.clone(),
            app.event_tx.clone(),
            twitch_event_tx,
        );
        tokio::spawn(twitch.run());
        app.log_event(AppEvent::Info("Twitch EventSub connecting".into()));
    } else {
        app.log_event(AppEvent::Info(
            "Twitch not configured (need oauth_token + client_id)".into(),
        ));
    }

    // Create privacy monitor channels and spawn its task
    let (privacy_cmd_tx, privacy_cmd_rx) =
        mpsc::channel::<infrastructure::hyprland::PrivacyCommand>(16);
    let (privacy_status_tx, privacy_status_rx) =
        mpsc::channel::<infrastructure::hyprland::PrivacyStatus>(64);

    let _privacy_handle = infrastructure::hyprland::privacy_monitor::spawn(
        privacy_cmd_rx,
        privacy_status_tx,
        Arc::new(cfg.privacy),
        Arc::from(cfg.obs.host.as_str()),
        cfg.obs.port,
        Arc::from(cfg.obs.password.as_str()),
        Arc::from(cfg.obs.capture_source.as_str()),
    );

    // Spawn TTS worker (returns None if API key is empty)
    let tts_tx = infrastructure::elevenlabs::spawn(cfg.livepix.tts.clone());
    let tts_available = tts_tx.is_some();

    // Create Livepix webhook server channels and spawn its task
    let (livepix_cmd_tx, livepix_cmd_rx) =
        mpsc::channel::<infrastructure::livepix::LivepixCommand>(16);
    let (livepix_status_tx, livepix_status_rx) =
        mpsc::channel::<infrastructure::livepix::LivepixStatus>(64);

    let _livepix_handle = infrastructure::livepix::spawn(
        livepix_cmd_rx,
        livepix_status_tx,
        app.event_tx.clone(),
        tts_tx,
        cfg.livepix.clone(),
    );

    // Spawn the Overlay HTTP server as a toggleable Output (presentation/http).
    // Unlike the tracer-bullet slice it is no longer always-on: it idles until
    // the TUI sends Start (Overlays toggle), then binds 127.0.0.1:<overlays.port>.
    let (overlays_cmd_tx, overlays_cmd_rx) =
        mpsc::channel::<presentation::http::OverlayCommand>(16);
    let (overlays_status_tx, overlays_status_rx) =
        mpsc::channel::<presentation::http::OverlayStatus>(64);
    let _overlays_handle = presentation::http::spawn(
        overlays_cmd_rx,
        overlays_status_tx,
        app.chat_tx.clone(),
        app.event_tx.clone(),
        cfg.overlays.port,
    );
    app.log_event(AppEvent::Info(format!(
        "Overlays Output ready (toggle to serve on http://127.0.0.1:{}/overlay/chat)",
        cfg.overlays.port
    )));

    // Resolve Twitch chat badges once at startup (Helix global + channel) into
    // a set/version → url map the IRC adapter looks up per message (M2).
    let badge_map = if !cfg.twitch.oauth_token.is_empty() && !cfg.twitch.client_id.is_empty() {
        infrastructure::twitch::badges::fetch(
            &reqwest::Client::new(),
            &cfg.twitch.client_id,
            &cfg.twitch.oauth_token,
            &cfg.twitch.broadcaster_user_id,
        )
        .await
    } else {
        infrastructure::twitch::BadgeMap::default()
    };

    // Spawn Twitch IRC chat client (fans out over the chat broadcast channel).
    let chat_tx = app.chat_tx.clone();
    if !cfg.twitch.channel.is_empty() {
        // Pass credentials if available (channel name as login, oauth_token for auth)
        let login_name = if !cfg.twitch.channel.is_empty() && !cfg.twitch.oauth_token.is_empty() {
            Some(cfg.twitch.channel.clone())
        } else {
            None
        };
        let oauth_token = if !cfg.twitch.oauth_token.is_empty() {
            Some(cfg.twitch.oauth_token.clone())
        } else {
            None
        };
        let chat = infrastructure::twitch::ChatClient::new(
            cfg.twitch.channel.clone(),
            login_name,
            oauth_token,
            badge_map,
        );
        tokio::spawn(chat.run(chat_tx, chat_event_tx));
        app.log_event(AppEvent::Info(format!(
            "Twitch chat connecting to #{}",
            cfg.twitch.channel
        )));
    } else {
        app.log_event(AppEvent::Info(
            "Twitch chat not configured (no channel)".into(),
        ));
    }

    // Spawn Hyprland event listener for the event log
    let (hyprland_tx, hyprland_rx) = mpsc::channel::<AppEvent>(128);
    let _hyprland_handle = infrastructure::hyprland::spawn(hyprland_tx);

    // Run the TUI nav shell with all integration channels bundled.
    let channels = presentation::RunChannels {
        privacy_cmd_tx,
        privacy_status_rx,
        livepix_cmd_tx,
        livepix_status_rx,
        overlays_cmd_tx,
        overlays_status_rx,
        hyprland_rx,
        twitch_event_rx,
    };
    presentation::run(
        &mut app,
        &cfg.waybar.output,
        channels,
        &cfg.event_log,
        &cfg.twitch.channel,
        tts_available,
        cfg.overlays.port,
    )
    .await
}

/// Standalone token refresh used at startup (before TwitchClient exists).
async fn refresh_twitch_token(
    twitch: &application::config::TwitchConfig,
) -> Result<(String, String), String> {
    let resp = reqwest::Client::new()
        .post("https://id.twitch.tv/oauth2/token")
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", twitch.refresh_token.as_str()),
            ("client_id", twitch.client_id.as_str()),
            ("client_secret", twitch.client_secret.as_str()),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("refresh failed: {body}"));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let access = body["access_token"]
        .as_str()
        .ok_or("no access_token")?
        .to_string();
    let refresh = body["refresh_token"]
        .as_str()
        .ok_or("no refresh_token")?
        .to_string();

    Ok((access, refresh))
}
