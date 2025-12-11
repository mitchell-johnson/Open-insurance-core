//! Party domain errors

use thiserror::Error;

/// Errors that can occur in the party domain
#[derive(Debug, Error)]
pub enum PartyError {
    #[error("Party not found: {0}")]
    PartyNotFound(String),

    #[error("Duplicate party: {0}")]
    DuplicateParty(String),

    #[error("KYC verification failed: {0}")]
    KycFailed(String),

    #[error("Invalid party data: {0}")]
    InvalidData(String),

    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("License expired")]
    LicenseExpired,
}
