//! Database connection pool management
//!
//! This module provides connection pool configuration and creation for PostgreSQL
//! using SQLx with support for compile-time query verification.

use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use tracing::info;

use crate::error::DatabaseError;

/// Type alias for the PostgreSQL connection pool
pub type DatabasePool = PgPool;

/// Configuration options for the database connection pool
///
/// # Example
///
/// ```rust
/// use infra_db::DatabaseConfig;
///
/// let config = DatabaseConfig::new("postgres://localhost/insurance")
///     .max_connections(20)
///     .min_connections(5)
///     .connect_timeout(Duration::from_secs(10));
/// ```
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// PostgreSQL connection string
    pub url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Minimum number of connections to maintain
    pub min_connections: u32,
    /// Connection timeout duration
    pub connect_timeout: Duration,
    /// Maximum lifetime of a connection
    pub max_lifetime: Duration,
    /// Idle timeout before closing a connection
    pub idle_timeout: Duration,
}

impl DatabaseConfig {
    /// Creates a new database configuration with the given connection URL
    ///
    /// # Arguments
    ///
    /// * `url` - PostgreSQL connection string (e.g., "postgres://user:pass@host/db")
    ///
    /// # Returns
    ///
    /// A new `DatabaseConfig` with sensible defaults
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            max_connections: 10,
            min_connections: 2,
            connect_timeout: Duration::from_secs(30),
            max_lifetime: Duration::from_secs(30 * 60), // 30 minutes
            idle_timeout: Duration::from_secs(10 * 60), // 10 minutes
        }
    }

    /// Sets the maximum number of connections in the pool
    ///
    /// # Arguments
    ///
    /// * `max` - Maximum connection count (default: 10)
    pub fn max_connections(mut self, max: u32) -> Self {
        self.max_connections = max;
        self
    }

    /// Sets the minimum number of connections to maintain
    ///
    /// # Arguments
    ///
    /// * `min` - Minimum connection count (default: 2)
    pub fn min_connections(mut self, min: u32) -> Self {
        self.min_connections = min;
        self
    }

    /// Sets the connection timeout duration
    ///
    /// # Arguments
    ///
    /// * `timeout` - Duration to wait for a connection (default: 30s)
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Sets the maximum lifetime of a connection
    ///
    /// # Arguments
    ///
    /// * `lifetime` - Maximum duration a connection can live (default: 30 min)
    pub fn max_lifetime(mut self, lifetime: Duration) -> Self {
        self.max_lifetime = lifetime;
        self
    }

    /// Sets the idle timeout before closing a connection
    ///
    /// # Arguments
    ///
    /// * `timeout` - Duration of inactivity before closing (default: 10 min)
    pub fn idle_timeout(mut self, timeout: Duration) -> Self {
        self.idle_timeout = timeout;
        self
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self::new("postgres://localhost/insurance")
    }
}

/// Creates a database connection pool with the given configuration
///
/// This function establishes a connection pool to PostgreSQL with the specified
/// settings. The pool manages connection lifecycle and provides efficient
/// connection reuse.
///
/// # Arguments
///
/// * `config` - Database configuration options
///
/// # Returns
///
/// A `Result` containing the connection pool or a database error
///
/// # Errors
///
/// Returns `DatabaseError::ConnectionFailed` if the pool cannot be created
///
/// # Example
///
/// ```rust,ignore
/// use infra_db::{DatabaseConfig, create_pool};
///
/// let config = DatabaseConfig::new("postgres://localhost/insurance");
/// let pool = create_pool(config).await?;
/// ```
pub async fn create_pool(config: DatabaseConfig) -> Result<DatabasePool, DatabaseError> {
    info!(
        "Creating database pool with max_connections={}, min_connections={}",
        config.max_connections, config.min_connections
    );

    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(config.connect_timeout)
        .max_lifetime(config.max_lifetime)
        .idle_timeout(config.idle_timeout)
        .connect(&config.url)
        .await
        .map_err(|e| DatabaseError::ConnectionFailed(e.to_string()))?;

    info!("Database pool created successfully");
    Ok(pool)
}

/// Creates a connection pool from a URL string with default settings
///
/// This is a convenience function for simple use cases where default
/// pool settings are acceptable.
///
/// # Arguments
///
/// * `url` - PostgreSQL connection string
///
/// # Returns
///
/// A `Result` containing the connection pool or a database error
pub async fn create_pool_from_url(url: &str) -> Result<DatabasePool, DatabaseError> {
    create_pool(DatabaseConfig::new(url)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = DatabaseConfig::new("postgres://test")
            .max_connections(50)
            .min_connections(10)
            .connect_timeout(Duration::from_secs(60));

        assert_eq!(config.max_connections, 50);
        assert_eq!(config.min_connections, 10);
        assert_eq!(config.connect_timeout, Duration::from_secs(60));
    }
}
