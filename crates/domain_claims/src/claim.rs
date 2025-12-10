//! Claim aggregate

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use core_kernel::{ClaimId, PolicyId, PartyId, Money, Currency};
use crate::reserve::Reserve;
use crate::payment::ClaimPayment;
use crate::error::ClaimError;

/// Claim status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClaimStatus {
    /// First Notice of Loss received
    Fnol,
    /// Under investigation
    UnderInvestigation,
    /// Pending documentation
    PendingDocumentation,
    /// Under review/adjudication
    UnderReview,
    /// Approved for payment
    Approved,
    /// Partially approved
    PartiallyApproved,
    /// Denied
    Denied,
    /// Paid and closed
    Closed,
    /// Withdrawn by claimant
    Withdrawn,
    /// Reopened
    Reopened,
}

/// Type of loss
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LossType {
    Death,
    Disability,
    CriticalIllness,
    Hospitalization,
    Accident,
    Property,
    Liability,
    Other,
}

/// A claim against a policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    /// Unique identifier
    pub id: ClaimId,
    /// Claim number
    pub claim_number: String,
    /// Policy ID
    pub policy_id: PolicyId,
    /// Claimant ID
    pub claimant_id: PartyId,
    /// Status
    pub status: ClaimStatus,
    /// Date of loss
    pub loss_date: NaiveDate,
    /// Date claim was reported
    pub notification_date: DateTime<Utc>,
    /// Type of loss
    pub loss_type: LossType,
    /// Description
    pub description: Option<String>,
    /// Location
    pub location: Option<String>,
    /// Claimed amount
    pub claimed_amount: Option<Money>,
    /// Approved amount
    pub approved_amount: Option<Money>,
    /// Total paid
    pub paid_amount: Money,
    /// Currency
    pub currency: Currency,
    /// Reserves
    pub reserves: Vec<Reserve>,
    /// Payments
    pub payments: Vec<ClaimPayment>,
    /// Assigned adjuster
    pub assigned_to: Option<String>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl Claim {
    /// Creates a new FNOL claim
    pub fn fnol(
        policy_id: PolicyId,
        claimant_id: PartyId,
        loss_date: NaiveDate,
        loss_type: LossType,
        currency: Currency,
    ) -> Self {
        let now = Utc::now();
        let id = ClaimId::new_v7();

        Self {
            id,
            claim_number: generate_claim_number(),
            policy_id,
            claimant_id,
            status: ClaimStatus::Fnol,
            loss_date,
            notification_date: now,
            loss_type,
            description: None,
            location: None,
            claimed_amount: None,
            approved_amount: None,
            paid_amount: Money::zero(currency),
            currency,
            reserves: Vec::new(),
            payments: Vec::new(),
            assigned_to: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Updates the status
    pub fn update_status(&mut self, status: ClaimStatus) -> Result<(), ClaimError> {
        // Validate transition
        if !self.can_transition_to(status) {
            return Err(ClaimError::InvalidStatusTransition {
                from: format!("{:?}", self.status),
                to: format!("{:?}", status),
            });
        }
        self.status = status;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Adds a reserve
    pub fn add_reserve(&mut self, reserve: Reserve) {
        self.reserves.push(reserve);
        self.updated_at = Utc::now();
    }

    /// Records a payment
    pub fn add_payment(&mut self, payment: ClaimPayment) {
        self.paid_amount = self.paid_amount + payment.amount;
        self.payments.push(payment);
        self.updated_at = Utc::now();
    }

    /// Gets total reserved amount
    pub fn total_reserve(&self) -> Money {
        self.reserves
            .iter()
            .fold(Money::zero(self.currency), |acc, r| acc + r.amount)
    }

    /// Checks if transition is valid
    fn can_transition_to(&self, target: ClaimStatus) -> bool {
        use ClaimStatus::*;
        matches!(
            (self.status, target),
            (Fnol, UnderInvestigation) |
            (Fnol, PendingDocumentation) |
            (UnderInvestigation, UnderReview) |
            (UnderInvestigation, PendingDocumentation) |
            (PendingDocumentation, UnderInvestigation) |
            (PendingDocumentation, UnderReview) |
            (UnderReview, Approved) |
            (UnderReview, PartiallyApproved) |
            (UnderReview, Denied) |
            (Approved, Closed) |
            (PartiallyApproved, Closed) |
            (Denied, Closed) |
            (Closed, Reopened) |
            (Reopened, UnderReview) |
            (_, Withdrawn)
        )
    }
}

fn generate_claim_number() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("CLM-{}", duration.as_millis() % 10_000_000_000)
}
