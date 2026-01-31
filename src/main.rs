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

    let mut app = app::AppState::new();
    tui::run(&mut app).await
}
