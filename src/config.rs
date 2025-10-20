use crate::models::{AuthMethod, ServerConnection};
use crate::themes::ThemeVariant;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Configuration structure for the Ghost SSH Manager
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    /// Application-wide settings
    pub settings: AppSettings,
    /// Server connection definitions
    pub servers: HashMap<String, ServerConfig>,
}

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Current theme variant
    pub theme: ThemeVariant,
    /// Auto-refresh interval in seconds
    pub refresh_interval: u64,
    /// Show only online servers by default
    pub show_only_online: bool,
    /// Animation speed multiplier
    pub animation_speed: f32,
    /// Enable smooth animations
    pub smooth_animations: bool,
    /// Show tooltips and help hints
    pub show_tooltips: bool,
    /// Panel layout (future: different layouts)
    pub panel_layout: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemeVariant::TokyoNightDark,
            refresh_interval: 30,
            show_only_online: false,
            animation_speed: 1.0,
            smooth_animations: true,
            show_tooltips: true,
            panel_layout: "default".to_string(),
        }
    }
}

/// Server configuration that gets serialized to TOML
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: AuthMethodConfig,
    pub description: Option<String>,
    pub tags: Vec<String>,
    /// Custom connection timeout in seconds
    pub timeout: Option<u64>,
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
        let mut connection = ServerConnection::new(
            config.name,
            config.host,
            config.port,
            config.username,
        );
        connection.auth_method = config.auth_method.into();
        connection.description = config.description;
        connection.tags = config.tags;
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
            timeout: None, // Default timeout
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
        let mut config_dir = dirs::config_dir()
            .context("Failed to get config directory")?;
        
        config_dir.push("ghost");
        
        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .context("Failed to create config directory")?;
        }
        
        config_dir.push("config.toml");
        Ok(config_dir)
    }

    /// Load configuration from file
    pub fn load_config(&self) -> Result<Config> {
        if !self.config_path.exists() {
            // Return default config with some sample servers
            let mut config = Config::default();
            self.add_default_servers(&mut config);
            return Ok(config);
        }

        let contents = fs::read_to_string(&self.config_path)
            .context("Failed to read config file")?;
        
        let config: Config = toml::from_str(&contents)
            .context("Failed to parse config file")?;
        
        Ok(config)
    }

    /// Save configuration to file
    pub fn save_config(&self, config: &Config) -> Result<()> {
        let toml_string = toml::to_string_pretty(config)
            .context("Failed to serialize config")?;
        
        fs::write(&self.config_path, toml_string)
            .context("Failed to write config file")?;
        
        Ok(())
    }

    /// Add default sample servers to configuration
    fn add_default_servers(&self, config: &mut Config) {
        let servers = vec![
            ServerConfig {
                name: "Production Server".to_string(),
                host: "prod.example.com".to_string(),
                port: 22,
                username: "admin".to_string(),
                auth_method: AuthMethodConfig::Agent,
                description: Some("Main production server".to_string()),
                tags: vec!["production".to_string(), "web".to_string()],
                timeout: Some(10),
            },
            ServerConfig {
                name: "Development Box".to_string(),
                host: "dev.local".to_string(),
                port: 22,
                username: "developer".to_string(),
                auth_method: AuthMethodConfig::PublicKey {
                    key_path: "~/.ssh/id_rsa".to_string(),
                },
                description: Some("Development environment".to_string()),
                tags: vec!["development".to_string(), "local".to_string()],
                timeout: Some(5),
            },
            ServerConfig {
                name: "Database Server".to_string(),
                host: "db.example.com".to_string(),
                port: 22,
                username: "dbadmin".to_string(),
                auth_method: AuthMethodConfig::Agent,
                description: Some("Database server cluster".to_string()),
                tags: vec!["database".to_string(), "production".to_string()],
                timeout: Some(15),
            },
        ];

        for (i, server) in servers.into_iter().enumerate() {
            config.servers.insert(format!("server_{}", i + 1), server);
        }
    }


    /// Convert config to server connections map
    pub fn config_to_connections(&self, config: &Config) -> HashMap<String, ServerConnection> {
        config.servers.iter()
            .map(|(id, server_config)| {
                let mut connection = ServerConnection::from(server_config.clone());
                connection.id = id.clone();
                (id.clone(), connection)
            })
            .collect()
    }

    /// Convert server connections map to config
    pub fn connections_to_config(&self, connections: &HashMap<String, ServerConnection>, settings: AppSettings) -> Config {
        let servers = connections.iter()
            .map(|(id, connection)| {
                (id.clone(), ServerConfig::from(connection.clone()))
            })
            .collect();

        Config {
            settings,
            servers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::env;

    #[test]
    fn test_config_serialization() {
        let mut config = Config::default();
        config.servers.insert("test".to_string(), ServerConfig {
            name: "Test Server".to_string(),
            host: "test.com".to_string(),
            port: 22,
            username: "user".to_string(),
            auth_method: AuthMethodConfig::Agent,
            description: None,
            tags: vec![],
            timeout: None,
        });

        let toml_str = toml::to_string(&config).unwrap();
        let parsed_config: Config = toml::from_str(&toml_str).unwrap();
        
        assert_eq!(config.servers.len(), parsed_config.servers.len());
        assert_eq!(config.settings.theme, parsed_config.settings.theme);
    }

    #[test]
    fn test_server_conversion() {
        let server_config = ServerConfig {
            name: "Test".to_string(),
            host: "test.com".to_string(),
            port: 22,
            username: "user".to_string(),
            auth_method: AuthMethodConfig::Agent,
            description: Some("test".to_string()),
            tags: vec!["test".to_string()],
            timeout: None,
        };

        let connection = ServerConnection::from(server_config.clone());
        let back_to_config = ServerConfig::from(connection);
        
        assert_eq!(server_config.name, back_to_config.name);
        assert_eq!(server_config.host, back_to_config.host);
        assert_eq!(server_config.port, back_to_config.port);
    }
}