//! Configuration management module
//!
//! Handles loading configuration from environment variables and config files,
//! with environment variables taking priority over config file values.
//!
//! # Configuration Priority
//!
//! 1. Environment variables (highest priority)
//! 2. Configuration file (config.toml)
//! 3. Default values (lowest priority)
//!
//! # Note
//!
//! The following configurations are managed via Web UI and stored in database:
//! - DNS listeners (ports, bind addresses, TLS certificates)
//! - Upstream DNS servers
//! - Cache settings
//! - Query strategy

use std::path::{Path, PathBuf};
use std::sync::RwLock;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    // Web service port
    pub web_port: u16,

    // Database configuration
    pub database_url: String,

    // Authentication configuration
    pub admin_username: String,
    pub admin_password: String,

    // Log configuration
    pub log_path: PathBuf,
    pub log_level: String,
    pub log_max_size: u64,
    pub log_retention_days: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            web_port: 8080,
            database_url: "sqlite:fluxdns.db?mode=rwc".to_string(),
            admin_username: "admin".to_string(),
            admin_password: "admin".to_string(),
            log_path: PathBuf::from("logs"),
            log_level: "info".to_string(),
            log_max_size: 10 * 1024 * 1024, // 10MB
            log_retention_days: 30,
        }
    }
}

/// Partial configuration for merging from different sources
#[derive(Debug, Default, Clone, Deserialize)]
pub struct PartialConfig {
    pub web_port: Option<u16>,
    pub database_url: Option<String>,
    pub admin_username: Option<String>,
    pub admin_password: Option<String>,
    pub log_path: Option<PathBuf>,
    pub log_level: Option<String>,
    pub log_max_size: Option<u64>,
    pub log_retention_days: Option<u32>,
}

/// Configuration manager responsible for loading and providing access to configuration
pub struct ConfigManager {
    config: RwLock<AppConfig>,
}

impl ConfigManager {
    /// Load configuration from environment variables and config file
    pub fn load() -> Result<Self> {
        Self::load_with_path("config.toml")
    }

    /// Load configuration with a custom config file path
    pub fn load_with_path<P: AsRef<Path>>(config_path: P) -> Result<Self> {
        // Load .env file if present
        let _ = dotenvy::dotenv();

        // Start with defaults
        let mut config = AppConfig::default();

        // Load from config file if exists (lower priority)
        if let Ok(file_config) = Self::load_from_file(config_path.as_ref()) {
            Self::merge_config(&mut config, file_config);
        }

        // Load from environment variables (higher priority)
        let env_config = Self::load_from_env();
        Self::merge_config(&mut config, env_config);

        Ok(Self {
            config: RwLock::new(config),
        })
    }

    /// Create ConfigManager from explicit configs for testing
    pub fn from_configs(
        file_config: Option<PartialConfig>,
        env_config: Option<PartialConfig>,
    ) -> Self {
        let mut config = AppConfig::default();

        if let Some(fc) = file_config {
            Self::merge_config(&mut config, fc);
        }

        if let Some(ec) = env_config {
            Self::merge_config(&mut config, ec);
        }

        Self {
            config: RwLock::new(config),
        }
    }

    /// Get current configuration
    pub fn get(&self) -> AppConfig {
        self.config.read().unwrap().clone()
    }

    /// Load configuration from environment variables
    pub fn load_from_env() -> PartialConfig {
        PartialConfig {
            web_port: std::env::var("WEB_PORT")
                .ok()
                .and_then(|v| v.parse().ok()),
            database_url: std::env::var("DATABASE_URL").ok(),
            admin_username: std::env::var("ADMIN_USERNAME").ok(),
            admin_password: std::env::var("ADMIN_PASSWORD").ok(),
            log_path: std::env::var("LOG_PATH").ok().map(PathBuf::from),
            log_level: std::env::var("LOG_LEVEL").ok(),
            log_max_size: std::env::var("LOG_MAX_SIZE")
                .ok()
                .and_then(|v| v.parse().ok()),
            log_retention_days: std::env::var("LOG_RETENTION_DAYS")
                .ok()
                .and_then(|v| v.parse().ok()),
        }
    }

    /// Load configuration from TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<PartialConfig> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;
        let config: PartialConfig =
            toml::from_str(&content).with_context(|| "Failed to parse config file as TOML")?;
        Ok(config)
    }

    /// Merge partial config into full config
    pub fn merge_config(config: &mut AppConfig, partial: PartialConfig) {
        if let Some(v) = partial.web_port {
            config.web_port = v;
        }
        if let Some(v) = partial.database_url {
            config.database_url = v;
        }
        if let Some(v) = partial.admin_username {
            config.admin_username = v;
        }
        if let Some(v) = partial.admin_password {
            config.admin_password = v;
        }
        if let Some(v) = partial.log_path {
            config.log_path = v;
        }
        if let Some(v) = partial.log_level {
            config.log_level = v;
        }
        if let Some(v) = partial.log_max_size {
            config.log_max_size = v;
        }
        if let Some(v) = partial.log_retention_days {
            config.log_retention_days = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.web_port, 8080);
        assert_eq!(config.database_url, "sqlite:fluxdns.db?mode=rwc");
        assert_eq!(config.admin_username, "admin");
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_load_from_toml_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
web_port = 9090
database_url = "sqlite:test.db"
admin_username = "testuser"
log_level = "debug"
"#
        )
        .unwrap();

        let config = ConfigManager::load_from_file(file.path()).unwrap();
        assert_eq!(config.web_port, Some(9090));
        assert_eq!(config.database_url, Some("sqlite:test.db".to_string()));
        assert_eq!(config.admin_username, Some("testuser".to_string()));
        assert_eq!(config.log_level, Some("debug".to_string()));
    }

    #[test]
    fn test_merge_config() {
        let mut config = AppConfig::default();
        let partial = PartialConfig {
            web_port: Some(9090),
            database_url: Some("sqlite:merged.db".to_string()),
            ..Default::default()
        };

        ConfigManager::merge_config(&mut config, partial);

        assert_eq!(config.web_port, 9090);
        assert_eq!(config.database_url, "sqlite:merged.db");
        assert_eq!(config.log_level, "info"); // unchanged
    }

    #[test]
    fn test_env_priority_over_file() {
        let file_config = PartialConfig {
            web_port: Some(9090),
            database_url: Some("sqlite:file.db".to_string()),
            ..Default::default()
        };

        let env_config = PartialConfig {
            web_port: Some(8888),
            ..Default::default()
        };

        let manager = ConfigManager::from_configs(Some(file_config), Some(env_config));
        let config = manager.get();

        assert_eq!(config.web_port, 8888);
        assert_eq!(config.database_url, "sqlite:file.db");
    }

    #[test]
    fn test_missing_config_file_uses_defaults() {
        let manager = ConfigManager::load_with_path("nonexistent_config.toml").unwrap();
        let config = manager.get();

        assert_eq!(config.web_port, 8080);
        assert_eq!(config.database_url, "sqlite:fluxdns.db?mode=rwc");
    }
}
