//! Configuration module for E3DC-MQTT bridge
//!
//! Loads configuration from TOML file with structure matching the Python version:
//! - [default] - General settings (log_level)
//! - [e3dc] - E3DC connection settings
//! - [mqtt] - MQTT broker settings

use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::time::Duration;

/// Log level for the application
#[derive(Debug, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// Convert to tracing LevelFilter string
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LogLevel::Trace => "TRACE",
                LogLevel::Debug => "DEBUG",
                LogLevel::Info => "INFO",
                LogLevel::Warn => "WARN",
                LogLevel::Error => "ERROR",
            }
        )
    }
}

/// Main configuration structure
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub default: DefaultConfig,
    pub e3dc: E3dcConfig,
    pub mqtt: MqttConfig,
}

/// General application settings
#[derive(Debug, Deserialize, Clone, Default)]
pub struct DefaultConfig {
    /// Log level: TRACE, DEBUG, INFO, WARN, ERROR
    #[serde(default)]
    pub log_level: LogLevel,
}

/// E3DC connection configuration
#[derive(Deserialize, Clone)]
pub struct E3dcConfig {
    /// E3DC hostname or IP address (required)
    pub host: String,

    /// E3DC portal username (required, usually email)
    pub username: String,

    /// E3DC portal password (required)
    pub password: String,

    /// RSCP key from E3DC settings (required)
    pub key: String,

    /// Status update interval (e.g., "5s", "10s")
    #[serde(default = "default_interval", with = "humantime_serde")]
    pub interval: Duration,

    /// Statistics update interval (e.g., "5m", "300s")
    #[serde(default = "default_statistic_interval", with = "humantime_serde")]
    pub statistic_update_interval: Duration,
}

fn default_interval() -> Duration {
    Duration::from_secs(5)
}

fn default_statistic_interval() -> Duration {
    Duration::from_secs(300)
}

impl std::fmt::Debug for E3dcConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("E3dcConfig")
            .field("host", &self.host)
            .field("username", &self.username)
            .field("password", &"***REDACTED***")
            .field("key", &"***REDACTED***")
            .field("interval", &self.interval)
            .field("statistic_update_interval", &self.statistic_update_interval)
            .finish()
    }
}

/// MQTT broker configuration
#[derive(Deserialize, Clone)]
pub struct MqttConfig {
    /// MQTT root topic (e.g., "e3dc")
    #[serde(default = "default_mqtt_root")]
    pub root: String,

    /// MQTT broker hostname
    pub host: String,

    /// MQTT broker port (default 1883)
    #[serde(default = "default_mqtt_port")]
    pub port: u16,

    /// MQTT username (required)
    pub username: String,

    /// MQTT password (required)
    pub password: String,
}

fn default_mqtt_root() -> String {
    "e3dc".to_string()
}

fn default_mqtt_port() -> u16 {
    1883
}

impl std::fmt::Debug for MqttConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("MqttConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username)
            .field("password", &"***REDACTED***")
            .field("root", &self.root)
            .finish()
    }
}

impl Config {
    /// Load configuration from TOML file
    ///
    /// # Arguments
    /// * `path` - Path to the config.toml file
    ///
    /// # Errors
    /// Returns error if file cannot be read or parsed
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(ConfigError::FileNotFound(
                path.to_string_lossy().to_string(),
            ));
        }

        let contents =
            fs::read_to_string(path).map_err(|e| ConfigError::ReadError(e.to_string()))?;

        let config: Config =
            toml::from_str(&contents).map_err(|e| ConfigError::ParseError(e.to_string()))?;

        config.validate()?;

        Ok(config)
    }

    /// Validate configuration logic (semantic validation beyond type checks)
    fn validate(&self) -> Result<(), ConfigError> {
        // Duration is always positive by type, no need to validate intervals

        // Validate MQTT host is not empty
        if self.mqtt.host.is_empty() {
            return Err(ConfigError::ValidationError(
                "mqtt.host must not be empty".to_string(),
            ));
        }

        Ok(())
    }
}

/// Configuration loading errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Configuration file not found: {0}")]
    FileNotFound(String),

    #[error("Failed to read configuration file: {0}")]
    ReadError(String),

    #[error("Failed to parse configuration: {0}")]
    ParseError(String),

    #[error("Configuration validation failed: {0}")]
    ValidationError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let default = DefaultConfig::default();
        assert_eq!(default.log_level, LogLevel::Info);
    }

    #[test]
    fn test_log_level_parsing() {
        // Test that log levels are parsed correctly from TOML
        let toml_str = r#"
            [default]
            log_level = "DEBUG"

            [e3dc]
            host = "test"
            username = "test"
            password = "test"
            key = "test"

            [mqtt]
            host = "test"
            username = "test"
            password = "test"
        "#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.default.log_level, LogLevel::Debug);
    }
}
