use tokio::sync::mpsc;
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::message::ServerMessage;
use twitch_irc::{ClientConfig, SecureTCPTransport, TwitchIRCClient};

pub struct ChatMessage {
    pub username: String,
    pub text: String,
    pub channel: String,
}

pub struct ChatClient {
    channel: String,
    oauth_token: Option<String>,
    login_name: Option<String>,
}

impl ChatClient {
    pub fn new(
        channel: String,
        login_name: Option<String>,
        oauth_token: Option<String>,
    ) -> Self {
        Self {
            channel,
            oauth_token,
            login_name,
        }
    }

    pub async fn run(self, tx: mpsc::Sender<ChatMessage>) {
        let config = match (&self.login_name, &self.oauth_token) {
            (Some(login), Some(token)) => ClientConfig::new_simple(
                StaticLoginCredentials::new(login.clone(), Some(token.clone())),
            ),
            _ => ClientConfig::default(),
        };

        let (mut incoming, client) =
            TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);

        if let Err(e) = client.join(self.channel.clone()) {
            tracing::error!("failed to join #{}: {e}", self.channel);
            return;
        }

        while let Some(message) = incoming.recv().await {
            if let ServerMessage::Privmsg(msg) = message {
                let _ = tx
                    .send(ChatMessage {
                        username: msg.sender.name,
                        text: msg.message_text,
                        channel: msg.channel_login,
                    })
                    .await;
            }
        }
    }
}
