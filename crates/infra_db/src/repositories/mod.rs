//! Repository implementations for domain entities
//!
//! This module provides concrete repository implementations that handle
//! database access for each domain aggregate. Repositories encapsulate
//! SQL queries and map between database rows and domain types.
//!
//! # Architecture
//!
//! Each repository follows these principles:
//! - Bi-temporal data handling by default
//! - Compile-time query verification with SQLx
//! - Transaction support for complex operations
//! - Optimistic concurrency control where needed

pub mod policy;
pub mod party;
pub mod billing;
pub mod fund;
pub mod claims;

pub use policy::PolicyRepository;
pub use party::PartyRepository;
pub use billing::BillingRepository;
pub use fund::FundRepository;
pub use claims::ClaimsRepository;
