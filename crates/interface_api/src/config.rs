//! API configuration

use serde::Deserialize;

/// API configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ApiConfig {
    /// Server host
    pub host: String,
    /// Server port
    pub port: u16,
    /// JWT secret for authentication
    pub jwt_secret: String,
    /// JWT expiration in seconds
    pub jwt_expiration_secs: u64,
    /// Database URL
    pub database_url: String,
    /// Log level
    pub log_level: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            jwt_secret: "change-me-in-production".to_string(),
            jwt_expiration_secs: 3600,
            database_url: "postgres://localhost/insurance".to_string(),
            log_level: "info".to_string(),
        }
    }
}

impl ApiConfig {
    /// Loads configuration from environment
    pub fn from_env() -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::Environment::with_prefix("API"))
            .build()?
            .try_deserialize()
    }

    /// Returns the server address
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
