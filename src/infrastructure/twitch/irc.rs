use std::time::Duration;

use tokio::sync::{broadcast, mpsc};
use tokio::time;
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::message::{PrivmsgMessage, ServerMessage};
use twitch_irc::{ClientConfig, SecureTCPTransport, TwitchIRCClient};

use crate::domain::{AppEvent, ChatMessage};

const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(120);
const INITIAL_RECONNECT_DELAY: Duration = Duration::from_secs(1);

pub struct ChatClient {
    channel: String,
    oauth_token: Option<String>,
    login_name: Option<String>,
}

impl ChatClient {
    pub fn new(channel: String, login_name: Option<String>, oauth_token: Option<String>) -> Self {
        Self {
            channel,
            oauth_token,
            login_name,
        }
    }

    pub async fn run(self, tx: broadcast::Sender<ChatMessage>, event_tx: mpsc::Sender<AppEvent>) {
        let mut delay = INITIAL_RECONNECT_DELAY;

        loop {
            self.connect_and_listen(&tx, &event_tx).await;

            let _ = event_tx
                .send(AppEvent::Error(format!(
                    "Twitch chat disconnected, reconnecting in {delay:?}"
                )))
                .await;

            time::sleep(delay).await;
            delay = (delay * 2).min(MAX_RECONNECT_DELAY);
        }
    }

    async fn connect_and_listen(
        &self,
        tx: &broadcast::Sender<ChatMessage>,
        event_tx: &mpsc::Sender<AppEvent>,
    ) {
        let authenticated = self.login_name.is_some() && self.oauth_token.is_some();

        let config = match (&self.login_name, &self.oauth_token) {
            (Some(login), Some(token)) => {
                let _ = event_tx
                    .send(AppEvent::Info(format!(
                        "Twitch chat authenticating as {login}"
                    )))
                    .await;
                ClientConfig::new_simple(StaticLoginCredentials::new(
                    login.clone(),
                    Some(token.clone()),
                ))
            }
            _ => {
                let _ = event_tx
                    .send(AppEvent::Info(
                        "Twitch chat connecting anonymously (no credentials)".into(),
                    ))
                    .await;
                ClientConfig::default()
            }
        };

        let (mut incoming, client) =
            TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);

        if let Err(e) = client.join(self.channel.clone()) {
            let _ = event_tx
                .send(AppEvent::Error(format!(
                    "Twitch chat failed to join #{}: {e}",
                    self.channel
                )))
                .await;
            return;
        }

        let _ = event_tx
            .send(AppEvent::Info(format!(
                "Twitch chat joined #{}{}",
                self.channel,
                if authenticated {
                    " (authenticated)"
                } else {
                    " (anonymous)"
                }
            )))
            .await;

        while let Some(message) = incoming.recv().await {
            if let ServerMessage::Privmsg(msg) = message {
                // broadcast::send errors only when there are no receivers yet;
                // safe to ignore for a fire-and-forget chat fan-out.
                let _ = tx.send(enrich_privmsg(msg));
            }
        }
    }
}

/// M1 — chat enrichment (pure): `Privmsg → ChatMessage`.
///
/// Text-only for this slice: the whole body becomes a single text fragment and
/// the author's Twitch chat colour is carried through (with a fallback). Emote
/// range splitting and badge collection arrive in later slices.
fn enrich_privmsg(msg: PrivmsgMessage) -> ChatMessage {
    let color = msg.name_color.map(|c| format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b));
    ChatMessage::from_text(
        msg.message_id,
        msg.sender.name,
        color.as_deref(),
        msg.channel_login,
        msg.message_text,
    )
}
