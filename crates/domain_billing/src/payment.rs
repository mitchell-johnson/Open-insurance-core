//! Payment processing
//!
//! This module handles payment recording and processing.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use core_kernel::{PaymentId, InvoiceId, PartyId, Money};

/// Payment method
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentMethod {
    /// Bank transfer
    BankTransfer,
    /// Credit card
    CreditCard,
    /// Debit card
    DebitCard,
    /// Direct debit
    DirectDebit,
    /// Check/cheque
    Check,
    /// Cash
    Cash,
    /// Digital wallet
    DigitalWallet,
    /// Auto-deduction from salary
    SalaryDeduction,
}

/// Payment status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentStatus {
    /// Payment is being processed
    Pending,
    /// Payment completed successfully
    Completed,
    /// Payment failed
    Failed,
    /// Payment was reversed/refunded
    Reversed,
    /// Payment is on hold
    OnHold,
}

/// A payment record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    /// Unique identifier
    pub id: PaymentId,
    /// Invoice being paid
    pub invoice_id: InvoiceId,
    /// Payer ID
    pub payer_id: PartyId,
    /// Payment amount
    pub amount: Money,
    /// Payment method
    pub method: PaymentMethod,
    /// External reference (bank ref, transaction ID)
    pub external_reference: Option<String>,
    /// Status
    pub status: PaymentStatus,
    /// Payment date
    pub payment_date: DateTime<Utc>,
    /// When status changed to completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Notes
    pub notes: Option<String>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

impl Payment {
    /// Creates a new payment
    ///
    /// # Arguments
    ///
    /// * `invoice_id` - Invoice being paid
    /// * `payer_id` - Who is paying
    /// * `amount` - Payment amount
    /// * `method` - Payment method
    pub fn new(
        invoice_id: InvoiceId,
        payer_id: PartyId,
        amount: Money,
        method: PaymentMethod,
    ) -> Self {
        let now = Utc::now();

        Self {
            id: PaymentId::new_v7(),
            invoice_id,
            payer_id,
            amount,
            method,
            external_reference: None,
            status: PaymentStatus::Pending,
            payment_date: now,
            completed_at: None,
            notes: None,
            created_at: now,
        }
    }

    /// Sets the external reference
    pub fn with_reference(mut self, reference: impl Into<String>) -> Self {
        self.external_reference = Some(reference.into());
        self
    }

    /// Marks the payment as completed
    pub fn complete(&mut self) {
        self.status = PaymentStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    /// Marks the payment as failed
    pub fn fail(&mut self, reason: &str) {
        self.status = PaymentStatus::Failed;
        self.notes = Some(reason.to_string());
    }

    /// Reverses the payment
    pub fn reverse(&mut self, reason: &str) {
        self.status = PaymentStatus::Reversed;
        self.notes = Some(format!("Reversed: {}", reason));
    }
}

/// Payment allocation to invoices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentAllocation {
    /// Payment ID
    pub payment_id: PaymentId,
    /// Invoice ID
    pub invoice_id: InvoiceId,
    /// Amount allocated to this invoice
    pub amount: Money,
    /// Allocation date
    pub allocated_at: DateTime<Utc>,
}
