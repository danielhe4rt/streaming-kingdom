mod alerts;
mod app;
mod config;
mod privacy;
mod stream;
mod tui;
mod waybar;

use std::io;
use std::sync::Arc;

use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cfg = config::load().map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    // Ensure the stream data file exists so waybar custom modules don't fail
    waybar::ensure_data_file()?;

    let mut app = app::AppState::new();

    // Start the waybar event-writer task (writes stream_data.json on each event)
    let event_rx = app.subscribe_events();
    tokio::spawn(waybar::event_writer(event_rx));

    // Start the Twitch EventSub WebSocket client (runs in background)
    if !cfg.twitch.oauth_token.is_empty() && !cfg.twitch.client_id.is_empty() {
        let twitch = stream::TwitchClient::new(cfg.twitch.clone(), app.event_tx.clone());
        tokio::spawn(twitch.run());
        tracing::info!("twitch EventSub client started");
    } else {
        tracing::warn!("twitch config incomplete — EventSub client not started");
    }

    // Create privacy monitor channels and spawn its task
    let (privacy_cmd_tx, privacy_cmd_rx) = mpsc::channel::<privacy::PrivacyCommand>(16);
    let (privacy_status_tx, privacy_status_rx) = mpsc::channel::<privacy::PrivacyStatus>(64);

    let _privacy_handle = privacy::spawn(
        privacy_cmd_rx,
        privacy_status_tx,
        Arc::new(cfg.privacy),
        Arc::from(cfg.obs.host.as_str()),
        cfg.obs.port,
        Arc::from(cfg.obs.password.as_str()),
        Arc::from(cfg.obs.capture_source.as_str()),
    );

    // Run the TUI with waybar config merge + privacy process management
    tui::run(&mut app, &cfg.waybar.output, privacy_cmd_tx, privacy_status_rx).await
}
