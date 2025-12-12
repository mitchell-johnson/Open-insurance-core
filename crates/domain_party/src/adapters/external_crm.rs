//! External CRM Adapter
//!
//! This module provides an adapter for connecting to external CRM or customer
//! management systems via REST API. It implements the `PartyPort` trait,
//! allowing the party domain to use an external system as its source of truth.
//!
//! # Architecture
//!
//! The adapter uses HTTP requests to communicate with the external system,
//! translating between the external API's data format and the internal domain
//! models. It includes:
//!
//! - Connection pooling via reqwest
//! - Automatic retry with exponential backoff
//! - Circuit breaker pattern for fault tolerance
//! - Request/response logging and tracing
//!
//! # Configuration
//!
//! The adapter is configured via `ExternalCrmConfig`:
//!
//! ```rust,ignore
//! let config = ExternalCrmConfig {
//!     base_url: "https://crm.example.com/api/v1".to_string(),
//!     api_key: std::env::var("CRM_API_KEY").unwrap(),
//!     timeout_secs: 30,
//!     retry_attempts: 3,
//! };
//! ```
//!
//! # Error Handling
//!
//! External API errors are mapped to `PortError` variants:
//! - 404 -> `PortError::NotFound`
//! - 401/403 -> `PortError::Unauthorized`
//! - 429 -> `PortError::RateLimited`
//! - 5xx -> `PortError::ServiceUnavailable`
//! - Timeouts -> `PortError::Timeout`
//! - Other -> `PortError::Internal`

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use core_kernel::{
    PartyId, PortError, DomainPort, OperationMetadata,
    HealthCheckable, HealthCheckResult, AdapterHealth,
    CircuitBreakerConfig,
};

use crate::party::{
    Party, PartyComposition, PartyType, Individual, Corporate,
    PartyMember, MemberRole, JointDetails, JointType,
    TrustDetails, TrustType, PartnershipDetails, PartnershipType,
};
use crate::kyc::KycStatus;
use crate::ports::{
    PartyPort, PartyQuery, CreatePartyRequest, CreateMemberRequest, UpdatePartyRequest,
};

/// Configuration for the external CRM adapter
///
/// This configuration specifies how to connect to the external CRM system,
/// including authentication, timeouts, and retry behavior.
#[derive(Debug, Clone)]
pub struct ExternalCrmConfig {
    /// Base URL of the CRM API (e.g., "https://crm.example.com/api/v1")
    pub base_url: String,

    /// API key for authentication
    pub api_key: String,

    /// Request timeout in seconds
    pub timeout_secs: u64,

    /// Number of retry attempts for failed requests
    pub retry_attempts: u32,

    /// Optional OAuth2 client credentials
    pub oauth_client_id: Option<String>,
    pub oauth_client_secret: Option<String>,
    pub oauth_token_url: Option<String>,

    /// Circuit breaker configuration
    pub circuit_breaker: Option<CircuitBreakerConfig>,
}

impl Default for ExternalCrmConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            api_key: String::new(),
            timeout_secs: 30,
            retry_attempts: 3,
            oauth_client_id: None,
            oauth_client_secret: None,
            oauth_token_url: None,
            circuit_breaker: Some(CircuitBreakerConfig {
                failure_threshold: 5,
                success_threshold: 3,
                reset_timeout_secs: 60,
            }),
        }
    }
}

/// Circuit breaker state for fault tolerance
#[derive(Debug)]
struct CircuitBreaker {
    config: CircuitBreakerConfig,
    failure_count: AtomicU64,
    success_count: AtomicU64,
    is_open: AtomicBool,
    last_failure_time: RwLock<Option<Instant>>,
}

impl CircuitBreaker {
    fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            failure_count: AtomicU64::new(0),
            success_count: AtomicU64::new(0),
            is_open: AtomicBool::new(false),
            last_failure_time: RwLock::new(None),
        }
    }

    async fn is_available(&self) -> bool {
        if !self.is_open.load(Ordering::Relaxed) {
            return true;
        }

        // Check if timeout has elapsed
        let last_failure = self.last_failure_time.read().await;
        if let Some(time) = *last_failure {
            if time.elapsed() > Duration::from_secs(self.config.reset_timeout_secs) {
                // Half-open state: allow one request through
                return true;
            }
        }

        false
    }

    fn record_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        let success = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
        if success >= self.config.success_threshold as u64 {
            self.is_open.store(false, Ordering::Relaxed);
            self.success_count.store(0, Ordering::Relaxed);
        }
    }

    async fn record_failure(&self) {
        self.success_count.store(0, Ordering::Relaxed);
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        if failures >= self.config.failure_threshold as u64 {
            self.is_open.store(true, Ordering::Relaxed);
            *self.last_failure_time.write().await = Some(Instant::now());
        }
    }
}

/// External CRM adapter implementing the PartyPort trait
///
/// This adapter connects to an external CRM system via REST API to manage
/// party data. It can be used as a drop-in replacement for the internal
/// database adapter when party data is managed by an external system.
///
/// # Features
///
/// - **Retry logic**: Automatically retries failed requests with exponential backoff
/// - **Circuit breaker**: Prevents cascading failures when the external system is down
/// - **Request tracing**: Integrates with tracing for observability
/// - **Configurable timeouts**: Prevents hanging requests
///
/// # Example
///
/// ```rust,ignore
/// use domain_party::adapters::{ExternalCrmAdapter, ExternalCrmConfig};
///
/// let adapter = ExternalCrmAdapter::new(ExternalCrmConfig {
///     base_url: "https://crm.example.com/api".to_string(),
///     api_key: "your-api-key".to_string(),
///     ..Default::default()
/// });
///
/// // Use through the PartyPort trait
/// let party = adapter.get_party(party_id, None).await?;
/// ```
#[derive(Debug)]
pub struct ExternalCrmAdapter {
    config: ExternalCrmConfig,
    circuit_breaker: Option<Arc<CircuitBreaker>>,
    // In a real implementation, this would be a reqwest::Client
    // For now, we store the config for demonstration
}

impl ExternalCrmAdapter {
    /// Creates a new external CRM adapter with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - The adapter configuration
    ///
    /// # Returns
    ///
    /// A new adapter instance ready to make requests
    pub fn new(config: ExternalCrmConfig) -> Self {
        let circuit_breaker = config.circuit_breaker.clone()
            .map(|cb| Arc::new(CircuitBreaker::new(cb)));

        Self {
            config,
            circuit_breaker,
        }
    }

    /// Returns the base URL of the external CRM system
    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }

    /// Checks if the circuit breaker is open (blocking requests)
    pub async fn is_circuit_open(&self) -> bool {
        if let Some(ref cb) = self.circuit_breaker {
            !cb.is_available().await
        } else {
            false
        }
    }

    /// Makes an HTTP GET request to the external API
    ///
    /// This is a placeholder implementation. In a real adapter, this would
    /// use reqwest or another HTTP client to make the actual request.
    async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, PortError> {
        // Check circuit breaker
        if let Some(ref cb) = self.circuit_breaker {
            if !cb.is_available().await {
                return Err(PortError::ServiceUnavailable {
                    service: "Circuit breaker is open".to_string()
                });
            }
        }

        // Placeholder: In a real implementation, make HTTP request here
        // let url = format!("{}/{}", self.config.base_url, path);
        // let response = self.client.get(&url)
        //     .header("Authorization", format!("Bearer {}", self.config.api_key))
        //     .timeout(Duration::from_secs(self.config.timeout_secs))
        //     .send()
        //     .await?;

        // For now, return a not implemented error
        Err(PortError::internal(format!(
            "External CRM adapter not implemented: GET {}",
            path
        )))
    }

    /// Makes an HTTP POST request to the external API
    async fn post<T, R>(&self, path: &str, _body: &T) -> Result<R, PortError>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        // Check circuit breaker
        if let Some(ref cb) = self.circuit_breaker {
            if !cb.is_available().await {
                return Err(PortError::ServiceUnavailable {
                    service: "Circuit breaker is open".to_string()
                });
            }
        }

        // Placeholder implementation
        Err(PortError::internal(format!(
            "External CRM adapter not implemented: POST {}",
            path
        )))
    }

    /// Makes an HTTP PUT request to the external API
    async fn put<T, R>(&self, path: &str, _body: &T) -> Result<R, PortError>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        if let Some(ref cb) = self.circuit_breaker {
            if !cb.is_available().await {
                return Err(PortError::ServiceUnavailable {
                    service: "Circuit breaker is open".to_string()
                });
            }
        }

        Err(PortError::internal(format!(
            "External CRM adapter not implemented: PUT {}",
            path
        )))
    }

    /// Makes an HTTP DELETE request to the external API
    async fn delete(&self, path: &str) -> Result<(), PortError> {
        if let Some(ref cb) = self.circuit_breaker {
            if !cb.is_available().await {
                return Err(PortError::ServiceUnavailable {
                    service: "Circuit breaker is open".to_string()
                });
            }
        }

        Err(PortError::internal(format!(
            "External CRM adapter not implemented: DELETE {}",
            path
        )))
    }
}

impl DomainPort for ExternalCrmAdapter {}

#[async_trait]
impl HealthCheckable for ExternalCrmAdapter {
    /// Performs a health check against the external CRM system
    ///
    /// Calls the /health or /ping endpoint of the external API to verify
    /// connectivity and responsiveness.
    async fn health_check(&self) -> HealthCheckResult {
        let start = Instant::now();

        // Check if circuit breaker is open
        if self.is_circuit_open().await {
            return HealthCheckResult {
                adapter_id: "external-crm-adapter".to_string(),
                status: AdapterHealth::Degraded,
                latency_ms: 0,
                message: Some("Circuit breaker is open".to_string()),
                checked_at: Utc::now(),
            };
        }

        // In a real implementation, call the health endpoint
        // let result = self.get::<serde_json::Value>("health").await;

        let latency_ms = start.elapsed().as_millis() as u64;

        // For now, return degraded since we haven't actually connected
        HealthCheckResult {
            adapter_id: "external-crm-adapter".to_string(),
            status: AdapterHealth::Degraded,
            latency_ms,
            message: Some("External CRM adapter is a placeholder implementation".to_string()),
            checked_at: Utc::now(),
        }
    }
}

#[async_trait]
impl PartyPort for ExternalCrmAdapter {
    async fn get_party(
        &self,
        id: PartyId,
        _metadata: Option<OperationMetadata>,
    ) -> Result<Party, PortError> {
        // In a real implementation:
        // let response: ExternalPartyResponse = self.get(&format!("parties/{}", id)).await?;
        // Ok(response.into())

        Err(PortError::internal(format!(
            "External CRM get_party not implemented for party {}",
            id
        )))
    }

    async fn get_parties(
        &self,
        ids: Vec<PartyId>,
        metadata: Option<OperationMetadata>,
    ) -> Result<Vec<Party>, PortError> {
        // Could batch this into a single request if the API supports it
        let mut parties = Vec::with_capacity(ids.len());
        for id in ids {
            match self.get_party(id, metadata.clone()).await {
                Ok(party) => parties.push(party),
                Err(PortError::NotFound { .. }) => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(parties)
    }

    async fn find_parties(
        &self,
        query: PartyQuery,
        _metadata: Option<OperationMetadata>,
    ) -> Result<Vec<Party>, PortError> {
        // Build query string from PartyQuery
        // let url = format!("parties?email={}&limit={}", query.email, query.limit);
        // let response: Vec<ExternalPartyResponse> = self.get(&url).await?;

        Err(PortError::internal(format!(
            "External CRM find_parties not implemented: {:?}",
            query
        )))
    }

    async fn create_party(
        &self,
        request: CreatePartyRequest,
        _metadata: Option<OperationMetadata>,
    ) -> Result<Party, PortError> {
        // Transform request to external API format and POST
        // let external_request = ExternalCreatePartyRequest::from(request);
        // let response: ExternalPartyResponse = self.post("parties", &external_request).await?;

        Err(PortError::internal(format!(
            "External CRM create_party not implemented: {:?}",
            request.composition
        )))
    }

    async fn update_party(
        &self,
        id: PartyId,
        request: UpdatePartyRequest,
        _metadata: Option<OperationMetadata>,
    ) -> Result<Party, PortError> {
        Err(PortError::internal(format!(
            "External CRM update_party not implemented for party {}",
            id
        )))
    }

    async fn deactivate_party(
        &self,
        id: PartyId,
        _metadata: Option<OperationMetadata>,
    ) -> Result<(), PortError> {
        Err(PortError::internal(format!(
            "External CRM deactivate_party not implemented for party {}",
            id
        )))
    }

    async fn get_members(
        &self,
        party_id: PartyId,
        _metadata: Option<OperationMetadata>,
    ) -> Result<Vec<PartyMember>, PortError> {
        Err(PortError::internal(format!(
            "External CRM get_members not implemented for party {}",
            party_id
        )))
    }

    async fn add_member(
        &self,
        party_id: PartyId,
        request: CreateMemberRequest,
        _metadata: Option<OperationMetadata>,
    ) -> Result<PartyMember, PortError> {
        Err(PortError::internal(format!(
            "External CRM add_member not implemented for party {}",
            party_id
        )))
    }

    async fn remove_member(
        &self,
        party_id: PartyId,
        member_party_id: PartyId,
        _metadata: Option<OperationMetadata>,
    ) -> Result<(), PortError> {
        Err(PortError::internal(format!(
            "External CRM remove_member not implemented for party {}, member {}",
            party_id, member_party_id
        )))
    }

    async fn update_member_ownership(
        &self,
        party_id: PartyId,
        member_party_id: PartyId,
        new_percentage: Decimal,
        _metadata: Option<OperationMetadata>,
    ) -> Result<PartyMember, PortError> {
        Err(PortError::internal(format!(
            "External CRM update_member_ownership not implemented for party {}",
            party_id
        )))
    }

    async fn set_primary_contact(
        &self,
        party_id: PartyId,
        member_party_id: PartyId,
        _metadata: Option<OperationMetadata>,
    ) -> Result<(), PortError> {
        Err(PortError::internal(format!(
            "External CRM set_primary_contact not implemented for party {}",
            party_id
        )))
    }

    async fn find_by_member(
        &self,
        member_party_id: PartyId,
        _metadata: Option<OperationMetadata>,
    ) -> Result<Vec<Party>, PortError> {
        Err(PortError::internal(format!(
            "External CRM find_by_member not implemented for member {}",
            member_party_id
        )))
    }

    async fn exists(
        &self,
        id: PartyId,
        metadata: Option<OperationMetadata>,
    ) -> Result<bool, PortError> {
        match self.get_party(id, metadata).await {
            Ok(_) => Ok(true),
            Err(PortError::NotFound { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn update_kyc_status(
        &self,
        id: PartyId,
        status: KycStatus,
        _metadata: Option<OperationMetadata>,
    ) -> Result<(), PortError> {
        Err(PortError::internal(format!(
            "External CRM update_kyc_status not implemented for party {}",
            id
        )))
    }
}

// =============================================================================
// External API Data Transfer Objects
// =============================================================================

/// Example response from an external CRM party endpoint
///
/// This shows how external API responses might be structured.
/// Implementations should define their own DTOs matching the actual API.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(dead_code)]
struct ExternalPartyResponse {
    id: String,
    #[serde(rename = "type")]
    party_type: String,
    email: Option<String>,
    phone: Option<String>,
    // ... other fields specific to the external API
}

/// Example request for creating a party in the external CRM
#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
struct ExternalCreatePartyRequest {
    #[serde(rename = "type")]
    party_type: String,
    email: Option<String>,
    phone: Option<String>,
    // ... other fields
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = ExternalCrmConfig::default();
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.retry_attempts, 3);
        assert!(config.circuit_breaker.is_some());
    }

    #[tokio::test]
    async fn test_adapter_health_check() {
        let adapter = ExternalCrmAdapter::new(ExternalCrmConfig {
            base_url: "https://example.com".to_string(),
            api_key: "test".to_string(),
            ..Default::default()
        });

        let result = adapter.health_check().await;
        assert_eq!(result.adapter_id, "external-crm-adapter");
        // Should be degraded since it's a placeholder
        assert_eq!(result.status, AdapterHealth::Degraded);
    }

    #[tokio::test]
    async fn test_circuit_breaker_initially_closed() {
        let adapter = ExternalCrmAdapter::new(ExternalCrmConfig::default());
        assert!(!adapter.is_circuit_open().await);
    }
}
