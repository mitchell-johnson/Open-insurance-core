//! Claim reserves

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use core_kernel::{ClaimId, Money};

/// Reserve type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReserveType {
    /// Initial case reserve
    CaseReserve,
    /// Incurred But Not Reported
    Ibnr,
    /// Legal expenses
    LegalExpense,
    /// Additional expenses
    Expense,
}

/// A claim reserve
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reserve {
    pub id: Uuid,
    pub claim_id: ClaimId,
    pub reserve_type: ReserveType,
    pub amount: Money,
    pub reason: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Reserve {
    /// Creates a new reserve
    pub fn new(claim_id: ClaimId, reserve_type: ReserveType, amount: Money) -> Self {
        Self {
            id: Uuid::new_v4(),
            claim_id,
            reserve_type,
            amount,
            reason: None,
            created_by: None,
            created_at: Utc::now(),
        }
    }
}
