use tokio::sync::mpsc;

use crate::domain::AppEvent;

/// Spawn a dedicated Hyprland event listener that forwards window lifecycle
/// events to the TUI event log via `tx`.
pub fn spawn(tx: mpsc::Sender<AppEvent>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut listener = hyprland::event_listener::AsyncEventListener::new();

        // Window opened
        let tx_open = tx.clone();
        listener.add_window_opened_handler(move |data| {
            let tx = tx_open.clone();
            Box::pin(async move {
                let _ = tx
                    .send(AppEvent::WindowOpened {
                        title: data.window_title,
                    })
                    .await;
            })
        });

        // Window closed
        let tx_close = tx.clone();
        listener.add_window_closed_handler(move |addr| {
            let tx = tx_close.clone();
            Box::pin(async move {
                let _ = tx
                    .send(AppEvent::WindowClosed {
                        address: addr.to_string(),
                    })
                    .await;
            })
        });

        // Window title changed
        let tx_title = tx.clone();
        listener.add_window_title_changed_handler(move |data| {
            let tx = tx_title.clone();
            Box::pin(async move {
                let _ = tx
                    .send(AppEvent::WindowTitleChanged { title: data.title })
                    .await;
            })
        });

        // Workspace changed
        let tx_ws = tx.clone();
        listener.add_workspace_changed_handler(move |data| {
            let tx = tx_ws.clone();
            Box::pin(async move {
                let _ = tx
                    .send(AppEvent::WorkspaceChanged {
                        name: data.name.to_string(),
                    })
                    .await;
            })
        });

        // Active monitor changed
        let tx_mon = tx.clone();
        listener.add_active_monitor_changed_handler(move |data| {
            let tx = tx_mon.clone();
            Box::pin(async move {
                let _ = tx
                    .send(AppEvent::MonitorFocused {
                        monitor: data.monitor_name,
                    })
                    .await;
            })
        });

        // Window moved
        let tx_moved = tx.clone();
        listener.add_window_moved_handler(move |data| {
            let tx = tx_moved.clone();
            Box::pin(async move {
                let _ = tx
                    .send(AppEvent::WindowMoved {
                        workspace: data.workspace_name.to_string(),
                    })
                    .await;
            })
        });

        if let Err(e) = listener.start_listener_async().await {
            tracing::error!("hyprland event listener error: {e}");
        }
    })
}
