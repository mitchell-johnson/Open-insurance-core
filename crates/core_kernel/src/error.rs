//! Core error types used across the system

use thiserror::Error;
use crate::money::MoneyError;
use crate::temporal::TemporalError;

/// Core error type for the kernel
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Money error: {0}")]
    Money(#[from] MoneyError),

    #[error("Temporal error: {0}")]
    Temporal(#[from] TemporalError),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Invalid state transition: {0}")]
    InvalidStateTransition(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Configuration error: {0}")]
    Configuration(String),
}

impl CoreError {
    pub fn validation(message: impl Into<String>) -> Self {
        CoreError::Validation(message.into())
    }

    pub fn invalid_state(message: impl Into<String>) -> Self {
        CoreError::InvalidStateTransition(message.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        CoreError::NotFound(message.into())
    }
}
