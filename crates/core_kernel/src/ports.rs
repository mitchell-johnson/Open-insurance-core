//! Ports and Adapters Infrastructure
//!
//! This module provides the foundational types for implementing the hexagonal
//! architecture (ports and adapters) pattern across all domain modules.
//!
//! # Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     Application Layer                        │
//! │              (Use Cases / Application Services)              │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      Port Traits                             │
//! │     (PartyPort, PolicyPort, ClaimsPort, etc.)               │
//! │   Defined in each domain, depend only on core_kernel         │
//! └─────────────────────────────────────────────────────────────┘
//!                    ▲                         ▲
//!                    │                         │
//!         ┌─────────┴─────────┐     ┌────────┴────────┐
//!         │  Internal Adapter │     │ External Adapter │
//!         │   (PostgreSQL)    │     │  (REST API to    │
//!         │                   │     │  external SOR)   │
//!         └───────────────────┘     └──────────────────┘
//! ```
//!
//! # Usage
//!
//! Each domain defines its own port trait that extends the marker traits here.
//! Adapters implement these traits to provide either internal (database) or
//! external (API) implementations.
//!
//! ```rust,ignore
//! // In domain_party/src/ports.rs
//! #[async_trait]
//! pub trait PartyPort: DomainPort {
//!     async fn get_party(&self, id: PartyId) -> Result<Party, PortError>;
//!     async fn save_party(&self, party: &Party) -> Result<(), PortError>;
//! }
//!
//! // In infra_db - internal adapter
//! impl PartyPort for PostgresPartyAdapter { ... }
//!
//! // In infra_external - external API adapter
//! impl PartyPort for ExternalCrmAdapter { ... }
//! ```

use std::fmt;
use thiserror::Error;
use serde::{Deserialize, Serialize};

/// Error type for port operations
///
/// Provides a unified error type that all port implementations must use,
/// ensuring consistent error handling across internal and external adapters.
#[derive(Debug, Error)]
pub enum PortError {
    /// The requested entity was not found
    #[error("Not found: {entity_type} with id {id}")]
    NotFound {
        entity_type: String,
        id: String,
    },

    /// A validation error occurred
    #[error("Validation error: {message}")]
    Validation {
        message: String,
        field: Option<String>,
    },

    /// The operation conflicts with existing data
    #[error("Conflict: {message}")]
    Conflict {
        message: String,
    },

    /// Connection to the underlying system failed
    #[error("Connection error: {message}")]
    Connection {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// The operation timed out
    #[error("Timeout after {duration_ms}ms: {operation}")]
    Timeout {
        operation: String,
        duration_ms: u64,
    },

    /// Authentication or authorization failed
    #[error("Unauthorized: {message}")]
    Unauthorized {
        message: String,
    },

    /// Rate limit exceeded for external API
    #[error("Rate limited: retry after {retry_after_secs}s")]
    RateLimited {
        retry_after_secs: u64,
    },

    /// The external system is unavailable
    #[error("Service unavailable: {service}")]
    ServiceUnavailable {
        service: String,
    },

    /// A data transformation error occurred
    #[error("Transformation error: {message}")]
    Transformation {
        message: String,
    },

    /// An internal error occurred
    #[error("Internal error: {message}")]
    Internal {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl PortError {
    /// Creates a NotFound error
    pub fn not_found(entity_type: impl Into<String>, id: impl fmt::Display) -> Self {
        PortError::NotFound {
            entity_type: entity_type.into(),
            id: id.to_string(),
        }
    }

    /// Creates a Validation error
    pub fn validation(message: impl Into<String>) -> Self {
        PortError::Validation {
            message: message.into(),
            field: None,
        }
    }

    /// Creates a Validation error with field information
    pub fn validation_field(message: impl Into<String>, field: impl Into<String>) -> Self {
        PortError::Validation {
            message: message.into(),
            field: Some(field.into()),
        }
    }

    /// Creates a Connection error
    pub fn connection(message: impl Into<String>) -> Self {
        PortError::Connection {
            message: message.into(),
            source: None,
        }
    }

    /// Creates an Internal error
    pub fn internal(message: impl Into<String>) -> Self {
        PortError::Internal {
            message: message.into(),
            source: None,
        }
    }

    /// Returns true if this error indicates a transient failure that may succeed on retry
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            PortError::Connection { .. }
                | PortError::Timeout { .. }
                | PortError::RateLimited { .. }
                | PortError::ServiceUnavailable { .. }
        )
    }

    /// Returns true if this error indicates the entity was not found
    pub fn is_not_found(&self) -> bool {
        matches!(self, PortError::NotFound { .. })
    }
}

/// Marker trait for all domain ports
///
/// All port traits should extend this marker to ensure they are
/// thread-safe and can be used in async contexts.
pub trait DomainPort: Send + Sync + 'static {}

/// Configuration for an adapter
///
/// Provides common configuration options for both internal and external adapters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    /// Unique identifier for this adapter instance
    pub adapter_id: String,
    /// Type of adapter (internal, external_api, external_grpc, etc.)
    pub adapter_type: AdapterType,
    /// Whether this adapter is enabled
    pub enabled: bool,
    /// Connection timeout in milliseconds
    pub timeout_ms: u64,
    /// Maximum retry attempts for transient failures
    pub max_retries: u32,
    /// Retry delay in milliseconds (exponential backoff base)
    pub retry_delay_ms: u64,
    /// Circuit breaker configuration
    pub circuit_breaker: Option<CircuitBreakerConfig>,
    /// Custom configuration as JSON
    #[serde(default)]
    pub custom: serde_json::Value,
}

impl Default for AdapterConfig {
    fn default() -> Self {
        Self {
            adapter_id: "default".to_string(),
            adapter_type: AdapterType::Internal,
            enabled: true,
            timeout_ms: 30000, // 30 seconds
            max_retries: 3,
            retry_delay_ms: 1000, // 1 second base
            circuit_breaker: None,
            custom: serde_json::Value::Null,
        }
    }
}

/// Type of adapter implementation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterType {
    /// Internal database adapter (PostgreSQL, etc.)
    Internal,
    /// External REST API adapter
    ExternalRestApi,
    /// External gRPC adapter
    ExternalGrpc,
    /// External message queue adapter (Kafka, RabbitMQ, etc.)
    ExternalMessageQueue,
    /// Mock adapter for testing
    Mock,
}

/// Circuit breaker configuration for external adapters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit
    pub failure_threshold: u32,
    /// Duration in seconds the circuit stays open before half-opening
    pub reset_timeout_secs: u64,
    /// Number of successful calls needed to close the circuit from half-open
    pub success_threshold: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            reset_timeout_secs: 30,
            success_threshold: 3,
        }
    }
}

/// External system configuration for API-based adapters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalSystemConfig {
    /// Base URL for the external system
    pub base_url: String,
    /// API version (e.g., "v1", "2024-01")
    pub api_version: Option<String>,
    /// Authentication configuration
    pub auth: ExternalAuthConfig,
    /// Custom headers to include in requests
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
}

/// Authentication configuration for external systems
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExternalAuthConfig {
    /// No authentication required
    None,
    /// API key authentication
    ApiKey {
        header_name: String,
        #[serde(skip_serializing)]
        key: String,
    },
    /// Bearer token authentication
    BearerToken {
        #[serde(skip_serializing)]
        token: String,
    },
    /// OAuth2 client credentials flow
    OAuth2ClientCredentials {
        token_url: String,
        client_id: String,
        #[serde(skip_serializing)]
        client_secret: String,
        scope: Option<String>,
    },
    /// Basic authentication
    Basic {
        username: String,
        #[serde(skip_serializing)]
        password: String,
    },
}

/// Health status for an adapter
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterHealth {
    /// Adapter is healthy and operational
    Healthy,
    /// Adapter is degraded but operational
    Degraded,
    /// Adapter is unhealthy and not operational
    Unhealthy,
    /// Health status is unknown
    Unknown,
}

/// Health check result for an adapter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Adapter identifier
    pub adapter_id: String,
    /// Current health status
    pub status: AdapterHealth,
    /// Latency of the health check in milliseconds
    pub latency_ms: u64,
    /// Optional message with additional details
    pub message: Option<String>,
    /// Timestamp of the health check
    pub checked_at: chrono::DateTime<chrono::Utc>,
}

/// Trait for adapters that support health checks
#[async_trait::async_trait]
pub trait HealthCheckable: Send + Sync {
    /// Performs a health check on the adapter
    ///
    /// # Returns
    ///
    /// A `HealthCheckResult` indicating the current health status
    async fn health_check(&self) -> HealthCheckResult;
}

/// Metadata about a port operation for auditing and tracing
#[derive(Debug, Clone, Default)]
pub struct OperationMetadata {
    /// Correlation ID for tracing across systems
    pub correlation_id: Option<String>,
    /// User or system that initiated the operation
    pub initiated_by: Option<String>,
    /// Source system identifier
    pub source_system: Option<String>,
    /// Additional context as key-value pairs
    pub context: std::collections::HashMap<String, String>,
}

impl OperationMetadata {
    /// Creates new metadata with a correlation ID
    pub fn with_correlation_id(correlation_id: impl Into<String>) -> Self {
        Self {
            correlation_id: Some(correlation_id.into()),
            ..Default::default()
        }
    }

    /// Adds context to the metadata
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_error_not_found() {
        let error = PortError::not_found("Party", "123");
        assert!(error.is_not_found());
        assert!(!error.is_transient());
        assert!(error.to_string().contains("Party"));
        assert!(error.to_string().contains("123"));
    }

    #[test]
    fn test_port_error_transient() {
        let timeout = PortError::Timeout {
            operation: "get_party".to_string(),
            duration_ms: 5000,
        };
        assert!(timeout.is_transient());

        let rate_limited = PortError::RateLimited {
            retry_after_secs: 60,
        };
        assert!(rate_limited.is_transient());

        let validation = PortError::validation("Invalid email");
        assert!(!validation.is_transient());
    }

    #[test]
    fn test_adapter_config_default() {
        let config = AdapterConfig::default();
        assert_eq!(config.adapter_type, AdapterType::Internal);
        assert!(config.enabled);
        assert_eq!(config.timeout_ms, 30000);
    }

    #[test]
    fn test_operation_metadata() {
        let metadata = OperationMetadata::with_correlation_id("req-123")
            .with_context("user_id", "user-456");

        assert_eq!(metadata.correlation_id, Some("req-123".to_string()));
        assert_eq!(
            metadata.context.get("user_id"),
            Some(&"user-456".to_string())
        );
    }
}
