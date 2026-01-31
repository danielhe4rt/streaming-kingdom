mod alerts;
mod app;
mod config;
mod hyprland;
mod privacy;
mod stream;
mod tui;
mod waybar;

use std::io;
use std::sync::Arc;

use tokio::sync::mpsc;

use app::AppEvent;

#[tokio::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cfg = config::load().map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    // Ensure the stream data file exists so waybar custom modules don't fail
    waybar::ensure_data_file()?;

    let mut app = app::AppState::new(&cfg.event_log);
    app.log_event(AppEvent::Info("streams-toolkit started".into()));
    app.log_event(AppEvent::Info("Config loaded".into()));

    // Start the waybar event-writer task (writes stream_data.json on each event)
    let event_rx = app.subscribe_events();
    tokio::spawn(waybar::event_writer(event_rx));

    // Start the Twitch EventSub WebSocket client (runs in background)
    if !cfg.twitch.oauth_token.is_empty() && !cfg.twitch.client_id.is_empty() {
        let twitch = stream::TwitchClient::new(cfg.twitch.clone(), app.event_tx.clone());
        tokio::spawn(twitch.run());
        app.log_event(AppEvent::Info("Twitch EventSub connected".into()));
    } else {
        app.log_event(AppEvent::Info("Twitch not configured".into()));
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

    // Spawn Hyprland event listener for the event log
    let (hyprland_tx, hyprland_rx) = mpsc::channel::<AppEvent>(128);
    let _hyprland_handle = hyprland::spawn(hyprland_tx);

    // Run the TUI with waybar config merge + privacy process management
    tui::run(
        &mut app,
        &cfg.waybar.output,
        privacy_cmd_tx,
        privacy_status_rx,
        &cfg.event_log,
        hyprland_rx,
    )
    .await
}
