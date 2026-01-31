mod alerts;
mod app;
mod config;
mod privacy;
mod stream;
mod tui;
mod waybar;

use std::io;

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cfg = config::load().map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    // Generate waybar config files (always, so they're ready when toggled on)
    let (wb_config, wb_style) = waybar::write_config_files(&cfg.waybar.output)?;

    let mut app = app::AppState::new();

    // Start the waybar event-writer task (writes stream_data.json on each event)
    let event_rx = app.subscribe_events();
    tokio::task::spawn_local(waybar::event_writer(event_rx));

    // Start the Twitch EventSub WebSocket client (runs in background)
    if !cfg.twitch.oauth_token.is_empty() && !cfg.twitch.client_id.is_empty() {
        let twitch = stream::TwitchClient::new(cfg.twitch.clone(), app.event_tx.clone());
        tokio::task::spawn_local(twitch.run());
        tracing::info!("twitch EventSub client started");
    } else {
        tracing::warn!("twitch config incomplete — EventSub client not started");
    }

    // Run the TUI with waybar process management
    tui::run(&mut app, &wb_config, &wb_style).await
}
