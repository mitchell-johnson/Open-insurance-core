//! Policy domain errors
//!
//! This module defines all error types that can occur within the
//! policy administration domain.

use thiserror::Error;

/// Errors that can occur in the policy domain
#[derive(Debug, Error)]
pub enum PolicyError {
    /// Invalid state transition attempted
    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition {
        from: String,
        to: String,
    },

    /// Policy cannot be modified in current state
    #[error("Policy cannot be modified in current state")]
    NotModifiable,

    /// Reinstatement period has expired
    #[error("Reinstatement period has expired")]
    ReinstatementPeriodExpired,

    /// Required field is missing
    #[error("Missing required field: {0}")]
    MissingRequiredField(String),

    /// Currency mismatch between policy and payment
    #[error("Currency mismatch: expected {expected}, got {actual}")]
    CurrencyMismatch {
        expected: String,
        actual: String,
    },

    /// Financial calculation error
    #[error("Financial error: {0}")]
    Financial(String),

    /// Coverage not found
    #[error("Coverage not found: {0}")]
    CoverageNotFound(String),

    /// Invalid coverage configuration
    #[error("Invalid coverage: {0}")]
    InvalidCoverage(String),

    /// Premium calculation error
    #[error("Premium calculation error: {0}")]
    PremiumCalculation(String),

    /// Underwriting error
    #[error("Underwriting error: {0}")]
    Underwriting(String),

    /// Product rule violation
    #[error("Product rule violation: {0}")]
    ProductRuleViolation(String),

    /// Quote has expired
    #[error("Quote has expired")]
    QuoteExpired,

    /// Endorsement error
    #[error("Endorsement error: {0}")]
    Endorsement(String),

    /// Beneficiary validation error
    #[error("Beneficiary error: {0}")]
    BeneficiaryError(String),

    /// Policy loan error
    #[error("Policy loan error: {0}")]
    PolicyLoan(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// External service error
    #[error("External service error: {0}")]
    ExternalService(String),
}

impl PolicyError {
    /// Creates a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        PolicyError::Validation(message.into())
    }

    /// Creates a coverage not found error
    pub fn coverage_not_found(coverage_id: impl std::fmt::Display) -> Self {
        PolicyError::CoverageNotFound(coverage_id.to_string())
    }

    /// Creates a product rule violation error
    pub fn rule_violation(rule: impl Into<String>) -> Self {
        PolicyError::ProductRuleViolation(rule.into())
    }
}
