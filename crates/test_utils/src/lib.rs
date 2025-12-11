//! Test Utilities Crate
//!
//! Provides shared test infrastructure, fixtures, and helpers for the
//! Open Insurance Core test suite.
//!
//! # Modules
//!
//! - `fixtures`: Pre-built test data for common entities
//! - `builders`: Builder patterns for test data construction
//! - `database`: Database test helpers and container management
//! - `assertions`: Custom assertion helpers for domain types
//! - `generators`: Property-based test data generators

pub mod fixtures;
pub mod builders;
pub mod database;
pub mod assertions;
pub mod generators;

pub use fixtures::*;
pub use builders::*;
pub use database::*;
pub use assertions::*;
pub use generators::*;
