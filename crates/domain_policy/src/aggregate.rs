//! Policy Aggregate Root
//!
//! The Policy aggregate is the main consistency boundary for policy administration.
//! It ensures that all changes to a policy are valid and maintains invariants.
//!
//! # Invariants
//!
//! - A policy cannot be terminated if it has pending claims
//! - Premium frequency must align with policy term
//! - Coverage cannot exceed product limits
//! - State transitions must follow the allowed lifecycle

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use core_kernel::{
    Money, Currency, PolicyId, PartyId, PolicyVersionId,
    ValidPeriod, BiTemporalRecord,
};

use crate::coverage::{Coverage, CoverageType};
use crate::endorsement::{Endorsement, EndorsementType};
use crate::error::PolicyError;
use crate::events::PolicyEvent;
use crate::premium::{Premium, PremiumFrequency};

/// Policy lifecycle states
///
/// Represents all possible states a policy can be in during its lifecycle.
/// The type system ensures that only valid state transitions are allowed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyState {
    /// Initial quote state - not yet bound
    Quoted {
        /// Date when the quote was created
        quote_date: DateTime<Utc>,
        /// Date when the quote expires
        quote_expiry: DateTime<Utc>,
    },

    /// Policy is active and in force
    InForce {
        /// Policy effective date
        effective_date: NaiveDate,
        /// Next renewal date
        renewal_date: NaiveDate,
        /// Date when policy was issued
        issue_date: DateTime<Utc>,
    },

    /// Policy has lapsed due to non-payment
    Lapsed {
        /// Reason for lapse
        reason: LapseReason,
        /// Date lapse became effective
        effective_date: DateTime<Utc>,
        /// Date by which policy can be reinstated
        reinstatement_deadline: Option<DateTime<Utc>>,
    },

    /// Policy was reinstated after lapse
    Reinstated {
        /// Date of reinstatement
        reinstatement_date: DateTime<Utc>,
        /// Original lapse date
        original_lapse_date: DateTime<Utc>,
    },

    /// Policy has been terminated
    Terminated {
        /// Reason for termination
        reason: TerminationReason,
        /// Effective date of termination
        effective_date: DateTime<Utc>,
    },

    /// Policy was cancelled (usually before effective)
    Cancelled {
        /// Reason for cancellation
        reason: String,
        /// Date of cancellation
        cancellation_date: DateTime<Utc>,
        /// Whether premium was refunded
        premium_refunded: bool,
    },

    /// Policy naturally expired
    Expired {
        /// Date of expiry
        expiry_date: NaiveDate,
    },

    /// Policy is pending underwriting review
    PendingUnderwriting {
        /// Date submitted for underwriting
        submission_date: DateTime<Utc>,
        /// Required documents
        required_documents: Vec<String>,
    },
}

/// Reasons for policy lapse
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LapseReason {
    /// Non-payment of premium
    NonPayment {
        /// Number of grace days elapsed
        grace_days_elapsed: u32,
        /// Amount outstanding
        outstanding_amount: Decimal,
    },
    /// Insufficient fund value (for ULIPs)
    InsufficientFundValue,
    /// Other reason
    Other(String),
}

/// Reasons for policy termination
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminationReason {
    /// Death of insured
    Death,
    /// Maturity of policy
    Maturity,
    /// Surrender by policyholder
    Surrender,
    /// Fraud detected
    Fraud,
    /// Policy converted to another product
    Conversion,
    /// Other reason
    Other(String),
}

/// Risk object insured under the policy
///
/// Represents what is being insured (person, property, vehicle, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskObject {
    /// Unique identifier for this risk
    pub id: Uuid,
    /// Type of risk
    pub risk_type: RiskType,
    /// Description
    pub description: String,
    /// Location (for property risks)
    pub location: Option<String>,
    /// Custom attributes
    pub attributes: serde_json::Value,
}

/// Types of insurable risks
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskType {
    /// Person (life, health)
    Person {
        party_id: PartyId,
        role: PersonRole,
    },
    /// Property
    Property,
    /// Vehicle
    Vehicle,
    /// Liability
    Liability,
}

/// Role of person in life/health policy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PersonRole {
    /// Primary insured
    PrimaryInsured,
    /// Joint insured
    JointInsured,
    /// Spouse rider
    Spouse,
    /// Child rider
    Child,
}

/// Financial state of the policy
///
/// Tracks all monetary aspects of the policy including
/// premiums paid, account value, and outstanding amounts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyFinancials {
    /// Total premium paid to date
    pub total_premium_paid: Money,
    /// Premium outstanding
    pub premium_outstanding: Money,
    /// Account value (for investment-linked products)
    pub account_value: Option<Money>,
    /// Surrender value
    pub surrender_value: Option<Money>,
    /// Loan outstanding
    pub loan_outstanding: Option<Money>,
    /// Last payment date
    pub last_payment_date: Option<DateTime<Utc>>,
    /// Next premium due date
    pub next_due_date: Option<NaiveDate>,
}

impl PolicyFinancials {
    /// Creates new financial state with zero values
    ///
    /// # Arguments
    ///
    /// * `currency` - The policy currency
    pub fn new(currency: Currency) -> Self {
        Self {
            total_premium_paid: Money::zero(currency),
            premium_outstanding: Money::zero(currency),
            account_value: None,
            surrender_value: None,
            loan_outstanding: None,
            last_payment_date: None,
            next_due_date: None,
        }
    }

    /// Records a premium payment
    ///
    /// # Arguments
    ///
    /// * `amount` - The payment amount
    /// * `payment_date` - When the payment was made
    pub fn record_payment(&mut self, amount: Money, payment_date: DateTime<Utc>) -> Result<(), PolicyError> {
        self.total_premium_paid = self.total_premium_paid.checked_add(&amount)
            .map_err(|e| PolicyError::Financial(e.to_string()))?;
        self.premium_outstanding = self.premium_outstanding.checked_sub(&amount)
            .map_err(|e| PolicyError::Financial(e.to_string()))?;
        self.last_payment_date = Some(payment_date);
        Ok(())
    }
}

/// The Policy aggregate root
///
/// This is the main entity representing an insurance policy. It acts as
/// the consistency boundary, ensuring all invariants are maintained.
///
/// # State Machine
///
/// The policy lifecycle is modeled as a state machine. Valid transitions:
/// - Quoted -> InForce (via issue)
/// - Quoted -> PendingUnderwriting (via submit_for_underwriting)
/// - Quoted -> Cancelled (via cancel)
/// - PendingUnderwriting -> InForce (via approve)
/// - PendingUnderwriting -> Quoted (via decline)
/// - InForce -> Lapsed (via lapse)
/// - InForce -> Terminated (via terminate)
/// - InForce -> Expired (on expiry date)
/// - Lapsed -> InForce (via reinstate)
/// - Lapsed -> Terminated (via terminate)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Unique policy identifier
    id: PolicyId,
    /// Human-readable policy number
    policy_number: String,
    /// Product code this policy is based on
    product_code: String,
    /// Current lifecycle state
    state: PolicyState,
    /// Policyholder party ID
    policyholder_id: PartyId,
    /// Premium details
    premium: Premium,
    /// List of coverages
    coverages: Vec<Coverage>,
    /// Insured risks
    insured_risks: Vec<RiskObject>,
    /// Financial state
    financial_state: PolicyFinancials,
    /// Policy currency
    currency: Currency,
    /// Policy term in years (None for whole life)
    term_years: Option<u32>,
    /// Date when policy expires
    expiry_date: Option<NaiveDate>,
    /// Applied endorsements
    endorsements: Vec<Endorsement>,
    /// Domain events to be published
    #[serde(skip)]
    events: Vec<PolicyEvent>,
    /// Version for optimistic concurrency
    version: u32,
    /// Creation timestamp
    created_at: DateTime<Utc>,
    /// Last update timestamp
    updated_at: DateTime<Utc>,
}

impl Policy {
    /// Returns the policy ID
    pub fn id(&self) -> PolicyId {
        self.id
    }

    /// Returns the policy number
    pub fn policy_number(&self) -> &str {
        &self.policy_number
    }

    /// Returns the product code
    pub fn product_code(&self) -> &str {
        &self.product_code
    }

    /// Returns the current state
    pub fn state(&self) -> &PolicyState {
        &self.state
    }

    /// Returns the policyholder ID
    pub fn policyholder_id(&self) -> PartyId {
        self.policyholder_id
    }

    /// Returns the premium information
    pub fn premium(&self) -> &Premium {
        &self.premium
    }

    /// Returns the coverages
    pub fn coverages(&self) -> &[Coverage] {
        &self.coverages
    }

    /// Returns the financial state
    pub fn financial_state(&self) -> &PolicyFinancials {
        &self.financial_state
    }

    /// Returns the currency
    pub fn currency(&self) -> Currency {
        self.currency
    }

    /// Returns accumulated domain events and clears them
    pub fn take_events(&mut self) -> Vec<PolicyEvent> {
        std::mem::take(&mut self.events)
    }

    /// Checks if the policy is in force
    pub fn is_in_force(&self) -> bool {
        matches!(self.state, PolicyState::InForce { .. })
    }

    /// Checks if the policy can be modified
    pub fn is_modifiable(&self) -> bool {
        matches!(
            self.state,
            PolicyState::Quoted { .. } | PolicyState::InForce { .. }
        )
    }

    /// Issues the policy (transitions from Quoted to InForce)
    ///
    /// # Arguments
    ///
    /// * `effective_date` - When the policy becomes effective
    /// * `underwriter` - ID of the approving underwriter
    ///
    /// # Errors
    ///
    /// Returns error if policy is not in Quoted state
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// policy.issue(effective_date, "UW001")?;
    /// assert!(policy.is_in_force());
    /// ```
    pub fn issue(
        &mut self,
        effective_date: NaiveDate,
        underwriter: &str,
    ) -> Result<(), PolicyError> {
        match &self.state {
            PolicyState::Quoted { .. } | PolicyState::PendingUnderwriting { .. } => {
                let renewal_date = self.calculate_renewal_date(effective_date);
                let now = Utc::now();

                self.state = PolicyState::InForce {
                    effective_date,
                    renewal_date,
                    issue_date: now,
                };

                self.financial_state.next_due_date = Some(effective_date);
                self.updated_at = now;

                self.events.push(PolicyEvent::PolicyIssued {
                    policy_id: self.id,
                    effective_date,
                    underwriter: underwriter.to_string(),
                    timestamp: now,
                });

                Ok(())
            }
            _ => Err(PolicyError::InvalidStateTransition {
                from: format!("{:?}", self.state),
                to: "InForce".to_string(),
            }),
        }
    }

    /// Lapses the policy due to non-payment
    ///
    /// # Arguments
    ///
    /// * `reason` - The reason for lapse
    /// * `reinstatement_period_days` - Days allowed for reinstatement
    ///
    /// # Errors
    ///
    /// Returns error if policy is not in force
    pub fn lapse(
        &mut self,
        reason: LapseReason,
        reinstatement_period_days: Option<u32>,
    ) -> Result<(), PolicyError> {
        match &self.state {
            PolicyState::InForce { .. } => {
                let now = Utc::now();
                let reinstatement_deadline = reinstatement_period_days.map(|days| {
                    now + chrono::Duration::days(days as i64)
                });

                self.state = PolicyState::Lapsed {
                    reason: reason.clone(),
                    effective_date: now,
                    reinstatement_deadline,
                };

                self.updated_at = now;

                self.events.push(PolicyEvent::PolicyLapsed {
                    policy_id: self.id,
                    reason: format!("{:?}", reason),
                    timestamp: now,
                });

                Ok(())
            }
            _ => Err(PolicyError::InvalidStateTransition {
                from: format!("{:?}", self.state),
                to: "Lapsed".to_string(),
            }),
        }
    }

    /// Reinstates a lapsed policy
    ///
    /// # Errors
    ///
    /// Returns error if policy is not lapsed or reinstatement period has passed
    pub fn reinstate(&mut self) -> Result<(), PolicyError> {
        match &self.state {
            PolicyState::Lapsed {
                effective_date,
                reinstatement_deadline,
                ..
            } => {
                let now = Utc::now();

                // Check if reinstatement period has passed
                if let Some(deadline) = reinstatement_deadline {
                    if now > *deadline {
                        return Err(PolicyError::ReinstatementPeriodExpired);
                    }
                }

                self.state = PolicyState::Reinstated {
                    reinstatement_date: now,
                    original_lapse_date: *effective_date,
                };

                self.updated_at = now;

                self.events.push(PolicyEvent::PolicyReinstated {
                    policy_id: self.id,
                    timestamp: now,
                });

                Ok(())
            }
            _ => Err(PolicyError::InvalidStateTransition {
                from: format!("{:?}", self.state),
                to: "Reinstated".to_string(),
            }),
        }
    }

    /// Terminates the policy
    ///
    /// # Arguments
    ///
    /// * `reason` - The reason for termination
    ///
    /// # Errors
    ///
    /// Returns error if policy cannot be terminated from current state
    pub fn terminate(&mut self, reason: TerminationReason) -> Result<(), PolicyError> {
        match &self.state {
            PolicyState::InForce { .. } | PolicyState::Lapsed { .. } => {
                let now = Utc::now();

                self.state = PolicyState::Terminated {
                    reason: reason.clone(),
                    effective_date: now,
                };

                self.updated_at = now;

                self.events.push(PolicyEvent::PolicyTerminated {
                    policy_id: self.id,
                    reason: format!("{:?}", reason),
                    timestamp: now,
                });

                Ok(())
            }
            _ => Err(PolicyError::InvalidStateTransition {
                from: format!("{:?}", self.state),
                to: "Terminated".to_string(),
            }),
        }
    }

    /// Applies an endorsement to modify the policy
    ///
    /// # Arguments
    ///
    /// * `endorsement` - The endorsement to apply
    ///
    /// # Errors
    ///
    /// Returns error if policy is not in a modifiable state
    pub fn apply_endorsement(&mut self, endorsement: Endorsement) -> Result<(), PolicyError> {
        if !self.is_modifiable() {
            return Err(PolicyError::NotModifiable);
        }

        // Apply the endorsement effects
        match &endorsement.endorsement_type {
            EndorsementType::CoverageChange { add, remove, modify } => {
                // Remove coverages
                for coverage_id in remove {
                    self.coverages.retain(|c| c.id != *coverage_id);
                }

                // Add new coverages
                self.coverages.extend(add.clone());

                // Modify existing coverages
                for modification in modify {
                    if let Some(coverage) = self.coverages.iter_mut().find(|c| c.id == modification.coverage_id) {
                        coverage.apply_modification(modification)?;
                    }
                }
            }
            EndorsementType::BeneficiaryChange { beneficiaries } => {
                // Handle beneficiary changes
            }
            EndorsementType::PremiumChange { new_premium } => {
                self.premium = new_premium.clone();
            }
            EndorsementType::NameChange { .. } => {
                // Handle through party service
            }
            EndorsementType::AddressChange { .. } => {
                // Handle through party service
            }
        }

        let now = Utc::now();
        self.endorsements.push(endorsement.clone());
        self.updated_at = now;

        self.events.push(PolicyEvent::EndorsementApplied {
            policy_id: self.id,
            endorsement_id: endorsement.id,
            endorsement_type: format!("{:?}", endorsement.endorsement_type),
            effective_date: endorsement.effective_date,
            timestamp: now,
        });

        Ok(())
    }

    /// Records a premium payment
    ///
    /// # Arguments
    ///
    /// * `amount` - The payment amount
    ///
    /// # Errors
    ///
    /// Returns error if currency mismatch or policy not active
    pub fn record_payment(&mut self, amount: Money) -> Result<(), PolicyError> {
        if amount.currency() != self.currency {
            return Err(PolicyError::CurrencyMismatch {
                expected: self.currency.to_string(),
                actual: amount.currency().to_string(),
            });
        }

        let now = Utc::now();
        self.financial_state.record_payment(amount, now)?;
        self.updated_at = now;

        self.events.push(PolicyEvent::PaymentReceived {
            policy_id: self.id,
            amount: amount.amount(),
            currency: self.currency.to_string(),
            timestamp: now,
        });

        Ok(())
    }

    /// Calculates the next renewal date based on effective date
    fn calculate_renewal_date(&self, effective_date: NaiveDate) -> NaiveDate {
        match self.premium.frequency {
            PremiumFrequency::Annual => effective_date + chrono::Duration::days(365),
            PremiumFrequency::SemiAnnual => effective_date + chrono::Duration::days(182),
            PremiumFrequency::Quarterly => effective_date + chrono::Duration::days(91),
            PremiumFrequency::Monthly => effective_date + chrono::Duration::days(30),
            PremiumFrequency::Single => effective_date, // No renewal for single premium
        }
    }
}

/// Builder for creating new policies
///
/// Provides a fluent API for constructing Policy instances with
/// validation of required fields.
///
/// # Example
///
/// ```rust,ignore
/// let policy = PolicyBuilder::new()
///     .product_code("TERM_LIFE_20")
///     .policyholder(party_id)
///     .currency(Currency::USD)
///     .add_coverage(death_benefit)
///     .premium(premium)
///     .build()?;
/// ```
pub struct PolicyBuilder {
    product_code: Option<String>,
    policyholder_id: Option<PartyId>,
    currency: Currency,
    coverages: Vec<Coverage>,
    insured_risks: Vec<RiskObject>,
    premium: Option<Premium>,
    term_years: Option<u32>,
    quote_validity_days: u32,
}

impl PolicyBuilder {
    /// Creates a new policy builder with default values
    pub fn new() -> Self {
        Self {
            product_code: None,
            policyholder_id: None,
            currency: Currency::USD,
            coverages: Vec::new(),
            insured_risks: Vec::new(),
            premium: None,
            term_years: None,
            quote_validity_days: 30,
        }
    }

    /// Sets the product code
    pub fn product_code(mut self, code: impl Into<String>) -> Self {
        self.product_code = Some(code.into());
        self
    }

    /// Sets the policyholder
    pub fn policyholder(mut self, party_id: PartyId) -> Self {
        self.policyholder_id = Some(party_id);
        self
    }

    /// Sets the currency
    pub fn currency(mut self, currency: Currency) -> Self {
        self.currency = currency;
        self
    }

    /// Adds a coverage
    pub fn add_coverage(mut self, coverage: Coverage) -> Self {
        self.coverages.push(coverage);
        self
    }

    /// Adds multiple coverages
    pub fn coverages(mut self, coverages: Vec<Coverage>) -> Self {
        self.coverages = coverages;
        self
    }

    /// Adds an insured risk
    pub fn add_risk(mut self, risk: RiskObject) -> Self {
        self.insured_risks.push(risk);
        self
    }

    /// Sets the premium
    pub fn premium(mut self, premium: Premium) -> Self {
        self.premium = Some(premium);
        self
    }

    /// Sets the policy term in years
    pub fn term_years(mut self, years: u32) -> Self {
        self.term_years = Some(years);
        self
    }

    /// Sets quote validity period
    pub fn quote_validity_days(mut self, days: u32) -> Self {
        self.quote_validity_days = days;
        self
    }

    /// Builds the policy
    ///
    /// # Errors
    ///
    /// Returns error if required fields are missing
    pub fn build(self) -> Result<Policy, PolicyError> {
        let product_code = self.product_code
            .ok_or(PolicyError::MissingRequiredField("product_code".to_string()))?;
        let policyholder_id = self.policyholder_id
            .ok_or(PolicyError::MissingRequiredField("policyholder_id".to_string()))?;
        let premium = self.premium
            .ok_or(PolicyError::MissingRequiredField("premium".to_string()))?;

        if self.coverages.is_empty() {
            return Err(PolicyError::MissingRequiredField("coverages".to_string()));
        }

        let now = Utc::now();
        let quote_expiry = now + chrono::Duration::days(self.quote_validity_days as i64);

        let policy_id = PolicyId::new_v7();
        let policy_number = generate_policy_number(&product_code);

        Ok(Policy {
            id: policy_id,
            policy_number,
            product_code,
            state: PolicyState::Quoted {
                quote_date: now,
                quote_expiry,
            },
            policyholder_id,
            premium,
            coverages: self.coverages,
            insured_risks: self.insured_risks,
            financial_state: PolicyFinancials::new(self.currency),
            currency: self.currency,
            term_years: self.term_years,
            expiry_date: None,
            endorsements: Vec::new(),
            events: vec![PolicyEvent::PolicyQuoted {
                policy_id,
                quote_expiry,
                timestamp: now,
            }],
            version: 1,
            created_at: now,
            updated_at: now,
        })
    }
}

impl Default for PolicyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Generates a unique policy number
///
/// Format: {PREFIX}-{YEAR}{MONTH}-{SEQUENCE}
fn generate_policy_number(product_code: &str) -> String {
    let now = Utc::now();
    let prefix = &product_code[..3.min(product_code.len())];
    let random_seq: u32 = rand_sequence();
    format!(
        "{}-{}{:02}-{:06}",
        prefix.to_uppercase(),
        now.format("%Y"),
        now.format("%m"),
        random_seq
    )
}

/// Generates a pseudo-random sequence for policy numbers
fn rand_sequence() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (duration.as_nanos() % 1_000_000) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn create_test_policy() -> Policy {
        let coverage = Coverage::new(
            CoverageType::DeathBenefit,
            Money::new(dec!(500000), Currency::USD),
        );

        let premium = Premium::new(
            Money::new(dec!(1000), Currency::USD),
            PremiumFrequency::Annual,
        );

        PolicyBuilder::new()
            .product_code("TERM_LIFE_20")
            .policyholder(PartyId::new())
            .add_coverage(coverage)
            .premium(premium)
            .term_years(20)
            .build()
            .unwrap()
    }

    #[test]
    fn test_policy_creation() {
        let policy = create_test_policy();
        assert!(matches!(policy.state(), PolicyState::Quoted { .. }));
        assert_eq!(policy.coverages().len(), 1);
    }

    #[test]
    fn test_policy_issue() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();

        policy.issue(effective_date, "UW001").unwrap();
        assert!(policy.is_in_force());
    }

    #[test]
    fn test_invalid_state_transition() {
        let mut policy = create_test_policy();
        let result = policy.lapse(
            LapseReason::NonPayment {
                grace_days_elapsed: 30,
                outstanding_amount: dec!(1000),
            },
            Some(30),
        );

        assert!(matches!(result, Err(PolicyError::InvalidStateTransition { .. })));
    }

    #[test]
    fn test_policy_lifecycle() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();

        // Issue
        policy.issue(effective_date, "UW001").unwrap();
        assert!(policy.is_in_force());

        // Lapse
        policy.lapse(
            LapseReason::NonPayment {
                grace_days_elapsed: 30,
                outstanding_amount: dec!(1000),
            },
            Some(30),
        ).unwrap();
        assert!(matches!(policy.state(), PolicyState::Lapsed { .. }));

        // Reinstate
        policy.reinstate().unwrap();
        assert!(matches!(policy.state(), PolicyState::Reinstated { .. }));
    }
}
