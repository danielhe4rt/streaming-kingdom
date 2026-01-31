mod alerts;
mod app;
mod config;
mod hyprland;
mod livepix;
mod privacy;
mod stream;
mod tui;
mod waybar;

use std::io;
use std::path::Path;
use std::sync::Arc;

use tokio::sync::mpsc;

use app::AppEvent;

/// Sync Python files from project to ~/.config/streams-toolkit/
fn sync_python_files() -> io::Result<()> {
    let project_dir = Path::new("/home/danielhe4rt/dev/lives/streams-toolkit");
    let config_dir = Path::new("/home/danielhe4rt/.config/streams-toolkit");
    
    // Files to sync: (source_relative_path, dest_relative_path)
    let files_to_sync = vec![
        ("scripts/stream_events.py", "scripts/stream_events.py"),
    ];
    
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

    // Spawn TTS worker (returns None if API key is empty)
    let tts_tx = livepix::tts::spawn(cfg.livepix.tts.clone());

    // Create Livepix webhook server channels and spawn its task
    let (livepix_cmd_tx, livepix_cmd_rx) = mpsc::channel::<livepix::LivepixCommand>(16);
    let (livepix_status_tx, livepix_status_rx) = mpsc::channel::<livepix::LivepixStatus>(64);

    let _livepix_handle = livepix::spawn(
        livepix_cmd_rx,
        livepix_status_tx,
        app.event_tx.clone(),
        tts_tx,
        cfg.livepix.clone(),
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
        livepix_cmd_tx,
        livepix_status_rx,
        &cfg.event_log,
        hyprland_rx,
    )
    .await
}
