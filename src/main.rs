mod alerts;
mod app;
mod config;
mod privacy;
mod stream;
mod tui;
mod waybar;

use hyprland::async_closure;
use hyprland::event_listener::AsyncEventListener;
use obws::Client;
use std::sync::Arc;

#[tokio::main(flavor = "current_thread")]
async fn main() -> hyprland::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cfg = config::load().expect("failed to load config");

    let obs_password = if cfg.obs.password.is_empty() {
        None
    } else {
        Some(cfg.obs.password.as_str())
    };

    let client = Arc::new(
        Client::connect(&cfg.obs.host, cfg.obs.port, obs_password)
            .await
            .unwrap(),
    );

    // clone once to move into the closure
    let _client_for_events = Arc::clone(&client);

    let mut event_listener = AsyncEventListener::new();

    event_listener.add_active_window_changed_handler(async_closure! {
        move |data| {
            println!("{data:#?}")
        }
    });

    event_listener.start_listener_async().await?;

    Ok(())
}
