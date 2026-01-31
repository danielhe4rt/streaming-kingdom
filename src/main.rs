mod alerts;
mod privacy;
mod stream;
mod tui;
mod waybar;

use hyprland::async_closure;
use hyprland::event_listener::{AsyncEventListener, EventListener};
use obws::Client;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() -> hyprland::Result<()> {
    let client = Arc::new(
        Client::connect("localhost", 4455, Some("123123"))
            .await
            .unwrap(),
    );

    // clone once to move into the closure
    let client_for_events = Arc::clone(&client);

    let mut event_listener = AsyncEventListener::new();

    event_listener.add_active_window_changed_handler(async_closure! {
        move |data| {
            println!("{data:#?}")

        }
    });


    event_listener.start_listener_async().await?;

    Ok(())
}
