//! Infrastructure Database Layer
//!
//! This crate provides the database infrastructure for the insurance core system,
//! implementing bi-temporal data patterns on PostgreSQL using SQLx.
//!
//! # Architecture
//!
//! The crate follows the repository pattern, providing data access abstractions
//! that hide the database implementation details from the domain layer.
//!
//! # Bi-Temporal Data Model
//!
//! All temporal entities support two time dimensions:
//! - **Valid Time**: When a fact is true in the real world
//! - **System Time**: When the fact was recorded in the database
//!
//! This enables "time travel" queries and full audit trails.
//!
//! # Example
//!
//! ```rust,ignore
//! use infra_db::{DatabasePool, create_pool};
//!
//! let pool = create_pool("postgres://localhost/insurance").await?;
//! let repo = PolicyRepository::new(pool);
//! ```

pub mod pool;
pub mod error;
pub mod bitemporal;
pub mod repositories;

pub use pool::{DatabasePool, create_pool, DatabaseConfig};
pub use error::DatabaseError;
pub use bitemporal::{BiTemporalRepository, BiTemporalQuery};
