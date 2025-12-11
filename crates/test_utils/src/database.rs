//! Database Test Utilities
//!
//! Provides helpers for database testing including testcontainer management
//! and connection pooling for integration tests.

use once_cell::sync::Lazy;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use testcontainers::{
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage,
};
use tokio::sync::OnceCell;

/// Default PostgreSQL image for testing
const POSTGRES_IMAGE: &str = "postgres";
const POSTGRES_TAG: &str = "16-alpine";
const POSTGRES_USER: &str = "test_user";
const POSTGRES_PASSWORD: &str = "test_password";
const POSTGRES_DB: &str = "insurance_test";

/// Configuration for test database
#[derive(Debug, Clone)]
pub struct TestDatabaseConfig {
    pub user: String,
    pub password: String,
    pub database: String,
    pub host: String,
    pub port: u16,
}

impl Default for TestDatabaseConfig {
    fn default() -> Self {
        Self {
            user: POSTGRES_USER.to_string(),
            password: POSTGRES_PASSWORD.to_string(),
            database: POSTGRES_DB.to_string(),
            host: "localhost".to_string(),
            port: 5432,
        }
    }
}

impl TestDatabaseConfig {
    /// Creates the database connection URL
    pub fn connection_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.user, self.password, self.host, self.port, self.database
        )
    }
}

/// A wrapper around a PostgreSQL test container
pub struct TestDatabase {
    _container: ContainerAsync<GenericImage>,
    pub config: TestDatabaseConfig,
    pub pool: PgPool,
}

impl TestDatabase {
    /// Starts a new PostgreSQL container for testing
    ///
    /// # Returns
    ///
    /// A new TestDatabase instance with an initialized schema
    ///
    /// # Errors
    ///
    /// Returns an error if the container fails to start or schema fails to initialize
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Create and start the container
        let container = GenericImage::new(POSTGRES_IMAGE, POSTGRES_TAG)
            .with_env_var("POSTGRES_USER", POSTGRES_USER)
            .with_env_var("POSTGRES_PASSWORD", POSTGRES_PASSWORD)
            .with_env_var("POSTGRES_DB", POSTGRES_DB)
            .with_exposed_port(5432.tcp())
            .with_wait_for(WaitFor::message_on_stderr("database system is ready to accept connections"))
            .start()
            .await?;

        // Get the mapped port
        let port = container.get_host_port_ipv4(5432).await?;
        let host = container.get_host().await?.to_string();

        let config = TestDatabaseConfig {
            user: POSTGRES_USER.to_string(),
            password: POSTGRES_PASSWORD.to_string(),
            database: POSTGRES_DB.to_string(),
            host,
            port,
        };

        // Create connection pool
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(30))
            .connect(&config.connection_url())
            .await?;

        let test_db = Self {
            _container: container,
            config,
            pool,
        };

        // Initialize schema
        test_db.init_schema().await?;

        Ok(test_db)
    }

    /// Initializes the database schema from the migrations file
    async fn init_schema(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Read and execute the initial schema
        let schema = include_str!("../../../migrations/20240101_000001_initial_schema.sql");
        sqlx::raw_sql(schema).execute(&self.pool).await?;
        Ok(())
    }

    /// Returns a reference to the connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Clears all data from the database while preserving the schema
    ///
    /// Useful for resetting state between tests
    pub async fn clear_data(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let tables = vec![
            "audit_log",
            "unit_transactions",
            "unit_holdings",
            "fund_navs",
            "funds",
            "postings",
            "journal_entries",
            "accounts",
            "claim_payments",
            "claim_reserves",
            "claim_status_history",
            "claims",
            "policy_versions",
            "party_versions",
        ];

        for table in tables {
            sqlx::query(&format!("TRUNCATE TABLE {} CASCADE", table))
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }
}

/// Global test database for shared integration tests
static SHARED_TEST_DB: OnceCell<Arc<TestDatabase>> = OnceCell::const_new();

/// Gets or creates a shared test database instance
///
/// This function provides a singleton test database that can be shared
/// across multiple tests to reduce container startup overhead.
///
/// # Returns
///
/// An Arc to the shared TestDatabase instance
///
/// # Panics
///
/// Panics if the database fails to initialize
pub async fn get_shared_test_database() -> Arc<TestDatabase> {
    SHARED_TEST_DB
        .get_or_init(|| async {
            Arc::new(
                TestDatabase::new()
                    .await
                    .expect("Failed to create shared test database"),
            )
        })
        .await
        .clone()
}

/// Creates an isolated test database for a single test
///
/// Use this when tests need to modify data and isolation is required
pub async fn create_isolated_test_database() -> Result<TestDatabase, Box<dyn std::error::Error + Send + Sync>> {
    TestDatabase::new().await
}

/// Helper macro for running database tests
#[macro_export]
macro_rules! db_test {
    ($name:ident, $body:expr) => {
        #[tokio::test]
        async fn $name() {
            let db = $crate::database::create_isolated_test_database()
                .await
                .expect("Failed to create test database");
            let pool = db.pool();
            $body
        }
    };
}

/// Helper trait for test assertions on database results
pub trait DatabaseTestAssertions {
    /// Asserts that a database operation succeeded
    fn assert_success(&self);

    /// Asserts that a specific number of rows were affected
    fn assert_rows_affected(&self, expected: u64);
}

impl DatabaseTestAssertions for sqlx::postgres::PgQueryResult {
    fn assert_success(&self) {
        // Query completed successfully if we got here
    }

    fn assert_rows_affected(&self, expected: u64) {
        assert_eq!(
            self.rows_affected(),
            expected,
            "Expected {} rows affected, got {}",
            expected,
            self.rows_affected()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_connection_url() {
        let config = TestDatabaseConfig::default();
        let url = config.connection_url();

        assert!(url.starts_with("postgres://"));
        assert!(url.contains(POSTGRES_USER));
        assert!(url.contains(POSTGRES_DB));
    }
}
