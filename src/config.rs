use crate::models::{AuthMethod, ServerConnection};
use crate::themes::ThemeVariant;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Configuration structure for the Ghost SSH Manager
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    /// Application-wide settings
    pub settings: AppSettings,
    /// Server connection definitions
    pub servers: HashMap<String, ServerConfig>,
}

/// Application settings.
///
/// Every field defaults, so a hand-edited config that sets only the keys the
/// user cares about still loads. Without this, omitting a single key made the
/// whole config unparseable and Ghost refused to start.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    /// Current theme variant
    pub theme: ThemeVariant,
    /// Auto-refresh interval in seconds
    pub refresh_interval: u64,
    /// Show only online servers by default
    pub show_only_online: bool,
    /// Show tooltips and help hints
    pub show_tooltips: bool,
    /// Panel layout: "single", "two", or "three".
    pub panel_layout: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemeVariant::TokyoNightDark,
            refresh_interval: 30,
            show_only_online: false,
            show_tooltips: true,
            panel_layout: "three".to_string(),
        }
    }
}

/// Server configuration that gets serialized to TOML.
///
/// `#[serde(default)]` on the optional fields keeps configs written by older
/// versions of Ghost loadable.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: AuthMethodConfig,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Custom ssh ConnectTimeout in seconds.
    #[serde(default)]
    pub timeout: Option<u64>,
    /// When this entry was first created. Persisted so the details panel shows
    /// a real age instead of resetting to "today" on every launch.
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_modified: Option<DateTime<Utc>>,
}

/// Authentication method configuration for TOML serialization
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthMethodConfig {
    Password,
    PublicKey { key_path: String },
    Agent,
    Interactive,
}

impl From<AuthMethodConfig> for AuthMethod {
    fn from(config: AuthMethodConfig) -> Self {
        match config {
            AuthMethodConfig::Password => AuthMethod::Password,
            AuthMethodConfig::PublicKey { key_path } => AuthMethod::PublicKey { key_path },
            AuthMethodConfig::Agent => AuthMethod::Agent,
            AuthMethodConfig::Interactive => AuthMethod::Interactive,
        }
    }
}

impl From<AuthMethod> for AuthMethodConfig {
    fn from(auth: AuthMethod) -> Self {
        match auth {
            AuthMethod::Password => AuthMethodConfig::Password,
            AuthMethod::PublicKey { key_path } => AuthMethodConfig::PublicKey { key_path },
            AuthMethod::Agent => AuthMethodConfig::Agent,
            AuthMethod::Interactive => AuthMethodConfig::Interactive,
        }
    }
}

impl From<ServerConfig> for ServerConnection {
    fn from(config: ServerConfig) -> Self {
        let mut connection =
            ServerConnection::new(config.name, config.host, config.port, config.username);
        connection.auth_method = config.auth_method.into();
        connection.description = config.description;
        connection.tags = config.tags;
        connection.timeout = config.timeout;
        if let Some(created) = config.created_at {
            connection.created_at = created;
        }
        if let Some(modified) = config.last_modified {
            connection.last_modified = modified;
        }
        connection
    }
}

impl From<ServerConnection> for ServerConfig {
    fn from(conn: ServerConnection) -> Self {
        Self {
            name: conn.name,
            host: conn.host,
            port: conn.port,
            username: conn.username,
            auth_method: conn.auth_method.into(),
            description: conn.description,
            tags: conn.tags,
            // Previously hardcoded to None, which wiped any user-set timeout on
            // the next save.
            timeout: conn.timeout,
            created_at: Some(conn.created_at),
            last_modified: Some(conn.last_modified),
        }
    }
}

/// Configuration manager for Ghost SSH Manager
pub struct ConfigManager {
    config_path: PathBuf,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub fn new() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        Ok(Self { config_path })
    }

    /// Get the configuration file path
    fn get_config_path() -> Result<PathBuf> {
        let mut config_dir = dirs::config_dir().context("Failed to get config directory")?;

        config_dir.push("ghost");

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).context("Failed to create config directory")?;
        }

        config_dir.push("config.toml");
        Ok(config_dir)
    }

    /// Load configuration from file.
    ///
    /// A missing config yields an empty one. Earlier versions seeded three
    /// fictional servers (prod.example.com and friends) which were then health
    /// checked, DNS-resolved, and written permanently into the user's config on
    /// first save. The UI shows a first-run hint instead.
    pub fn load_config(&self) -> Result<Config> {
        if !self.config_path.exists() {
            return Ok(Config::default());
        }

        let contents =
            fs::read_to_string(&self.config_path).context("Failed to read config file")?;

        let config: Config = toml::from_str(&contents).with_context(|| {
            format!(
                "Failed to parse config file at {}",
                self.config_path.display()
            )
        })?;

        Ok(config)
    }

    /// Where the config lives, for display in the UI.
    pub fn config_path(&self) -> &std::path::Path {
        &self.config_path
    }

    /// Save configuration to file.
    ///
    /// Written atomically: serialize to a temp file in the same directory,
    /// restrict its permissions, then rename it over the real config. A crash or
    /// full disk mid-write can no longer truncate/corrupt the existing server
    /// list, and the config is not left world-readable.
    pub fn save_config(&self, config: &Config) -> Result<()> {
        let toml_string = toml::to_string_pretty(config).context("Failed to serialize config")?;

        // Temp file in the same directory (so the rename stays on one filesystem
        // and is therefore atomic). The PID suffix avoids collisions between
        // concurrent writers.
        let tmp_path = self
            .config_path
            .with_extension(format!("tmp.{}", std::process::id()));

        fs::write(&tmp_path, &toml_string).context("Failed to write temporary config file")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Err(e) = fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600)) {
                let _ = fs::remove_file(&tmp_path); // best-effort cleanup
                return Err(e).context("Failed to set config file permissions");
            }
        }

        fs::rename(&tmp_path, &self.config_path).context("Failed to replace config file")?;

        Ok(())
    }

    /// Convert config to server connections map
    pub fn config_to_connections(&self, config: &Config) -> HashMap<String, ServerConnection> {
        config
            .servers
            .iter()
            .map(|(id, server_config)| {
                let mut connection = ServerConnection::from(server_config.clone());
                connection.id = id.clone();
                (id.clone(), connection)
            })
            .collect()
    }

    /// Convert server connections map to config
    pub fn connections_to_config(
        &self,
        connections: &HashMap<String, ServerConnection>,
        settings: AppSettings,
    ) -> Config {
        let servers = connections
            .iter()
            .map(|(id, connection)| (id.clone(), ServerConfig::from(connection.clone())))
            .collect();

        Config { settings, servers }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ServerConfig {
        ServerConfig {
            name: "Test Server".to_string(),
            host: "test.example.com".to_string(),
            port: 2222,
            username: "user".to_string(),
            auth_method: AuthMethodConfig::Agent,
            description: Some("a server".to_string()),
            tags: vec!["prod".to_string()],
            timeout: Some(15),
            created_at: Some(Utc::now()),
            last_modified: Some(Utc::now()),
        }
    }

    #[test]
    fn test_config_serialization() {
        let mut config = Config::default();
        config.servers.insert("test".to_string(), sample());

        let toml_str = toml::to_string(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(config.servers.len(), parsed.servers.len());
        assert_eq!(config.settings.theme, parsed.settings.theme);
    }

    #[test]
    fn round_trip_preserves_every_field() {
        // Regression: `ServerConfig::from(ServerConnection)` hardcoded
        // `timeout: None` and dropped `created_at`, so a user's configured
        // timeout was wiped and the creation date reset on every save.
        let original = sample();
        let created = original.created_at.unwrap();

        let connection = ServerConnection::from(original.clone());
        assert_eq!(connection.timeout, Some(15));
        assert_eq!(connection.created_at, created);

        let back = ServerConfig::from(connection);
        assert_eq!(back.timeout, Some(15));
        assert_eq!(back.created_at, Some(created));
        assert_eq!(back.name, original.name);
        assert_eq!(back.host, original.host);
        assert_eq!(back.port, original.port);
        assert_eq!(back.tags, original.tags);
        assert_eq!(back.description, original.description);
    }

    #[test]
    fn configs_from_older_versions_still_load() {
        // Fields added later must be optional or upgrades break every config.
        let legacy = r#"
[settings]
theme = "TokyoNightDark"
refresh_interval = 30
show_only_online = false
animation_speed = 1.0
smooth_animations = true
show_tooltips = true
panel_layout = "default"
# ^ animation_speed / smooth_animations were removed; unknown keys must be
#   ignored rather than failing the load.

[servers.old]
name = "Legacy"
host = "legacy.example.com"
port = 22
username = "root"

[servers.old.auth_method]
type = "agent"
"#;
        let config: Config = toml::from_str(legacy).unwrap();
        let server = &config.servers["old"];
        assert_eq!(server.name, "Legacy");
        assert_eq!(server.timeout, None);
        assert!(server.created_at.is_none());
        assert!(server.tags.is_empty());
    }

    #[test]
    fn a_partial_settings_block_still_loads() {
        // Hand-editing a config is normal; omitting a key must not brick it.
        let minimal = r#"
[settings]
theme = "DraculaDark"

[servers.a]
name = "A"
host = "a.example.com"
port = 22
username = "me"

[servers.a.auth_method]
type = "agent"
"#;
        let config: Config = toml::from_str(minimal).unwrap();
        assert_eq!(config.settings.theme, ThemeVariant::DraculaDark);
        // Unspecified keys fall back to their defaults.
        assert_eq!(config.settings.refresh_interval, 30);
        assert!(config.settings.show_tooltips);
    }

    #[test]
    fn a_config_with_no_settings_block_loads() {
        let servers_only = r#"
[servers.a]
name = "A"
host = "a.example.com"
port = 22
username = "me"

[servers.a.auth_method]
type = "agent"
"#;
        let config: Config = toml::from_str(servers_only).unwrap();
        assert_eq!(config.servers.len(), 1);
        assert_eq!(config.settings.theme, ThemeVariant::default());
    }

    #[test]
    fn the_shipped_example_config_parses() {
        // The example is documentation people copy from; if it doesn't parse,
        // it teaches the wrong syntax.
        let example = include_str!("../example-config.toml");
        let config: Config = toml::from_str(example).expect("example-config.toml must parse");
        assert!(!config.servers.is_empty());
    }

    #[test]
    fn public_key_auth_survives_a_round_trip() {
        let mut cfg = sample();
        cfg.auth_method = AuthMethodConfig::PublicKey {
            key_path: "~/.ssh/id_ed25519".to_string(),
        };
        let connection = ServerConnection::from(cfg);
        let back = ServerConfig::from(connection);
        match back.auth_method {
            AuthMethodConfig::PublicKey { key_path } => {
                assert_eq!(key_path, "~/.ssh/id_ed25519")
            }
            other => panic!("expected PublicKey, got {:?}", other),
        }
    }

    #[test]
    fn a_fresh_config_has_no_servers() {
        // Ghost used to invent three fictional example servers on first run and
        // then persist them into the user's config.
        let config = Config::default();
        assert!(config.servers.is_empty());
    }

    #[test]
    fn connection_ids_come_from_the_toml_keys() {
        let manager = ConfigManager {
            config_path: PathBuf::from("/nonexistent/config.toml"),
        };
        let mut config = Config::default();
        config.servers.insert("my-key".to_string(), sample());

        let connections = manager.config_to_connections(&config);
        assert!(connections.contains_key("my-key"));
        assert_eq!(connections["my-key"].id, "my-key");
    }
}
