//! Claims domain errors

use thiserror::Error;

/// Errors that can occur in the claims domain
#[derive(Debug, Error)]
pub enum ClaimError {
    #[error("Claim not found: {0}")]
    ClaimNotFound(String),

    #[error("Invalid status transition from {from} to {to}")]
    InvalidStatusTransition { from: String, to: String },

    #[error("Policy not found: {0}")]
    PolicyNotFound(String),

    #[error("Coverage not in force at loss date")]
    CoverageNotInForce,

    #[error("Invalid reserve: {0}")]
    InvalidReserve(String),

    #[error("Payment exceeds approved amount")]
    PaymentExceedsApproved,

    #[error("Claim already closed")]
    ClaimClosed,
}
