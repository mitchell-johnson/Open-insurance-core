//! Domain Registry
//!
//! This module provides a central registry for configuring and retrieving
//! domain port implementations. It enables swapping between internal (database)
//! and external (API) adapters at runtime based on configuration.
//!
//! # Architecture
//!
//! The registry follows the Service Locator pattern, providing a single point
//! of access for all domain ports. This allows:
//!
//! - Configuration-driven adapter selection
//! - Runtime swapping of implementations
//! - Health checking across all adapters
//! - Centralized lifecycle management
//!
//! # Usage
//!
//! ```rust,ignore
//! use core_kernel::registry::{DomainRegistry, DomainConfig, AdapterSource};
//!
//! // Create configuration
//! let config = DomainConfig {
//!     party_source: AdapterSource::Internal,
//!     // ... other domains
//! };
//!
//! // Build the registry with database pool
//! let registry = DomainRegistry::new(config, pool).await?;
//!
//! // Get the party port
//! let party_port = registry.party_port();
//! let party = party_port.get_party(party_id, None).await?;
//! ```
//!
//! # Configuration
//!
//! Each domain can be configured independently:
//!
//! ```rust,ignore
//! DomainConfig {
//!     party_source: AdapterSource::External(ExternalConfig {
//!         base_url: "https://crm.example.com".to_string(),
//!         api_key: "secret".to_string(),
//!         ..Default::default()
//!     }),
//!     policy_source: AdapterSource::Internal,
//!     // Internal source uses the database
//! }
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use crate::ports::{
    AdapterType, AdapterHealth, HealthCheckable, HealthCheckResult,
    ExternalSystemConfig,
};

/// Source configuration for an adapter
///
/// Specifies whether a domain should use an internal (database) or
/// external (API) adapter implementation.
#[derive(Debug, Clone)]
pub enum AdapterSource {
    /// Use the internal database adapter
    Internal,

    /// Use an external API adapter with the given configuration
    External(ExternalConfig),

    /// Use a mock adapter (for testing)
    Mock,
}

impl Default for AdapterSource {
    fn default() -> Self {
        AdapterSource::Internal
    }
}

/// Configuration for an external adapter
///
/// Contains all settings needed to connect to an external system.
#[derive(Debug, Clone, Default)]
pub struct ExternalConfig {
    /// Base URL of the external API
    pub base_url: String,

    /// API key for authentication
    pub api_key: Option<String>,

    /// OAuth2 client ID
    pub oauth_client_id: Option<String>,

    /// OAuth2 client secret
    pub oauth_client_secret: Option<String>,

    /// OAuth2 token URL
    pub oauth_token_url: Option<String>,

    /// Request timeout in seconds
    pub timeout_secs: u64,

    /// Number of retry attempts
    pub retry_attempts: u32,

    /// Additional headers to include in requests
    pub headers: HashMap<String, String>,
}

impl ExternalConfig {
    /// Creates a new external config with just a base URL and API key
    pub fn simple(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: Some(api_key.into()),
            timeout_secs: 30,
            retry_attempts: 3,
            ..Default::default()
        }
    }

    /// Creates a new external config with OAuth2 credentials
    pub fn oauth(
        base_url: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        token_url: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            oauth_client_id: Some(client_id.into()),
            oauth_client_secret: Some(client_secret.into()),
            oauth_token_url: Some(token_url.into()),
            timeout_secs: 30,
            retry_attempts: 3,
            ..Default::default()
        }
    }
}

/// Configuration for all domain adapters
///
/// Specifies which adapter implementation to use for each domain.
/// Domains not specified will use the internal (database) adapter.
#[derive(Debug, Clone, Default)]
pub struct DomainConfig {
    /// Party domain adapter source
    pub party_source: AdapterSource,

    /// Policy domain adapter source
    pub policy_source: AdapterSource,

    /// Claims domain adapter source
    pub claims_source: AdapterSource,

    /// Billing domain adapter source
    pub billing_source: AdapterSource,

    /// Fund domain adapter source
    pub fund_source: AdapterSource,
}

impl DomainConfig {
    /// Creates a configuration with all domains using internal adapters
    pub fn all_internal() -> Self {
        Self::default()
    }

    /// Creates a configuration with all domains using external adapters
    pub fn all_external(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        let config = ExternalConfig::simple(base_url, api_key);
        Self {
            party_source: AdapterSource::External(config.clone()),
            policy_source: AdapterSource::External(config.clone()),
            claims_source: AdapterSource::External(config.clone()),
            billing_source: AdapterSource::External(config.clone()),
            fund_source: AdapterSource::External(config),
        }
    }

    /// Sets the party domain to use an external adapter
    pub fn with_external_party(mut self, config: ExternalConfig) -> Self {
        self.party_source = AdapterSource::External(config);
        self
    }

    /// Sets the policy domain to use an external adapter
    pub fn with_external_policy(mut self, config: ExternalConfig) -> Self {
        self.policy_source = AdapterSource::External(config);
        self
    }

    /// Creates a configuration suitable for testing
    pub fn for_testing() -> Self {
        Self {
            party_source: AdapterSource::Mock,
            policy_source: AdapterSource::Mock,
            claims_source: AdapterSource::Mock,
            billing_source: AdapterSource::Mock,
            fund_source: AdapterSource::Mock,
        }
    }
}

/// Registration information for an adapter
///
/// Stores metadata about a registered adapter including its health status.
#[derive(Debug, Clone)]
pub struct AdapterRegistration {
    /// Unique identifier for this registration
    pub id: String,

    /// The domain this adapter serves
    pub domain: String,

    /// The type of adapter (internal, external, mock)
    pub adapter_type: AdapterType,

    /// Whether the adapter is currently active
    pub is_active: bool,

    /// Last health check result
    pub last_health_check: Option<HealthCheckResult>,
}

/// Result of health checks across all registered adapters
#[derive(Debug, Clone)]
pub struct RegistryHealthReport {
    /// Overall health status
    pub overall_status: AdapterHealth,

    /// Health results by domain
    pub domain_health: HashMap<String, HealthCheckResult>,

    /// Number of healthy adapters
    pub healthy_count: usize,

    /// Number of unhealthy adapters
    pub unhealthy_count: usize,

    /// Total number of registered adapters
    pub total_count: usize,
}

impl RegistryHealthReport {
    /// Returns true if all adapters are healthy
    pub fn is_fully_healthy(&self) -> bool {
        self.overall_status == AdapterHealth::Healthy
    }

    /// Returns true if the system is at least partially operational
    pub fn is_operational(&self) -> bool {
        matches!(
            self.overall_status,
            AdapterHealth::Healthy | AdapterHealth::Degraded
        )
    }
}

/// Trait for domain registries
///
/// This trait defines the interface that domain registries must implement.
/// It allows for type-safe access to domain ports while maintaining
/// the ability to swap implementations.
pub trait DomainPortRegistry: Send + Sync {
    /// Returns the current configuration
    fn config(&self) -> &DomainConfig;

    /// Returns a list of all registered adapters
    fn registrations(&self) -> Vec<AdapterRegistration>;

    /// Returns the registration for a specific domain
    fn get_registration(&self, domain: &str) -> Option<AdapterRegistration>;
}

/// Builder for creating domain registries
///
/// Provides a fluent interface for configuring and building a domain registry.
///
/// # Example
///
/// ```rust,ignore
/// let registry = DomainRegistryBuilder::new()
///     .with_party_external(ExternalConfig::simple("https://crm.example.com", "key"))
///     .with_policy_internal(pool.clone())
///     .build()
///     .await?;
/// ```
#[derive(Debug, Default)]
pub struct DomainRegistryBuilder {
    config: DomainConfig,
}

impl DomainRegistryBuilder {
    /// Creates a new registry builder with default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts with all internal adapters
    pub fn all_internal(mut self) -> Self {
        self.config = DomainConfig::all_internal();
        self
    }

    /// Starts with a testing configuration (all mocks)
    pub fn for_testing(mut self) -> Self {
        self.config = DomainConfig::for_testing();
        self
    }

    /// Sets the party domain to use an external adapter
    pub fn with_external_party(mut self, config: ExternalConfig) -> Self {
        self.config.party_source = AdapterSource::External(config);
        self
    }

    /// Sets the party domain to use the internal adapter
    pub fn with_internal_party(mut self) -> Self {
        self.config.party_source = AdapterSource::Internal;
        self
    }

    /// Sets the party domain to use a mock adapter
    pub fn with_mock_party(mut self) -> Self {
        self.config.party_source = AdapterSource::Mock;
        self
    }

    /// Sets the policy domain to use an external adapter
    pub fn with_external_policy(mut self, config: ExternalConfig) -> Self {
        self.config.policy_source = AdapterSource::External(config);
        self
    }

    /// Sets the policy domain to use the internal adapter
    pub fn with_internal_policy(mut self) -> Self {
        self.config.policy_source = AdapterSource::Internal;
        self
    }

    /// Returns the current configuration
    pub fn config(&self) -> &DomainConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_all_internal() {
        let config = DomainConfig::default();
        assert!(matches!(config.party_source, AdapterSource::Internal));
        assert!(matches!(config.policy_source, AdapterSource::Internal));
    }

    #[test]
    fn test_testing_config() {
        let config = DomainConfig::for_testing();
        assert!(matches!(config.party_source, AdapterSource::Mock));
        assert!(matches!(config.policy_source, AdapterSource::Mock));
    }

    #[test]
    fn test_external_config_simple() {
        let config = ExternalConfig::simple("https://api.example.com", "my-api-key");
        assert_eq!(config.base_url, "https://api.example.com");
        assert_eq!(config.api_key, Some("my-api-key".to_string()));
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.retry_attempts, 3);
    }

    #[test]
    fn test_external_config_oauth() {
        let config = ExternalConfig::oauth(
            "https://api.example.com",
            "client-id",
            "client-secret",
            "https://auth.example.com/token",
        );
        assert_eq!(config.base_url, "https://api.example.com");
        assert_eq!(config.oauth_client_id, Some("client-id".to_string()));
        assert_eq!(config.oauth_client_secret, Some("client-secret".to_string()));
        assert_eq!(config.oauth_token_url, Some("https://auth.example.com/token".to_string()));
    }

    #[test]
    fn test_builder_fluent_api() {
        let builder = DomainRegistryBuilder::new()
            .all_internal()
            .with_external_party(ExternalConfig::simple("https://crm.example.com", "key"));

        let config = builder.config();
        assert!(matches!(config.party_source, AdapterSource::External(_)));
        assert!(matches!(config.policy_source, AdapterSource::Internal));
    }

    #[test]
    fn test_config_with_external_party() {
        let config = DomainConfig::all_internal()
            .with_external_party(ExternalConfig::simple("https://party.example.com", "key"));

        assert!(matches!(config.party_source, AdapterSource::External(_)));
        assert!(matches!(config.policy_source, AdapterSource::Internal));
    }

    #[test]
    fn test_health_report() {
        let report = RegistryHealthReport {
            overall_status: AdapterHealth::Healthy,
            domain_health: HashMap::new(),
            healthy_count: 5,
            unhealthy_count: 0,
            total_count: 5,
        };

        assert!(report.is_fully_healthy());
        assert!(report.is_operational());
    }

    #[test]
    fn test_degraded_health_report() {
        let report = RegistryHealthReport {
            overall_status: AdapterHealth::Degraded,
            domain_health: HashMap::new(),
            healthy_count: 3,
            unhealthy_count: 2,
            total_count: 5,
        };

        assert!(!report.is_fully_healthy());
        assert!(report.is_operational());
    }

    #[test]
    fn test_unhealthy_report() {
        let report = RegistryHealthReport {
            overall_status: AdapterHealth::Unhealthy,
            domain_health: HashMap::new(),
            healthy_count: 0,
            unhealthy_count: 5,
            total_count: 5,
        };

        assert!(!report.is_fully_healthy());
        assert!(!report.is_operational());
    }
}
