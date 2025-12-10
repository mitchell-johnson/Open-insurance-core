//! Core Kernel - Foundational types and utilities for the insurance system
//!
//! This crate provides the fundamental building blocks used across all domain modules:
//! - Money types with precise decimal arithmetic
//! - Temporal types for bi-temporal data handling
//! - Common identifiers and value objects

pub mod money;
pub mod temporal;
pub mod identifiers;
pub mod error;

pub use money::{Money, Currency, MoneyError};
pub use temporal::{ValidPeriod, SystemPeriod, BiTemporalRecord, Timezone};
pub use identifiers::{
    PolicyId, ClaimId, PartyId, AccountId, JournalEntryId,
    FundId, UnitHoldingId, VersionId,
};
pub use error::CoreError;
