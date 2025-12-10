//! Policy endorsements and amendments
//!
//! This module handles policy modifications through endorsements,
//! which are the formal mechanism for changing policy terms.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use core_kernel::{EndorsementId, PartyId};
use crate::coverage::{Coverage, CoverageModification};
use crate::premium::Premium;

/// Types of endorsements that can be applied to a policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EndorsementType {
    /// Change in coverage (add, remove, modify)
    CoverageChange {
        /// Coverages to add
        add: Vec<Coverage>,
        /// Coverage IDs to remove
        remove: Vec<Uuid>,
        /// Modifications to existing coverages
        modify: Vec<CoverageModification>,
    },

    /// Change in beneficiaries
    BeneficiaryChange {
        /// New beneficiary assignments
        beneficiaries: Vec<BeneficiaryAssignment>,
    },

    /// Change in premium
    PremiumChange {
        /// New premium structure
        new_premium: Premium,
    },

    /// Change in policyholder name
    NameChange {
        /// New first name
        new_first_name: Option<String>,
        /// New last name
        new_last_name: Option<String>,
        /// Reason for change
        reason: String,
    },

    /// Change in address
    AddressChange {
        /// New address
        new_address: Address,
    },

    /// Change in sum assured
    SumAssuredChange {
        /// Coverage ID
        coverage_id: Uuid,
        /// New sum assured amount
        new_amount: rust_decimal::Decimal,
        /// Currency
        currency: String,
    },

    /// Policy loan
    PolicyLoan {
        /// Loan amount
        amount: rust_decimal::Decimal,
        /// Currency
        currency: String,
    },

    /// Partial withdrawal (for ULIPs)
    PartialWithdrawal {
        /// Withdrawal amount
        amount: rust_decimal::Decimal,
        /// Currency
        currency: String,
    },

    /// Fund switch (for ULIPs)
    FundSwitch {
        /// Switches to perform
        switches: Vec<FundSwitchInstruction>,
    },

    /// Premium redirection (for ULIPs)
    PremiumRedirection {
        /// New fund allocation
        allocations: Vec<FundAllocation>,
    },

    /// Free-form endorsement
    Custom {
        /// Endorsement code
        code: String,
        /// Description
        description: String,
        /// Custom data
        data: serde_json::Value,
    },
}

/// Beneficiary assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeneficiaryAssignment {
    /// Party ID of beneficiary
    pub party_id: PartyId,
    /// Relationship to insured
    pub relationship: String,
    /// Percentage share
    pub share_percent: rust_decimal::Decimal,
    /// Beneficiary type (primary, contingent)
    pub beneficiary_type: BeneficiaryType,
    /// Whether this is a revocable designation
    pub is_revocable: bool,
}

/// Type of beneficiary designation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeneficiaryType {
    /// Primary beneficiary
    Primary,
    /// Contingent/secondary beneficiary
    Contingent,
}

/// Address structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    /// Address line 1
    pub line1: String,
    /// Address line 2
    pub line2: Option<String>,
    /// City
    pub city: String,
    /// State/province
    pub state: Option<String>,
    /// Postal code
    pub postal_code: String,
    /// Country code (ISO 3166)
    pub country: String,
}

/// Fund switch instruction (for ULIPs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundSwitchInstruction {
    /// Source fund ID
    pub from_fund_id: Uuid,
    /// Target fund ID
    pub to_fund_id: Uuid,
    /// Percentage to switch
    pub percentage: rust_decimal::Decimal,
}

/// Fund allocation (for premium redirection)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundAllocation {
    /// Fund ID
    pub fund_id: Uuid,
    /// Allocation percentage
    pub percentage: rust_decimal::Decimal,
}

/// An endorsement to a policy
///
/// Endorsements are the formal mechanism for making changes to a policy.
/// They maintain an audit trail and can have effective dates in the past
/// (retroactive) or future (prospective).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endorsement {
    /// Unique endorsement identifier
    pub id: EndorsementId,
    /// Endorsement number (human-readable)
    pub endorsement_number: String,
    /// Type of endorsement
    pub endorsement_type: EndorsementType,
    /// Effective date of the change
    pub effective_date: NaiveDate,
    /// Reason for the endorsement
    pub reason: Option<String>,
    /// Premium adjustment (positive = increase, negative = refund)
    pub premium_adjustment: Option<rust_decimal::Decimal>,
    /// Status of the endorsement
    pub status: EndorsementStatus,
    /// Who requested the endorsement
    pub requested_by: Option<String>,
    /// Who approved the endorsement
    pub approved_by: Option<String>,
    /// When the endorsement was created
    pub created_at: DateTime<Utc>,
    /// When the endorsement was processed
    pub processed_at: Option<DateTime<Utc>>,
}

/// Status of an endorsement
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndorsementStatus {
    /// Pending approval
    Pending,
    /// Approved and ready to apply
    Approved,
    /// Applied to the policy
    Applied,
    /// Rejected
    Rejected,
    /// Cancelled
    Cancelled,
}

impl Endorsement {
    /// Creates a new endorsement
    ///
    /// # Arguments
    ///
    /// * `endorsement_type` - The type of change
    /// * `effective_date` - When the change takes effect
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let endorsement = Endorsement::new(
    ///     EndorsementType::BeneficiaryChange { beneficiaries: vec![...] },
    ///     effective_date,
    /// );
    /// ```
    pub fn new(endorsement_type: EndorsementType, effective_date: NaiveDate) -> Self {
        let id = EndorsementId::new_v7();
        let now = Utc::now();

        Self {
            id,
            endorsement_number: generate_endorsement_number(),
            endorsement_type,
            effective_date,
            reason: None,
            premium_adjustment: None,
            status: EndorsementStatus::Pending,
            requested_by: None,
            approved_by: None,
            created_at: now,
            processed_at: None,
        }
    }

    /// Sets the reason for this endorsement
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Sets the premium adjustment
    pub fn with_premium_adjustment(mut self, adjustment: rust_decimal::Decimal) -> Self {
        self.premium_adjustment = Some(adjustment);
        self
    }

    /// Sets who requested the endorsement
    pub fn requested_by(mut self, user: impl Into<String>) -> Self {
        self.requested_by = Some(user.into());
        self
    }

    /// Approves the endorsement
    ///
    /// # Arguments
    ///
    /// * `approver` - ID of the approving user
    pub fn approve(&mut self, approver: &str) {
        self.status = EndorsementStatus::Approved;
        self.approved_by = Some(approver.to_string());
    }

    /// Rejects the endorsement
    ///
    /// # Arguments
    ///
    /// * `reason` - Reason for rejection
    pub fn reject(&mut self, reason: &str) {
        self.status = EndorsementStatus::Rejected;
        self.reason = Some(reason.to_string());
    }

    /// Marks the endorsement as applied
    pub fn mark_applied(&mut self) {
        self.status = EndorsementStatus::Applied;
        self.processed_at = Some(Utc::now());
    }

    /// Checks if this is a retroactive endorsement
    pub fn is_retroactive(&self) -> bool {
        let today = Utc::now().date_naive();
        self.effective_date < today
    }

    /// Checks if this endorsement requires additional premium
    pub fn requires_additional_premium(&self) -> bool {
        self.premium_adjustment.map_or(false, |adj| adj.is_sign_positive())
    }
}

/// Generates a unique endorsement number
fn generate_endorsement_number() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("END-{}", duration.as_millis() % 10_000_000_000)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_endorsement_creation() {
        let endorsement = Endorsement::new(
            EndorsementType::BeneficiaryChange {
                beneficiaries: vec![],
            },
            NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
        );

        assert_eq!(endorsement.status, EndorsementStatus::Pending);
    }

    #[test]
    fn test_endorsement_approval() {
        let mut endorsement = Endorsement::new(
            EndorsementType::NameChange {
                new_first_name: Some("John".to_string()),
                new_last_name: None,
                reason: "Marriage".to_string(),
            },
            NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
        );

        endorsement.approve("ADMIN001");
        assert_eq!(endorsement.status, EndorsementStatus::Approved);
        assert_eq!(endorsement.approved_by, Some("ADMIN001".to_string()));
    }
}
