//! Policy Administration Domain
//!
//! This crate implements the core policy administration logic for the insurance
//! system, following Domain-Driven Design (DDD) and Hexagonal Architecture principles.
//!
//! # Architecture
//!
//! The domain layer is infrastructure-agnostic, containing only business logic:
//! - **Aggregates**: Policy is the main aggregate root
//! - **Value Objects**: Coverage, Premium, RiskObject
//! - **Domain Services**: Underwriting, Rating, Endorsement processing
//! - **Domain Events**: PolicyIssued, PolicyEndorsed, PolicyLapsed
//!
//! # Policy Lifecycle
//!
//! ```text
//! Quoted -> InForce -> Lapsed -> Reinstated -> InForce
//!                  \-> Terminated
//!                  \-> Cancelled
//!                  \-> Expired
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use domain_policy::{Policy, PolicyBuilder, Coverage};
//!
//! let policy = PolicyBuilder::new()
//!     .product_code("TERM_LIFE_20")
//!     .policyholder(party_id)
//!     .add_coverage(Coverage::death_benefit(sum_assured))
//!     .effective_date(start_date)
//!     .term_years(20)
//!     .build()?;
//!
//! policy.issue(underwriter)?;
//! ```

pub mod aggregate;
pub mod coverage;
pub mod premium;
pub mod endorsement;
pub mod underwriting;
pub mod events;
pub mod error;
pub mod services;
pub mod rules_engine;

pub use aggregate::{Policy, PolicyState, PolicyBuilder};
pub use coverage::{Coverage, CoverageType, Benefit};
pub use premium::{Premium, PremiumFrequency, PremiumSchedule};
pub use endorsement::{Endorsement, EndorsementType};
pub use events::PolicyEvent;
pub use error::PolicyError;
pub use services::{UnderwritingService, RatingService};
pub use rules_engine::{RulesEngine, ProductRules, EvaluationResult, ProductMetadata, RulesError};
