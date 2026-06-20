// ---------------------------------------------------------------------------
// Discord voice-roster adapter (CDP source) — public surface + spawn() lifecycle.
//
// The toolkit reads the roster from Vesktop with NO Vencord build: `cdp.rs`
// injects a reader (`inject.js`) into the renderer over the Chrome DevTools
// protocol, and that reader pushes snapshots to the local WebSocket ingress
// (`bridge.rs`), which publishes a domain `VoiceRoster` on the watch channel the
// Overlay Feed subscribes to. `spawn()` mirrors the Overlay/Livepix shape: idle
// until `Start`, run until `Stop`/disconnect, publish `None` on the way out so
// widgets clear. See `docs/arch/14-infra-discord.md`.
// ---------------------------------------------------------------------------

mod activity;
mod bridge;
mod bridge_payload;
mod cdp;

use tokio::sync::{mpsc, watch};

use crate::domain::VoiceRoster;

/// Start/Stop toggles delivered from the TUI Services pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscordCommand {
    Start,
    Stop,
}

/// Lifecycle status surfaced back to the TUI Discord row.
#[derive(Debug, Clone)]
pub enum DiscordStatus {
    Stopped,
    /// Listening + injecting (channel name once a roster snapshot has arrived).
    Running { channel: Option<String> },
    /// A debugging line for the TUI event log (connect/disconnect, channel hop,
    /// joins/leaves, speaking) — does not change the Services-row state.
    Activity(String),
    Error(String),
}

/// Bridge ingress + CDP injection configuration.
#[derive(Debug, Clone)]
pub struct DiscordConfig {
    /// Local WS port the injected reader pushes roster snapshots to.
    pub bridge_port: u16,
    /// Vesktop DevTools port to inject over (`--remote-debugging-port`).
    pub cdp_port: u16,
    /// Log per-member speaking start/stop to the TUI (noisy; off by default).
    pub log_speaking: bool,
}

/// Spawn the Discord roster adapter as a tokio task.
///
/// Idles until `Start`, then runs the bridge ingress + CDP injector until `Stop`
/// or a fatal error, publishing `None` on the way out. Same shape as
/// `livepix::spawn` / the Overlay server.
pub fn spawn(
    mut cmd_rx: mpsc::Receiver<DiscordCommand>,
    status_tx: mpsc::Sender<DiscordStatus>,
    voice_roster_tx: watch::Sender<Option<VoiceRoster>>,
    config: DiscordConfig,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            match cmd_rx.recv().await {
                Some(DiscordCommand::Start) => {}
                Some(DiscordCommand::Stop) => continue,
                None => return, // channel closed
            }
            run_session(&mut cmd_rx, &status_tx, &voice_roster_tx, &config).await;
        }
    })
}

/// Run one session, racing it against a `Stop` command. Publishes `None` and
/// `Stopped` on the way out whether it ended by Stop or a fatal error.
async fn run_session(
    cmd_rx: &mut mpsc::Receiver<DiscordCommand>,
    status_tx: &mpsc::Sender<DiscordStatus>,
    voice_roster_tx: &watch::Sender<Option<VoiceRoster>>,
    config: &DiscordConfig,
) {
    let _ = status_tx.send(DiscordStatus::Running { channel: None }).await;

    tokio::select! {
        result = session(config, status_tx, voice_roster_tx) => {
            if let Err(e) = result {
                let _ = status_tx.send(DiscordStatus::Error(e.to_string())).await;
            }
        }
        _ = wait_for_stop(cmd_rx) => {}
    }

    let _ = voice_roster_tx.send(None);
    let _ = status_tx.send(DiscordStatus::Stopped).await;
}

/// Run the bridge ingress and the CDP injector together; either ending ends the
/// session (e.g. the ingress failing to bind its port). Both emit
/// `DiscordStatus::Activity` lines to the TUI for debugging.
async fn session(
    config: &DiscordConfig,
    status_tx: &mpsc::Sender<DiscordStatus>,
    voice_roster_tx: &watch::Sender<Option<VoiceRoster>>,
) -> std::io::Result<()> {
    tokio::select! {
        result = bridge::serve(config.bridge_port, config.log_speaking, status_tx, voice_roster_tx) => result,
        result = cdp::inject_loop(config.cdp_port, config.bridge_port, status_tx) => result,
    }
}

/// Resolve once a `Stop` arrives (or the command channel closes).
async fn wait_for_stop(cmd_rx: &mut mpsc::Receiver<DiscordCommand>) {
    loop {
        match cmd_rx.recv().await {
            Some(DiscordCommand::Stop) | None => return,
            Some(DiscordCommand::Start) => {} // already running, ignore
        }
    }
}
