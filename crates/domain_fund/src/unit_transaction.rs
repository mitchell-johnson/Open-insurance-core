//! Unit transactions

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use core_kernel::{FundId, PolicyId};

/// Types of unit transactions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionType {
    /// Premium allocation
    Allocation,
    /// Redemption/withdrawal
    Redemption,
    /// Fund switch in
    SwitchIn,
    /// Fund switch out
    SwitchOut,
    /// Mortality charge deduction
    MortalityCharge,
    /// Policy fee deduction
    PolicyFee,
    /// Management fee deduction
    ManagementFee,
    /// Bonus/loyalty units
    Bonus,
}

/// A unit transaction record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitTransaction {
    /// Unique identifier
    pub id: Uuid,
    /// Policy ID
    pub policy_id: PolicyId,
    /// Fund ID
    pub fund_id: FundId,
    /// Transaction type
    pub transaction_type: TransactionType,
    /// Number of units (positive for in, negative for out)
    pub units: Decimal,
    /// NAV at transaction
    pub nav: Decimal,
    /// Monetary value
    pub value: Decimal,
    /// Transaction date
    pub transaction_date: DateTime<Utc>,
    /// Reference (e.g., premium payment ID)
    pub reference: Option<String>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

impl UnitTransaction {
    /// Creates a new unit transaction
    pub fn new(
        policy_id: PolicyId,
        fund_id: FundId,
        transaction_type: TransactionType,
        units: Decimal,
        nav: Decimal,
    ) -> Self {
        let value = units * nav;
        let now = Utc::now();

        Self {
            id: Uuid::new_v4(),
            policy_id,
            fund_id,
            transaction_type,
            units,
            nav,
            value,
            transaction_date: now,
            reference: None,
            created_at: now,
        }
    }

    /// Sets the reference
    pub fn with_reference(mut self, reference: impl Into<String>) -> Self {
        self.reference = Some(reference.into());
        self
    }
}
