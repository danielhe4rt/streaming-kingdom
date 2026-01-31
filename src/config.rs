use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{fs, io};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub obs: ObsConfig,
    pub twitch: TwitchConfig,
    pub alerts: AlertsConfig,
    pub waybar: WaybarConfig,
    pub privacy: PrivacyConfig,
    #[serde(default)]
    pub event_log: EventLogConfig,
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
    pub oauth_token: String,
    pub refresh_token: String,
    pub broadcaster_user_id: String,
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
    pub max_events: usize,
}

impl Default for EventLogConfig {
    fn default() -> Self {
        Self {
            show_stream: true,
            show_privacy: true,
            show_system: true,
            show_hyprland: false,
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
                oauth_token: String::new(),
                refresh_token: String::new(),
                broadcaster_user_id: String::new(),
            },
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
        }
    }
}

fn config_path() -> Result<PathBuf, ConfigError> {
    let config_dir = dirs::config_dir().ok_or(ConfigError::NoConfigDir)?;
    Ok(config_dir.join("streams-toolkit").join("config.toml"))
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
pub fn load() -> Result<Config, ConfigError> {
    let path = config_path()?;

    if !path.exists() {
        let default = Config::default();
        let contents = toml::to_string_pretty(&default)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, &contents)?;
        tracing::info!("created default config at {}", path.display());
        return Ok(default);
    }

    let contents = fs::read_to_string(&path)?;
    let config: Config = toml::from_str(&contents)?;
    tracing::info!("loaded config from {}", path.display());
    Ok(config)
}
