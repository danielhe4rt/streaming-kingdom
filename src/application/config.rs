use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{env, fs, io};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub obs: ObsConfig,
    pub twitch: TwitchConfig,
    #[serde(default)]
    pub livepix: LivepixConfig,
    pub alerts: AlertsConfig,
    pub waybar: WaybarConfig,
    pub privacy: PrivacyConfig,
    #[serde(default)]
    pub event_log: EventLogConfig,
    #[serde(default)]
    pub overlays: OverlaysConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OverlaysConfig {
    /// Port the `http` Overlay server binds on (127.0.0.1). The only
    /// configurable Overlays field for v1.
    #[serde(default = "default_overlays_port")]
    pub port: u16,
}

fn default_overlays_port() -> u16 {
    1337
}

impl Default for OverlaysConfig {
    fn default() -> Self {
        Self {
            port: default_overlays_port(),
        }
    }
}

impl OverlaysConfig {
    /// Override the port from `OVERLAYS_PORT` when set.
    pub fn apply_env_overrides(&mut self) {
        if let Ok(v) = env::var("OVERLAYS_PORT")
            && let Ok(port) = v.parse::<u16>()
        {
            self.port = port;
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ObsConfig {
    pub host: String,
    pub port: u16,
    pub password: String,
    pub capture_source: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TwitchConfig {
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
    pub oauth_token: String,
    pub refresh_token: String,
    pub broadcaster_user_id: String,
    #[serde(default)]
    pub channel: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LivepixConfig {
    pub client_id: String,
    pub client_secret: String,
    pub webhook_port: u16,
    #[serde(default)]
    pub tts: TtsConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TtsConfig {
    pub elevenlabs_api_key: String,
    pub voice_id: String,
    pub model_id: String,
    pub language_code: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AlertsConfig {
    pub overlay_url: String,
    pub browser_command: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WaybarConfig {
    pub output: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PrivacyConfig {
    pub sensitive_patterns: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EventLogConfig {
    pub show_stream: bool,
    pub show_privacy: bool,
    pub show_system: bool,
    pub show_hyprland: bool,
    #[serde(default = "default_true")]
    pub show_livepix: bool,
    #[serde(default = "default_true")]
    pub show_chat: bool,
    pub max_events: usize,
}

fn default_true() -> bool {
    true
}

impl ObsConfig {
    /// Override fields from environment variables when set.
    ///
    /// Env vars: `OBS_HOST`, `OBS_PORT`, `OBS_PASSWORD`, `OBS_CAPTURE_SOURCE`.
    pub fn apply_env_overrides(&mut self) {
        if let Ok(v) = env::var("OBS_HOST") {
            self.host = v;
        }
        if let Ok(v) = env::var("OBS_PORT")
            && let Ok(port) = v.parse::<u16>()
        {
            self.port = port;
        }
        if let Ok(v) = env::var("OBS_PASSWORD") {
            self.password = v;
        }
        if let Ok(v) = env::var("OBS_CAPTURE_SOURCE") {
            self.capture_source = v;
        }
    }
}

impl LivepixConfig {
    /// Override empty fields with environment variables from .env.
    pub fn apply_env_overrides(&mut self) {
        if self.client_id.is_empty()
            && let Ok(val) = env::var("LIVEPIX_CLIENT_ID")
        {
            self.client_id = val;
        }
        if self.client_secret.is_empty()
            && let Ok(val) = env::var("LIVEPIX_CLIENT_SECRET")
        {
            self.client_secret = val;
        }
        if self.tts.elevenlabs_api_key.is_empty()
            && let Ok(val) = env::var("ELEVENLABS_API_KEY")
        {
            self.tts.elevenlabs_api_key = val;
        }
    }
}

impl TwitchConfig {
    /// Override fields from environment variables when set.
    ///
    /// Env vars: `TWITCH_CLIENT_ID`, `TWITCH_CLIENT_SECRET`, `TWITCH_OAUTH_TOKEN`,
    /// `TWITCH_REFRESH_TOKEN`, `TWITCH_BROADCASTER_USER_ID`.
    pub fn apply_env_overrides(&mut self) {
        if let Ok(v) = env::var("TWITCH_CLIENT_ID") {
            self.client_id = v;
        }
        if let Ok(v) = env::var("TWITCH_CLIENT_SECRET") {
            self.client_secret = v;
        }
        if let Ok(v) = env::var("TWITCH_OAUTH_TOKEN") {
            self.oauth_token = v;
        }
        if let Ok(v) = env::var("TWITCH_REFRESH_TOKEN") {
            self.refresh_token = v;
        }
        if let Ok(v) = env::var("TWITCH_BROADCASTER_USER_ID") {
            self.broadcaster_user_id = v;
        }
        if let Ok(v) = env::var("TWITCH_CHANNEL") {
            self.channel = v;
        }
    }
}

impl Default for LivepixConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            webhook_port: 8000,
            tts: TtsConfig::default(),
        }
    }
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            elevenlabs_api_key: String::new(),
            voice_id: "Qrdut83w0Cr152Yb4Xn3".into(),
            model_id: "eleven_v3".into(),
            language_code: "pt".into(),
        }
    }
}

impl Default for EventLogConfig {
    fn default() -> Self {
        Self {
            show_stream: true,
            show_privacy: true,
            show_system: true,
            show_hyprland: false,
            show_livepix: true,
            show_chat: true,
            max_events: 100,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            obs: ObsConfig {
                host: "localhost".into(),
                port: 4455,
                password: String::new(),
                capture_source: "Screen Capture".into(),
            },
            twitch: TwitchConfig {
                client_id: String::new(),
                client_secret: String::new(),
                oauth_token: String::new(),
                refresh_token: String::new(),
                broadcaster_user_id: String::new(),
                channel: String::new(),
            },
            livepix: LivepixConfig::default(),
            alerts: AlertsConfig {
                overlay_url: String::new(),
                browser_command: "firefox".into(),
            },
            waybar: WaybarConfig {
                output: String::new(),
            },
            privacy: PrivacyConfig {
                sensitive_patterns: vec![
                    ".env".into(),
                    "credentials".into(),
                    ".secret".into(),
                    ".pem".into(),
                    "id_rsa".into(),
                ],
            },
            event_log: EventLogConfig::default(),
            overlays: OverlaysConfig::default(),
        }
    }
}

fn config_path() -> Result<PathBuf, ConfigError> {
    let config_dir = dirs::config_dir().ok_or(ConfigError::NoConfigDir)?;
    Ok(config_dir.join("streams-toolkit").join("config.toml"))
}

/// Persist updated Twitch tokens back to config.toml.
///
/// Reads the existing config, updates only the twitch token fields,
/// and writes it back to avoid clobbering other values.
pub fn save_twitch_tokens(oauth_token: &str, refresh_token: &str) -> Result<(), ConfigError> {
    let path = config_path()?;
    let contents = fs::read_to_string(&path)?;
    let mut doc: toml::Table = contents
        .parse::<toml::Table>()
        .map_err(ConfigError::Parse)?;

    if let Some(twitch) = doc.get_mut("twitch").and_then(|v| v.as_table_mut()) {
        twitch.insert(
            "oauth_token".into(),
            toml::Value::String(oauth_token.to_string()),
        );
        twitch.insert(
            "refresh_token".into(),
            toml::Value::String(refresh_token.to_string()),
        );
    }

    let new_contents = toml::to_string_pretty(&doc)?;
    fs::write(&path, new_contents)?;
    tracing::info!("persisted refreshed tokens to {}", path.display());
    Ok(())
}

#[derive(Debug)]
pub enum ConfigError {
    NoConfigDir,
    Io(io::Error),
    Parse(toml::de::Error),
    Serialize(toml::ser::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoConfigDir => write!(f, "could not determine config directory"),
            Self::Io(e) => write!(f, "config I/O error: {e}"),
            Self::Parse(e) => write!(f, "config parse error: {e}"),
            Self::Serialize(e) => write!(f, "config serialize error: {e}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<io::Error> for ConfigError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(e: toml::de::Error) -> Self {
        Self::Parse(e)
    }
}

impl From<toml::ser::Error> for ConfigError {
    fn from(e: toml::ser::Error) -> Self {
        Self::Serialize(e)
    }
}

/// Load config from `~/.config/streams-toolkit/config.toml`.
/// Creates the default config file if it doesn't exist.
/// Environment variables (loaded via `.env` file) override config values.
pub fn load() -> Result<Config, ConfigError> {
    dotenv::dotenv().ok();

    let path = config_path()?;

    let mut config = if !path.exists() {
        let default = Config::default();
        let contents = toml::to_string_pretty(&default)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, &contents)?;
        tracing::info!("created default config at {}", path.display());
        default
    } else {
        let contents = fs::read_to_string(&path)?;
        let c: Config = toml::from_str(&contents)?;
        tracing::info!("loaded config from {}", path.display());
        c
    };

    config.obs.apply_env_overrides();
    config.twitch.apply_env_overrides();
    config.livepix.apply_env_overrides();
    config.overlays.apply_env_overrides();

    Ok(config)
}
