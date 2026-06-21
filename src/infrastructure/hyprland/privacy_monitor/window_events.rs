//! Watching Hyprland window lifecycle: a snapshot of open windows plus a
//! listener task that forwards open/close/title-change events.

use std::collections::HashMap;

use hyprland::prelude::*;
use tokio::sync::mpsc;

/// A window lifecycle event observed from the Hyprland listener.
#[derive(Debug)]
pub(super) enum WindowEvent {
    Opened { address: String, title: String },
    Closed { address: String },
    TitleChanged { address: String, title: String },
}

/// Snapshot every window already open, keyed by address → title.
///
/// We track ALL open windows so blur stays active when a sensitive file is open
/// on any monitor, not just the focused one.
pub(super) async fn snapshot_open_windows() -> HashMap<String, String> {
    let mut windows = HashMap::new();
    match hyprland::data::Clients::get_async().await {
        Ok(clients) => {
            for client in clients {
                windows.insert(client.address.to_string(), client.title);
            }
        }
        Err(e) => {
            tracing::warn!("failed to query existing windows: {e}");
        }
    }
    windows
}

/// Spawn the Hyprland event listener, forwarding each lifecycle event on `tx`.
pub(super) fn spawn_window_listener(tx: mpsc::Sender<WindowEvent>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut listener = hyprland::event_listener::AsyncEventListener::new();

        // Track newly opened windows.
        let tx_open = tx.clone();
        listener.add_window_opened_handler(move |data| {
            let tx = tx_open.clone();
            Box::pin(async move {
                let _ = tx
                    .send(WindowEvent::Opened {
                        address: data.window_address.to_string(),
                        title: data.window_title,
                    })
                    .await;
            })
        });

        // Track closed windows.
        let tx_close = tx.clone();
        listener.add_window_closed_handler(move |addr| {
            let tx = tx_close.clone();
            Box::pin(async move {
                let _ = tx
                    .send(WindowEvent::Closed {
                        address: addr.to_string(),
                    })
                    .await;
            })
        });

        // Track title changes (e.g. editor opens a different file).
        let tx_title = tx.clone();
        listener.add_window_title_changed_handler(move |data| {
            let tx = tx_title.clone();
            Box::pin(async move {
                let _ = tx
                    .send(WindowEvent::TitleChanged {
                        address: data.address.to_string(),
                        title: data.title,
                    })
                    .await;
            })
        });

        if let Err(e) = listener.start_listener_async().await {
            tracing::error!("hyprland listener error: {e}");
        }
    })
}
