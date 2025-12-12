//! Party domain errors
//!
//! This module defines all error types that can occur in the party domain,
//! including validation errors, not found errors, and composition errors.

use thiserror::Error;

/// Errors that can occur in the party domain
#[derive(Debug, Error)]
pub enum PartyError {
    /// Party with the given ID was not found
    #[error("Party not found: {0}")]
    PartyNotFound(String),

    /// Attempted to create a party that already exists
    #[error("Duplicate party: {0}")]
    DuplicateParty(String),

    /// KYC verification failed
    #[error("KYC verification failed: {0}")]
    KycFailed(String),

    /// Invalid party data provided
    #[error("Invalid party data: {0}")]
    InvalidData(String),

    /// Agent not found
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    /// Agent license has expired
    #[error("License expired")]
    LicenseExpired,

    /// Invalid party composition
    #[error("Invalid party composition: {0}")]
    InvalidComposition(String),

    /// Member not found in party
    #[error("Member not found: {0}")]
    MemberNotFound(String),

    /// Invalid member role for party type
    #[error("Invalid member role {0} for party type {1}")]
    InvalidMemberRole(String, String),

    /// Invalid ownership percentage
    #[error("Invalid ownership percentage: {0}")]
    InvalidOwnership(String),

    /// Party validation failed
    #[error("Party validation failed: {0}")]
    ValidationFailed(String),

    /// Cannot modify inactive party
    #[error("Cannot modify inactive party")]
    InactiveParty,

    /// Operation not allowed for party type
    #[error("Operation not allowed for {0} party")]
    OperationNotAllowed(String),

    /// Circular reference detected in party membership
    #[error("Circular reference detected: {0}")]
    CircularReference(String),
}

impl PartyError {
    /// Creates a PartyNotFound error from any ID type
    pub fn not_found(id: impl std::fmt::Display) -> Self {
        PartyError::PartyNotFound(id.to_string())
    }

    /// Creates an InvalidData error with a message
    pub fn invalid(message: impl Into<String>) -> Self {
        PartyError::InvalidData(message.into())
    }

    /// Creates a ValidationFailed error from validation errors
    pub fn validation_failed(errors: Vec<String>) -> Self {
        PartyError::ValidationFailed(errors.join("; "))
    }

    /// Creates an InvalidComposition error
    pub fn invalid_composition(message: impl Into<String>) -> Self {
        PartyError::InvalidComposition(message.into())
    }

    /// Creates an InvalidOwnership error
    pub fn invalid_ownership(message: impl Into<String>) -> Self {
        PartyError::InvalidOwnership(message.into())
    }
}
