//! Claims Management Domain
//!
//! This crate implements the claims lifecycle from First Notice of Loss (FNOL)
//! through adjudication, reserving, and settlement.
//!
//! # Claim Lifecycle
//!
//! ```text
//! FNOL -> Under Investigation -> Under Review -> Approved/Denied -> Paid/Closed
//! ```

pub mod claim;
pub mod reserve;
pub mod payment;
pub mod adjudication;
pub mod workflow;
pub mod error;

pub use claim::{Claim, ClaimStatus, LossType};
pub use reserve::{Reserve, ReserveType};
pub use payment::{ClaimPayment, PaymentType};
pub use adjudication::{AdjudicationDecision, AdjudicationReason};
pub use error::ClaimError;
