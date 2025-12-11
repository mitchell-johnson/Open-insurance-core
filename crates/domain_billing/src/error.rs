//! Billing domain errors

use rust_decimal::Decimal;
use thiserror::Error;

/// Errors that can occur in the billing domain
#[derive(Debug, Error)]
pub enum BillingError {
    /// Account not found
    #[error("Account not found: {0}")]
    AccountNotFound(String),

    /// Account already exists
    #[error("Account already exists: {0}")]
    AccountAlreadyExists(String),

    /// Journal entry not found
    #[error("Journal entry not found: {0}")]
    EntryNotFound(String),

    /// Transaction is not balanced
    #[error("Unbalanced transaction: debits={debits}, credits={credits}")]
    UnbalancedTransaction {
        debits: Decimal,
        credits: Decimal,
    },

    /// Calculation error
    #[error("Calculation error: {0}")]
    CalculationError(String),

    /// Invalid posting
    #[error("Invalid posting: {0}")]
    InvalidPosting(String),

    /// Invoice not found
    #[error("Invoice not found: {0}")]
    InvoiceNotFound(String),

    /// Payment not found
    #[error("Payment not found: {0}")]
    PaymentNotFound(String),

    /// Insufficient funds
    #[error("Insufficient funds in account: {0}")]
    InsufficientFunds(String),

    /// Invalid operation
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}
