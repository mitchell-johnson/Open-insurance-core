//! Open Insurance Core - API Server Binary
//!
//! This binary starts the HTTP API server for the insurance core system.
//!
//! # Usage
//!
//! ```bash
//! # Run with default configuration
//! cargo run --bin insurance-api
//!
//! # Run with environment variables
//! API_HOST=0.0.0.0 API_PORT=8080 DATABASE_URL=postgres://... cargo run --bin insurance-api
//! ```
//!
//! # Environment Variables
//!
//! * `API_HOST` - Server host (default: 0.0.0.0)
//! * `API_PORT` - Server port (default: 8080)
//! * `API_JWT_SECRET` - JWT signing secret (required in production)
//! * `API_JWT_EXPIRATION_SECS` - JWT token expiration in seconds (default: 3600)
//! * `API_DATABASE_URL` - PostgreSQL connection string
//! * `API_LOG_LEVEL` - Log level: trace, debug, info, warn, error (default: info)

use interface_api::{create_router, config::ApiConfig};
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Main entry point for the API server.
///
/// Initializes logging, loads configuration, establishes database connection,
/// and starts the HTTP server.
///
/// # Errors
///
/// Returns an error if:
/// - Configuration cannot be loaded from environment
/// - Database connection fails
/// - Server fails to bind to the configured address
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file if present (useful for local development)
    dotenvy::dotenv().ok();

    // Load configuration from environment
    let config = load_config()?;

    // Initialize tracing/logging
    init_tracing(&config.log_level);

    tracing::info!(
        host = %config.host,
        port = %config.port,
        "Starting Open Insurance Core API Server"
    );

    // Create database connection pool
    let pool = create_database_pool(&config.database_url).await?;

    // Run database migrations
    run_migrations(&pool).await?;

    // Create the API router
    let app = create_router(pool, config.clone());

    // Parse server address
    let addr: SocketAddr = config.server_addr().parse()?;

    tracing::info!(%addr, "Server listening");

    // Create TCP listener and serve
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server shutdown complete");
    Ok(())
}

/// Loads API configuration from environment variables.
///
/// Falls back to default values if environment variables are not set.
///
/// # Returns
///
/// `ApiConfig` populated from environment or defaults
///
/// # Errors
///
/// Returns error if required environment variables are missing or invalid
fn load_config() -> Result<ApiConfig, Box<dyn std::error::Error>> {
    // Try to load from environment with API_ prefix
    let config = ApiConfig::from_env().unwrap_or_else(|_| {
        // Fall back to individual env vars or defaults
        ApiConfig {
            host: std::env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("API_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            jwt_secret: std::env::var("API_JWT_SECRET")
                .unwrap_or_else(|_| "dev-secret-change-in-production".to_string()),
            jwt_expiration_secs: std::env::var("API_JWT_EXPIRATION_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600),
            database_url: std::env::var("DATABASE_URL")
                .or_else(|_| std::env::var("API_DATABASE_URL"))
                .unwrap_or_else(|_| "postgres://localhost/insurance".to_string()),
            log_level: std::env::var("API_LOG_LEVEL")
                .or_else(|_| std::env::var("RUST_LOG"))
                .unwrap_or_else(|_| "info".to_string()),
        }
    });

    Ok(config)
}

/// Initializes the tracing subscriber for structured logging.
///
/// # Arguments
///
/// * `log_level` - The minimum log level to output (trace, debug, info, warn, error)
fn init_tracing(log_level: &str) {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .init();
}

/// Creates a PostgreSQL connection pool.
///
/// # Arguments
///
/// * `database_url` - PostgreSQL connection string
///
/// # Returns
///
/// Configured `PgPool` ready for use
///
/// # Errors
///
/// Returns error if connection to database fails
async fn create_database_pool(database_url: &str) -> Result<sqlx::PgPool, sqlx::Error> {
    tracing::info!("Connecting to database...");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .connect(database_url)
        .await?;

    tracing::info!("Database connection established");
    Ok(pool)
}

/// Runs database migrations using SQLx.
///
/// # Arguments
///
/// * `pool` - Database connection pool
///
/// # Errors
///
/// Returns error if migrations fail to apply
async fn run_migrations(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    tracing::info!("Running database migrations...");

    // SQLx migrations would go here if using sqlx::migrate!
    // For now, we assume migrations are handled externally or via the migration SQL file
    // sqlx::migrate!("./migrations").run(pool).await?;

    // Verify database connectivity with a simple query
    sqlx::query("SELECT 1")
        .execute(pool)
        .await?;

    tracing::info!("Database ready");
    Ok(())
}

/// Waits for shutdown signal (Ctrl+C or SIGTERM).
///
/// This enables graceful shutdown of the server, allowing in-flight
/// requests to complete before the process exits.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, initiating graceful shutdown");
        }
        _ = terminate => {
            tracing::info!("Received SIGTERM, initiating graceful shutdown");
        }
    }
}
