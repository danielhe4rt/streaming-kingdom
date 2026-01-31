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

    // Run the TUI with waybar process management
    tui::run(&mut app, &wb_config, &wb_style).await
}
