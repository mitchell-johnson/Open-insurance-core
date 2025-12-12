//! Core Kernel - Foundational types and utilities for the insurance system
//!
//! This crate provides the fundamental building blocks used across all domain modules:
//! - Money types with precise decimal arithmetic
//! - Temporal types for bi-temporal data handling
//! - Common identifiers and value objects
//! - Ports infrastructure for hexagonal architecture
//!
//! # Ports and Adapters
//!
//! The `ports` module provides the foundation for implementing swappable domain
//! implementations. Each domain can define its own port trait, and multiple adapters
//! can implement that trait (internal database, external API, mock, etc.).
//!
//! ```rust,ignore
//! use core_kernel::ports::{DomainPort, PortError, AdapterConfig};
//!
//! // Define a port in your domain
//! #[async_trait]
//! pub trait PartyPort: DomainPort {
//!     async fn get_party(&self, id: PartyId) -> Result<Party, PortError>;
//! }
//!
//! // Implement with internal adapter (database)
//! impl PartyPort for PostgresPartyAdapter { ... }
//!
//! // Or implement with external adapter (API)
//! impl PartyPort for ExternalCrmAdapter { ... }
//! ```

pub mod money;
pub mod temporal;
pub mod identifiers;
pub mod error;
pub mod ports;
pub mod registry;

pub use money::{Money, Currency, MoneyError};
pub use temporal::{ValidPeriod, SystemPeriod, BiTemporalRecord, Timezone};
pub use identifiers::{
    PolicyId, ClaimId, PartyId, AccountId, JournalEntryId,
    FundId, UnitHoldingId, VersionId, AgentId, AddressId,
    PolicyVersionId, CoverageId, EndorsementId, ClaimLineId,
    ReserveId, PaymentId, ContactId, PostingId, InvoiceId,
    NavId, TransactionId, AuditEventId, RiskObjectId,
};
pub use error::CoreError;
pub use ports::{
    PortError, DomainPort, AdapterConfig, AdapterType,
    ExternalSystemConfig, ExternalAuthConfig,
    HealthCheckable, HealthCheckResult, AdapterHealth,
    OperationMetadata, CircuitBreakerConfig,
};
pub use registry::{
    DomainConfig, AdapterSource, ExternalConfig,
    DomainRegistryBuilder, DomainPortRegistry,
    AdapterRegistration, RegistryHealthReport,
};
