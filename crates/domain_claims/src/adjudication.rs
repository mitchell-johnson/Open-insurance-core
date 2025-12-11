//! Claim adjudication logic

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use core_kernel::{ClaimId, Money};

/// Adjudication decision
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdjudicationDecision {
    /// Fully approved
    Approved,
    /// Partially approved
    PartiallyApproved,
    /// Denied
    Denied,
    /// Requires more information
    PendingInformation,
    /// Referred to senior adjuster
    Referred,
}

/// Reason for adjudication decision
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdjudicationReason {
    /// Covered under policy
    Covered,
    /// Not covered - exclusion applies
    Exclusion(String),
    /// Waiting period not satisfied
    WaitingPeriod,
    /// Policy not in force at loss date
    PolicyNotInForce,
    /// Exceeds coverage limit
    ExceedsLimit,
    /// Fraud suspected
    FraudSuspected,
    /// Documentation insufficient
    InsufficientDocumentation,
    /// Pre-existing condition
    PreExistingCondition,
    /// Other
    Other(String),
}

/// Result of claim adjudication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjudicationResult {
    pub claim_id: ClaimId,
    pub decision: AdjudicationDecision,
    pub reasons: Vec<AdjudicationReason>,
    pub approved_amount: Option<Money>,
    pub reduction_percent: Option<Decimal>,
    pub notes: Option<String>,
    pub adjudicator: String,
    pub adjudicated_at: DateTime<Utc>,
}

impl AdjudicationResult {
    /// Creates an approval
    pub fn approve(claim_id: ClaimId, amount: Money, adjudicator: &str) -> Self {
        Self {
            claim_id,
            decision: AdjudicationDecision::Approved,
            reasons: vec![AdjudicationReason::Covered],
            approved_amount: Some(amount),
            reduction_percent: None,
            notes: None,
            adjudicator: adjudicator.to_string(),
            adjudicated_at: Utc::now(),
        }
    }

    /// Creates a denial
    pub fn deny(claim_id: ClaimId, reason: AdjudicationReason, adjudicator: &str) -> Self {
        Self {
            claim_id,
            decision: AdjudicationDecision::Denied,
            reasons: vec![reason],
            approved_amount: None,
            reduction_percent: None,
            notes: None,
            adjudicator: adjudicator.to_string(),
            adjudicated_at: Utc::now(),
        }
    }
}
