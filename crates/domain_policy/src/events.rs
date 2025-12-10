//! Domain events for the policy aggregate
//!
//! Domain events represent significant occurrences within the policy lifecycle.
//! They are used for:
//! - Audit trails
//! - Event-driven integrations
//! - Triggering downstream processes

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use core_kernel::{EndorsementId, PolicyId};

/// Domain events emitted by the Policy aggregate
///
/// These events capture all significant state changes and business
/// events that occur during a policy's lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyEvent {
    /// Policy has been quoted
    PolicyQuoted {
        policy_id: PolicyId,
        quote_expiry: DateTime<Utc>,
        timestamp: DateTime<Utc>,
    },

    /// Policy has been issued (moved to InForce)
    PolicyIssued {
        policy_id: PolicyId,
        effective_date: NaiveDate,
        underwriter: String,
        timestamp: DateTime<Utc>,
    },

    /// Policy has lapsed
    PolicyLapsed {
        policy_id: PolicyId,
        reason: String,
        timestamp: DateTime<Utc>,
    },

    /// Policy has been reinstated
    PolicyReinstated {
        policy_id: PolicyId,
        timestamp: DateTime<Utc>,
    },

    /// Policy has been terminated
    PolicyTerminated {
        policy_id: PolicyId,
        reason: String,
        timestamp: DateTime<Utc>,
    },

    /// Policy has been cancelled
    PolicyCancelled {
        policy_id: PolicyId,
        reason: String,
        refund_amount: Option<Decimal>,
        timestamp: DateTime<Utc>,
    },

    /// Policy has expired
    PolicyExpired {
        policy_id: PolicyId,
        expiry_date: NaiveDate,
        timestamp: DateTime<Utc>,
    },

    /// Policy has been renewed
    PolicyRenewed {
        policy_id: PolicyId,
        new_effective_date: NaiveDate,
        new_expiry_date: NaiveDate,
        timestamp: DateTime<Utc>,
    },

    /// An endorsement has been applied
    EndorsementApplied {
        policy_id: PolicyId,
        endorsement_id: EndorsementId,
        endorsement_type: String,
        effective_date: NaiveDate,
        timestamp: DateTime<Utc>,
    },

    /// Premium payment received
    PaymentReceived {
        policy_id: PolicyId,
        amount: Decimal,
        currency: String,
        timestamp: DateTime<Utc>,
    },

    /// Premium is due
    PremiumDue {
        policy_id: PolicyId,
        amount: Decimal,
        currency: String,
        due_date: NaiveDate,
        timestamp: DateTime<Utc>,
    },

    /// Premium is overdue
    PremiumOverdue {
        policy_id: PolicyId,
        amount: Decimal,
        currency: String,
        days_overdue: u32,
        timestamp: DateTime<Utc>,
    },

    /// Policy entered grace period
    GracePeriodStarted {
        policy_id: PolicyId,
        grace_end_date: NaiveDate,
        timestamp: DateTime<Utc>,
    },

    /// Policy submitted for underwriting
    SubmittedForUnderwriting {
        policy_id: PolicyId,
        timestamp: DateTime<Utc>,
    },

    /// Underwriting decision made
    UnderwritingDecision {
        policy_id: PolicyId,
        decision: UnderwritingDecisionType,
        underwriter: String,
        notes: Option<String>,
        timestamp: DateTime<Utc>,
    },

    /// Beneficiary changed
    BeneficiaryChanged {
        policy_id: PolicyId,
        endorsement_id: EndorsementId,
        timestamp: DateTime<Utc>,
    },

    /// Policy loan taken
    PolicyLoanTaken {
        policy_id: PolicyId,
        amount: Decimal,
        currency: String,
        timestamp: DateTime<Utc>,
    },

    /// Policy loan repaid
    PolicyLoanRepaid {
        policy_id: PolicyId,
        amount: Decimal,
        currency: String,
        timestamp: DateTime<Utc>,
    },
}

/// Types of underwriting decisions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnderwritingDecisionType {
    /// Standard approval
    Approved,
    /// Approved with conditions
    ApprovedWithConditions,
    /// Approved with rating (extra premium)
    ApprovedWithRating,
    /// Declined
    Declined,
    /// Postponed
    Postponed,
    /// Referred to higher authority
    Referred,
}

impl PolicyEvent {
    /// Returns the policy ID associated with this event
    pub fn policy_id(&self) -> PolicyId {
        match self {
            PolicyEvent::PolicyQuoted { policy_id, .. } => *policy_id,
            PolicyEvent::PolicyIssued { policy_id, .. } => *policy_id,
            PolicyEvent::PolicyLapsed { policy_id, .. } => *policy_id,
            PolicyEvent::PolicyReinstated { policy_id, .. } => *policy_id,
            PolicyEvent::PolicyTerminated { policy_id, .. } => *policy_id,
            PolicyEvent::PolicyCancelled { policy_id, .. } => *policy_id,
            PolicyEvent::PolicyExpired { policy_id, .. } => *policy_id,
            PolicyEvent::PolicyRenewed { policy_id, .. } => *policy_id,
            PolicyEvent::EndorsementApplied { policy_id, .. } => *policy_id,
            PolicyEvent::PaymentReceived { policy_id, .. } => *policy_id,
            PolicyEvent::PremiumDue { policy_id, .. } => *policy_id,
            PolicyEvent::PremiumOverdue { policy_id, .. } => *policy_id,
            PolicyEvent::GracePeriodStarted { policy_id, .. } => *policy_id,
            PolicyEvent::SubmittedForUnderwriting { policy_id, .. } => *policy_id,
            PolicyEvent::UnderwritingDecision { policy_id, .. } => *policy_id,
            PolicyEvent::BeneficiaryChanged { policy_id, .. } => *policy_id,
            PolicyEvent::PolicyLoanTaken { policy_id, .. } => *policy_id,
            PolicyEvent::PolicyLoanRepaid { policy_id, .. } => *policy_id,
        }
    }

    /// Returns the timestamp of this event
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            PolicyEvent::PolicyQuoted { timestamp, .. } => *timestamp,
            PolicyEvent::PolicyIssued { timestamp, .. } => *timestamp,
            PolicyEvent::PolicyLapsed { timestamp, .. } => *timestamp,
            PolicyEvent::PolicyReinstated { timestamp, .. } => *timestamp,
            PolicyEvent::PolicyTerminated { timestamp, .. } => *timestamp,
            PolicyEvent::PolicyCancelled { timestamp, .. } => *timestamp,
            PolicyEvent::PolicyExpired { timestamp, .. } => *timestamp,
            PolicyEvent::PolicyRenewed { timestamp, .. } => *timestamp,
            PolicyEvent::EndorsementApplied { timestamp, .. } => *timestamp,
            PolicyEvent::PaymentReceived { timestamp, .. } => *timestamp,
            PolicyEvent::PremiumDue { timestamp, .. } => *timestamp,
            PolicyEvent::PremiumOverdue { timestamp, .. } => *timestamp,
            PolicyEvent::GracePeriodStarted { timestamp, .. } => *timestamp,
            PolicyEvent::SubmittedForUnderwriting { timestamp, .. } => *timestamp,
            PolicyEvent::UnderwritingDecision { timestamp, .. } => *timestamp,
            PolicyEvent::BeneficiaryChanged { timestamp, .. } => *timestamp,
            PolicyEvent::PolicyLoanTaken { timestamp, .. } => *timestamp,
            PolicyEvent::PolicyLoanRepaid { timestamp, .. } => *timestamp,
        }
    }

    /// Returns the event type name
    pub fn event_type(&self) -> &'static str {
        match self {
            PolicyEvent::PolicyQuoted { .. } => "PolicyQuoted",
            PolicyEvent::PolicyIssued { .. } => "PolicyIssued",
            PolicyEvent::PolicyLapsed { .. } => "PolicyLapsed",
            PolicyEvent::PolicyReinstated { .. } => "PolicyReinstated",
            PolicyEvent::PolicyTerminated { .. } => "PolicyTerminated",
            PolicyEvent::PolicyCancelled { .. } => "PolicyCancelled",
            PolicyEvent::PolicyExpired { .. } => "PolicyExpired",
            PolicyEvent::PolicyRenewed { .. } => "PolicyRenewed",
            PolicyEvent::EndorsementApplied { .. } => "EndorsementApplied",
            PolicyEvent::PaymentReceived { .. } => "PaymentReceived",
            PolicyEvent::PremiumDue { .. } => "PremiumDue",
            PolicyEvent::PremiumOverdue { .. } => "PremiumOverdue",
            PolicyEvent::GracePeriodStarted { .. } => "GracePeriodStarted",
            PolicyEvent::SubmittedForUnderwriting { .. } => "SubmittedForUnderwriting",
            PolicyEvent::UnderwritingDecision { .. } => "UnderwritingDecision",
            PolicyEvent::BeneficiaryChanged { .. } => "BeneficiaryChanged",
            PolicyEvent::PolicyLoanTaken { .. } => "PolicyLoanTaken",
            PolicyEvent::PolicyLoanRepaid { .. } => "PolicyLoanRepaid",
        }
    }
}
