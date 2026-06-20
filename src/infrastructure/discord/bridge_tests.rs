// End-to-end over a real loopback socket: a client connects, pushes one snapshot
// frame, and the server publishes the mapped roster on the watch channel.

use super::*;
use futures_util::SinkExt;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, watch};

#[tokio::test]
async fn snapshot_over_ws_publishes_roster() {
    let (tx, mut rx) = watch::channel::<Option<crate::domain::VoiceRoster>>(None);
    let (status_tx, mut status_rx) = mpsc::channel::<DiscordStatus>(64);
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let _ = accept_loop(listener, false, &status_tx, &tx).await;
    });

    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}/"))
        .await
        .unwrap();
    let snapshot = r#"{"channelId":"99","channelName":"general",
        "members":[{"userId":"5","displayName":"ferris","speaking":true}]}"#;
    ws.send(Message::Text(snapshot.to_owned().into())).await.unwrap();

    // Bounded wait so a regression fails instead of hanging the suite.
    tokio::time::timeout(std::time::Duration::from_secs(2), rx.changed())
        .await
        .expect("roster published in time")
        .unwrap();

    let roster = rx.borrow().clone().expect("Some(roster)");
    assert_eq!(roster.channel_id.as_deref(), Some("99"));
    assert_eq!(roster.members.len(), 1);
    assert!(roster.members[0].speaking);

    // The TUI also gets debugging activity: a connect line + the channel summary.
    let mut lines = Vec::new();
    while let Ok(DiscordStatus::Activity(line)) = status_rx.try_recv() {
        lines.push(line);
    }
    assert!(lines.iter().any(|l| l.contains("reader connected")), "got: {lines:?}");
    assert!(lines.iter().any(|l| l.contains("ferris")), "got: {lines:?}");
}
