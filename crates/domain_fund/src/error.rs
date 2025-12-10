//! Fund domain errors

use thiserror::Error;

/// Errors that can occur in the fund domain
#[derive(Debug, Error)]
pub enum FundError {
    #[error("Fund not found: {0}")]
    FundNotFound(String),

    #[error("NAV not found: {0}")]
    NavNotFound(String),

    #[error("Invalid allocation: {0}")]
    InvalidAllocation(String),

    #[error("Insufficient units: {0}")]
    InsufficientUnits(String),

    #[error("Fund is closed for new investments")]
    FundClosed,

    #[error("Calculation error: {0}")]
    CalculationError(String),
}
