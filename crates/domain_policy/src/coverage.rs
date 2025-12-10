//! Coverage types and value objects
//!
//! This module defines the various types of coverages that can be
//! included in an insurance policy.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use core_kernel::Money;
use crate::error::PolicyError;

/// Types of insurance coverage
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoverageType {
    /// Life insurance death benefit
    DeathBenefit,
    /// Accidental death benefit (ADD)
    AccidentalDeath,
    /// Total and permanent disability
    TotalPermanentDisability,
    /// Critical illness coverage
    CriticalIllness,
    /// Hospitalization coverage
    Hospitalization,
    /// Waiver of premium on disability
    WaiverOfPremium,
    /// Term rider
    TermRider,
    /// Whole life rider
    WholeLifeRider,
    /// Income benefit
    IncomeBenefit,
    /// Maturity benefit
    MaturityBenefit,
    /// Custom/product-specific coverage
    Custom(String),
}

/// A coverage benefit specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Benefit {
    /// Benefit type
    pub benefit_type: BenefitType,
    /// Benefit amount or formula
    pub amount: BenefitAmount,
    /// Waiting period in days (if applicable)
    pub waiting_period_days: Option<u32>,
    /// Maximum claim period (if applicable)
    pub max_claim_period_days: Option<u32>,
}

/// Types of benefits
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BenefitType {
    /// Lump sum payment
    LumpSum,
    /// Regular income payments
    Income,
    /// Reimbursement up to limit
    Reimbursement,
    /// Per-day benefit
    PerDiem,
}

/// Benefit amount specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BenefitAmount {
    /// Fixed monetary amount
    Fixed(Money),
    /// Percentage of sum assured
    PercentageOfSumAssured(Decimal),
    /// Multiple of annual premium
    MultipleOfPremium(Decimal),
    /// Formula-based
    Formula(String),
}

/// Modification to be applied to a coverage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageModification {
    /// ID of coverage to modify
    pub coverage_id: Uuid,
    /// New sum assured (if changing)
    pub new_sum_assured: Option<Money>,
    /// New benefits (if changing)
    pub new_benefits: Option<Vec<Benefit>>,
    /// New exclusions (if changing)
    pub new_exclusions: Option<Vec<Exclusion>>,
}

/// Exclusion from coverage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exclusion {
    /// Exclusion code
    pub code: String,
    /// Description
    pub description: String,
    /// Whether this is a standard or special exclusion
    pub exclusion_type: ExclusionType,
    /// Effective date of exclusion
    pub effective_date: Option<NaiveDate>,
}

/// Types of exclusions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExclusionType {
    /// Standard product exclusion
    Standard,
    /// Special exclusion due to underwriting
    Underwriting,
    /// Regulatory exclusion
    Regulatory,
}

/// A coverage specification for a policy
///
/// Represents a single coverage component of an insurance policy,
/// defining what is covered, for how much, and any exclusions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coverage {
    /// Unique identifier for this coverage
    pub id: Uuid,
    /// Type of coverage
    pub coverage_type: CoverageType,
    /// Sum assured / coverage limit
    pub sum_assured: Money,
    /// Coverage benefits
    pub benefits: Vec<Benefit>,
    /// Exclusions
    pub exclusions: Vec<Exclusion>,
    /// Whether coverage is active
    pub is_active: bool,
    /// Effective date
    pub effective_date: Option<NaiveDate>,
    /// Expiry date
    pub expiry_date: Option<NaiveDate>,
    /// Additional loading percentage (if any)
    pub loading_percent: Option<Decimal>,
}

impl Coverage {
    /// Creates a new coverage with basic settings
    ///
    /// # Arguments
    ///
    /// * `coverage_type` - The type of coverage
    /// * `sum_assured` - The sum assured/coverage limit
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let coverage = Coverage::new(
    ///     CoverageType::DeathBenefit,
    ///     Money::new(dec!(500000), Currency::USD),
    /// );
    /// ```
    pub fn new(coverage_type: CoverageType, sum_assured: Money) -> Self {
        Self {
            id: Uuid::new_v4(),
            coverage_type,
            sum_assured,
            benefits: vec![Benefit {
                benefit_type: BenefitType::LumpSum,
                amount: BenefitAmount::Fixed(sum_assured),
                waiting_period_days: None,
                max_claim_period_days: None,
            }],
            exclusions: Vec::new(),
            is_active: true,
            effective_date: None,
            expiry_date: None,
            loading_percent: None,
        }
    }

    /// Creates a death benefit coverage
    ///
    /// # Arguments
    ///
    /// * `sum_assured` - The death benefit amount
    pub fn death_benefit(sum_assured: Money) -> Self {
        Self::new(CoverageType::DeathBenefit, sum_assured)
    }

    /// Creates a critical illness coverage
    ///
    /// # Arguments
    ///
    /// * `sum_assured` - The coverage amount
    /// * `waiting_period_days` - Waiting period before coverage is active
    pub fn critical_illness(sum_assured: Money, waiting_period_days: u32) -> Self {
        let mut coverage = Self::new(CoverageType::CriticalIllness, sum_assured);
        coverage.benefits[0].waiting_period_days = Some(waiting_period_days);
        coverage
    }

    /// Creates a hospitalization coverage
    ///
    /// # Arguments
    ///
    /// * `per_day_benefit` - Daily hospitalization benefit
    /// * `max_days` - Maximum number of days covered
    pub fn hospitalization(per_day_benefit: Money, max_days: u32) -> Self {
        let mut coverage = Self::new(CoverageType::Hospitalization, per_day_benefit);
        coverage.benefits[0].benefit_type = BenefitType::PerDiem;
        coverage.benefits[0].max_claim_period_days = Some(max_days);
        coverage
    }

    /// Adds an exclusion to this coverage
    ///
    /// # Arguments
    ///
    /// * `exclusion` - The exclusion to add
    pub fn add_exclusion(&mut self, exclusion: Exclusion) {
        self.exclusions.push(exclusion);
    }

    /// Adds a loading to this coverage
    ///
    /// # Arguments
    ///
    /// * `loading_percent` - The additional loading percentage
    pub fn with_loading(mut self, loading_percent: Decimal) -> Self {
        self.loading_percent = Some(loading_percent);
        self
    }

    /// Checks if a claim type is covered
    ///
    /// # Arguments
    ///
    /// * `claim_type` - The type of claim being checked
    pub fn is_covered(&self, claim_type: &CoverageType) -> bool {
        self.is_active && self.coverage_type == *claim_type
    }

    /// Applies a modification to this coverage
    ///
    /// # Arguments
    ///
    /// * `modification` - The modification to apply
    ///
    /// # Errors
    ///
    /// Returns error if modification is invalid
    pub fn apply_modification(&mut self, modification: &CoverageModification) -> Result<(), PolicyError> {
        if let Some(new_sum) = &modification.new_sum_assured {
            self.sum_assured = *new_sum;
        }
        if let Some(new_benefits) = &modification.new_benefits {
            self.benefits = new_benefits.clone();
        }
        if let Some(new_exclusions) = &modification.new_exclusions {
            self.exclusions = new_exclusions.clone();
        }
        Ok(())
    }

    /// Calculates the effective sum assured after loading
    pub fn effective_sum_assured(&self) -> Money {
        match self.loading_percent {
            Some(loading) => {
                // Loading reduces the benefit, not increases it
                // So we don't apply loading to sum assured directly
                self.sum_assured
            }
            None => self.sum_assured,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_kernel::Currency;
    use rust_decimal_macros::dec;

    #[test]
    fn test_coverage_creation() {
        let coverage = Coverage::death_benefit(
            Money::new(dec!(500000), Currency::USD)
        );

        assert_eq!(coverage.coverage_type, CoverageType::DeathBenefit);
        assert!(coverage.is_active);
    }

    #[test]
    fn test_coverage_with_exclusion() {
        let mut coverage = Coverage::critical_illness(
            Money::new(dec!(100000), Currency::USD),
            90,
        );

        coverage.add_exclusion(Exclusion {
            code: "PRE001".to_string(),
            description: "Pre-existing conditions".to_string(),
            exclusion_type: ExclusionType::Standard,
            effective_date: None,
        });

        assert_eq!(coverage.exclusions.len(), 1);
    }
}
