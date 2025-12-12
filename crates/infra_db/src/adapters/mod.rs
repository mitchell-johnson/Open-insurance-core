//! Domain Adapters
//!
//! This module provides adapter implementations for domain ports,
//! connecting domain interfaces to the PostgreSQL database layer.
//!
//! # Architecture
//!
//! Each domain has a corresponding adapter that:
//! - Implements the domain's port trait
//! - Translates between domain models and database row types
//! - Uses the repository layer for database operations
//!
//! # Usage
//!
//! ```rust,ignore
//! use infra_db::adapters::PostgresPartyAdapter;
//! use domain_party::PartyPort;
//!
//! let adapter = PostgresPartyAdapter::new(pool);
//! let party = adapter.get_party(party_id, None).await?;
//! ```

pub mod party;

pub use party::PostgresPartyAdapter;
