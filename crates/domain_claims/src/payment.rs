//! Claim payments

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use core_kernel::{ClaimId, PartyId, Money};

/// Payment type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentType {
    Indemnity,
    Expense,
    Partial,
    FinalSettlement,
}

/// Payment method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentMethod {
    BankTransfer,
    Check,
    DirectDeposit,
    Wire,
}

/// A claim payment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimPayment {
    pub id: Uuid,
    pub claim_id: ClaimId,
    pub payee_id: PartyId,
    pub amount: Money,
    pub payment_type: PaymentType,
    pub payment_method: PaymentMethod,
    pub reference: Option<String>,
    pub paid_at: DateTime<Utc>,
}

impl ClaimPayment {
    /// Creates a new claim payment
    pub fn new(
        claim_id: ClaimId,
        payee_id: PartyId,
        amount: Money,
        payment_type: PaymentType,
        payment_method: PaymentMethod,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            claim_id,
            payee_id,
            amount,
            payment_type,
            payment_method,
            reference: None,
            paid_at: Utc::now(),
        }
    }
}
