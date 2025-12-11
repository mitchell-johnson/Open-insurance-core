//! Policy domain services
//!
//! This module contains domain services that orchestrate complex operations
//! involving multiple aggregates or external systems.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde_json::Value;

use core_kernel::Money;
use crate::coverage::Coverage;
use crate::error::PolicyError;
use crate::premium::Premium;
use crate::underwriting::{
    UnderwritingApplication, UnderwritingDecision, RiskClass,
    evaluate_basic_rules, determine_risk_class, RuleImpact,
};

/// Service for underwriting policy applications
///
/// The UnderwritingService evaluates applications against product rules
/// and underwriting guidelines to determine insurability and pricing.
pub struct UnderwritingService {
    /// Product rules (would be loaded from zen-engine in production)
    product_rules: Option<Value>,
}

impl UnderwritingService {
    /// Creates a new underwriting service
    pub fn new() -> Self {
        Self {
            product_rules: None,
        }
    }

    /// Loads product-specific rules
    ///
    /// # Arguments
    ///
    /// * `rules` - JSON rules document (JDM format for zen-engine)
    pub fn with_rules(mut self, rules: Value) -> Self {
        self.product_rules = Some(rules);
        self
    }

    /// Evaluates an application and returns an underwriting decision
    ///
    /// This method:
    /// 1. Validates the application
    /// 2. Runs basic underwriting rules
    /// 3. Applies product-specific rules (if loaded)
    /// 4. Determines risk classification
    /// 5. Calculates any required loadings or exclusions
    ///
    /// # Arguments
    ///
    /// * `application` - The underwriting application to evaluate
    ///
    /// # Returns
    ///
    /// An underwriting decision with risk class, exclusions, and any modifications
    ///
    /// # Errors
    ///
    /// Returns error if application is invalid
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let service = UnderwritingService::new();
    /// let decision = service.evaluate(&application)?;
    /// match decision.risk_class {
    ///     RiskClass::Declined => println!("Application declined"),
    ///     _ => println!("Application accepted with class {:?}", decision.risk_class),
    /// }
    /// ```
    pub fn evaluate(&self, application: &UnderwritingApplication) -> Result<UnderwritingDecision, PolicyError> {
        // Validate application
        self.validate_application(application)?;

        // Run basic rules
        let rule_results = evaluate_basic_rules(application);

        // Determine risk class
        let risk_class = determine_risk_class(&rule_results);

        // Collect reasons and impacts
        let reasons: Vec<String> = rule_results
            .iter()
            .filter(|r| !r.passed || !matches!(r.impact, RuleImpact::None))
            .map(|r| r.message.clone())
            .collect();

        // Calculate total loading
        let total_loading: Decimal = rule_results
            .iter()
            .filter_map(|r| {
                if let RuleImpact::Loading(l) = r.impact {
                    Some(l)
                } else {
                    None
                }
            })
            .sum();

        // Determine required documents based on risk
        let required_documents = self.determine_required_documents(&risk_class, application);

        Ok(UnderwritingDecision {
            risk_class,
            reasons,
            exclusions: vec![],
            loading_percent: if total_loading.is_zero() {
                None
            } else {
                Some(total_loading)
            },
            coverage_modifications: vec![],
            required_documents,
            notes: None,
        })
    }

    /// Validates an underwriting application
    fn validate_application(&self, application: &UnderwritingApplication) -> Result<(), PolicyError> {
        // Check age
        let age = application.applicant.age();
        if age < 18 || age > 85 {
            return Err(PolicyError::Underwriting(
                format!("Age {} is outside valid range (18-85)", age)
            ));
        }

        // Check coverages
        if application.coverages.is_empty() {
            return Err(PolicyError::Underwriting(
                "At least one coverage is required".to_string()
            ));
        }

        Ok(())
    }

    /// Determines required documents based on risk profile
    fn determine_required_documents(
        &self,
        risk_class: &RiskClass,
        application: &UnderwritingApplication,
    ) -> Vec<String> {
        let mut docs = vec!["Completed Application Form".to_string()];

        // Age-based requirements
        let age = application.applicant.age();
        if age >= 50 {
            docs.push("Medical Examination Report".to_string());
        }

        // Sum assured based requirements
        let total_sum: Decimal = application.coverages
            .iter()
            .map(|c| c.sum_assured.amount())
            .sum();

        if total_sum >= dec!(500000) {
            docs.push("Financial Questionnaire".to_string());
        }

        if total_sum >= dec!(1000000) {
            docs.push("Attending Physician Statement".to_string());
            docs.push("Blood Test Results".to_string());
        }

        // Risk class based requirements
        match risk_class {
            RiskClass::Substandard | RiskClass::TableRated(_) => {
                docs.push("Detailed Medical History".to_string());
            }
            _ => {}
        }

        // Medical condition based
        if !application.medical_history.conditions.is_empty() {
            docs.push("Medical Records for Disclosed Conditions".to_string());
        }

        docs
    }
}

impl Default for UnderwritingService {
    fn default() -> Self {
        Self::new()
    }
}

/// Service for rating (premium calculation)
///
/// The RatingService calculates premiums based on product rules,
/// underwriting decisions, and actuarial tables.
pub struct RatingService {
    /// Base rate tables (would be loaded from database in production)
    rate_tables: Option<Value>,
}

impl RatingService {
    /// Creates a new rating service
    pub fn new() -> Self {
        Self {
            rate_tables: None,
        }
    }

    /// Loads rate tables
    pub fn with_rate_tables(mut self, tables: Value) -> Self {
        self.rate_tables = Some(tables);
        self
    }

    /// Calculates premium for a policy
    ///
    /// # Arguments
    ///
    /// * `coverages` - The coverages to rate
    /// * `age` - Age of the insured
    /// * `gender` - Gender of the insured
    /// * `is_smoker` - Smoking status
    /// * `risk_class` - Underwriting risk class
    ///
    /// # Returns
    ///
    /// Calculated premium information
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let service = RatingService::new();
    /// let premium = service.calculate_premium(
    ///     &coverages, 35, Gender::Male, false, RiskClass::Standard
    /// )?;
    /// ```
    pub fn calculate_premium(
        &self,
        coverages: &[Coverage],
        age: u32,
        is_smoker: bool,
        risk_class: RiskClass,
        currency: core_kernel::Currency,
    ) -> Result<Premium, PolicyError> {
        let mut total_premium = Money::zero(currency);

        for coverage in coverages {
            let base_rate = self.get_base_rate(&coverage, age, is_smoker)?;
            let sum_assured = coverage.sum_assured.amount();

            // Calculate coverage premium
            let coverage_premium = sum_assured * base_rate / dec!(1000);

            // Apply risk class multiplier
            let adjusted_premium = coverage_premium * risk_class.rate_multiplier();

            // Apply any coverage-specific loading
            let final_premium = if let Some(loading) = coverage.loading_percent {
                adjusted_premium * (dec!(1) + loading / dec!(100))
            } else {
                adjusted_premium
            };

            total_premium = total_premium + Money::new(final_premium, currency);
        }

        Ok(Premium::new(
            total_premium,
            crate::premium::PremiumFrequency::Annual,
        ))
    }

    /// Gets the base rate for a coverage
    ///
    /// In production, this would look up actuarial tables.
    /// For now, uses simplified formula.
    fn get_base_rate(
        &self,
        coverage: &Coverage,
        age: u32,
        is_smoker: bool,
    ) -> Result<Decimal, PolicyError> {
        // Simplified base rate calculation
        // In production, this would use mortality/morbidity tables
        let age_factor = dec!(0.5) + Decimal::from(age) * dec!(0.05);
        let smoker_factor = if is_smoker { dec!(2.0) } else { dec!(1.0) };

        let base_rate = match &coverage.coverage_type {
            crate::coverage::CoverageType::DeathBenefit => dec!(1.5),
            crate::coverage::CoverageType::CriticalIllness => dec!(2.0),
            crate::coverage::CoverageType::TotalPermanentDisability => dec!(0.8),
            crate::coverage::CoverageType::AccidentalDeath => dec!(0.5),
            crate::coverage::CoverageType::Hospitalization => dec!(3.0),
            _ => dec!(1.0),
        };

        Ok(base_rate * age_factor * smoker_factor)
    }
}

impl Default for RatingService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::underwriting::*;
    use crate::coverage::CoverageType;
    use chrono::NaiveDate;
    use core_kernel::Currency;

    fn create_test_application() -> UnderwritingApplication {
        UnderwritingApplication {
            applicant: ApplicantInfo {
                date_of_birth: NaiveDate::from_ymd_opt(1985, 6, 15).unwrap(),
                gender: Gender::Male,
                occupation: "Software Engineer".to_string(),
                occupation_class: 1,
                country: "US".to_string(),
            },
            medical_history: MedicalHistory {
                height_cm: 175,
                weight_kg: 75.0,
                is_smoker: false,
                is_former_smoker: false,
                conditions: vec![],
                family_history: vec![],
            },
            lifestyle: LifestyleInfo {
                hazardous_sports: vec![],
                aviation: None,
                alcohol_consumption: AlcoholLevel::Light,
                travel_risk_countries: vec![],
            },
            financial: FinancialInfo {
                annual_income: dec!(100000),
                net_worth: dec!(500000),
                existing_coverage: dec!(0),
                purpose: InsurancePurpose::FamilyProtection,
            },
            coverages: vec![
                Coverage::death_benefit(Money::new(dec!(500000), Currency::USD))
            ],
        }
    }

    #[test]
    fn test_underwriting_service() {
        let service = UnderwritingService::new();
        let application = create_test_application();

        let decision = service.evaluate(&application).unwrap();

        // Clean applicant should get good rating
        assert!(matches!(
            decision.risk_class,
            RiskClass::PreferredPlus | RiskClass::Preferred | RiskClass::Standard
        ));
    }

    #[test]
    fn test_rating_service() {
        let service = RatingService::new();
        let coverages = vec![
            Coverage::death_benefit(Money::new(dec!(500000), Currency::USD))
        ];

        let premium = service.calculate_premium(
            &coverages,
            35,
            false,
            RiskClass::Standard,
            Currency::USD,
        ).unwrap();

        assert!(premium.total_per_payment().amount() > dec!(0));
    }
}
