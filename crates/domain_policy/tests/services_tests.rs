//! Domain Services Tests
//!
//! This module contains comprehensive tests for the domain services:
//! - `UnderwritingService` - Application evaluation and risk assessment
//! - `RatingService` - Premium calculation based on risk factors
//!
//! # Test Coverage
//!
//! ## UnderwritingService Tests
//! - Application validation (age limits, coverage requirements)
//! - Risk class determination based on various factors
//! - Required document generation based on applicant profile
//! - Service configuration with custom rules
//!
//! ## RatingService Tests
//! - Premium calculation for different coverage types
//! - Age and smoking status impact on premiums
//! - Risk class multiplier effects
//! - Loading percentage application
//!
//! # Test Organization
//!
//! - `underwriting_validation` - Application validation tests
//! - `underwriting_evaluation` - Risk evaluation and document requirements
//! - `rating_calculation` - Premium calculation tests
//! - `rating_factors` - Tests for factors affecting premium

use chrono::Utc;
use core_kernel::{Currency, Money};
use domain_policy::coverage::{Coverage, CoverageType};
use domain_policy::services::{RatingService, UnderwritingService};
use domain_policy::underwriting::{
    AlcoholLevel, ApplicantInfo, ConditionStatus, FinancialInfo, Gender,
    InsurancePurpose, LifestyleInfo, MedicalCondition, MedicalHistory, RiskClass,
    UnderwritingApplication,
};
use rust_decimal_macros::dec;

// ============================================================================
// TEST FIXTURES
// ============================================================================

/// Creates a standard healthy applicant of specified age
fn create_healthy_applicant(age_years: i32) -> ApplicantInfo {
    let birth_date = Utc::now().date_naive() - chrono::Duration::days(age_years as i64 * 365);
    ApplicantInfo {
        date_of_birth: birth_date,
        gender: Gender::Male,
        occupation: "Software Engineer".to_string(),
        occupation_class: 1, // Low risk occupation
        country: "US".to_string(),
    }
}

/// Creates a standard medical history for testing
fn create_healthy_medical_history() -> MedicalHistory {
    MedicalHistory {
        height_cm: 175,
        weight_kg: 70.0, // BMI ~22.9 - healthy
        is_smoker: false,
        is_former_smoker: false,
        conditions: vec![],
        family_history: vec![],
    }
}

/// Creates a standard lifestyle with no risk factors
fn create_standard_lifestyle() -> LifestyleInfo {
    LifestyleInfo {
        hazardous_sports: vec![],
        aviation: None,
        alcohol_consumption: AlcoholLevel::None,
        travel_risk_countries: vec![],
    }
}

/// Creates standard financial info
fn create_standard_financial_info() -> FinancialInfo {
    FinancialInfo {
        annual_income: dec!(100000),
        net_worth: dec!(500000),
        existing_coverage: dec!(0),
        purpose: InsurancePurpose::FamilyProtection,
    }
}

/// Creates a complete valid underwriting application
fn create_valid_application(age_years: i32, sum_assured: rust_decimal::Decimal) -> UnderwritingApplication {
    UnderwritingApplication {
        applicant: create_healthy_applicant(age_years),
        medical_history: create_healthy_medical_history(),
        lifestyle: create_standard_lifestyle(),
        financial: create_standard_financial_info(),
        coverages: vec![Coverage::death_benefit(Money::new(sum_assured, Currency::USD))],
    }
}

// ============================================================================
// UNDERWRITING SERVICE - VALIDATION TESTS
// ============================================================================

mod underwriting_validation {
    use super::*;

    /// Verifies that applicants under 18 are rejected
    #[test]
    fn test_age_under_18_rejected() {
        let service = UnderwritingService::new();
        let application = create_valid_application(15, dec!(500000));

        let result = service.evaluate(&application);

        assert!(result.is_err(), "Applicants under 18 should be rejected");
    }

    /// Verifies that applicants over 85 are rejected
    #[test]
    fn test_age_over_85_rejected() {
        let service = UnderwritingService::new();
        let application = create_valid_application(90, dec!(500000));

        let result = service.evaluate(&application);

        assert!(result.is_err(), "Applicants over 85 should be rejected");
    }

    /// Verifies that applications with empty coverages are rejected
    #[test]
    fn test_empty_coverages_rejected() {
        let service = UnderwritingService::new();
        let birth_date = Utc::now().date_naive() - chrono::Duration::days(35 * 365);

        let application = UnderwritingApplication {
            applicant: ApplicantInfo {
                date_of_birth: birth_date,
                gender: Gender::Male,
                occupation: "Office Worker".to_string(),
                occupation_class: 1,
                country: "US".to_string(),
            },
            medical_history: create_healthy_medical_history(),
            lifestyle: create_standard_lifestyle(),
            financial: create_standard_financial_info(),
            coverages: vec![], // Empty coverages
        };

        let result = service.evaluate(&application);

        assert!(result.is_err(), "Empty coverages should be rejected");
    }

    /// Verifies that valid applications within acceptable age range are accepted
    #[test]
    fn test_valid_age_range_accepted() {
        let service = UnderwritingService::new();

        // Test age 20 (safely above minimum)
        let young_application = create_valid_application(20, dec!(500000));
        let young_result = service.evaluate(&young_application);
        assert!(young_result.is_ok(), "20 year old should be accepted");

        // Test age 80 (safely below maximum)
        let senior_application = create_valid_application(80, dec!(500000));
        let senior_result = service.evaluate(&senior_application);
        assert!(senior_result.is_ok(), "80 year old should be accepted");
    }
}

// ============================================================================
// UNDERWRITING SERVICE - EVALUATION TESTS
// ============================================================================

mod underwriting_evaluation {
    use super::*;

    /// Verifies that healthy applicants receive good risk classification
    #[test]
    fn test_healthy_applicant_receives_good_rating() {
        let service = UnderwritingService::new();
        let application = create_valid_application(35, dec!(500000));

        let decision = service.evaluate(&application).expect("Should evaluate successfully");

        assert!(
            matches!(
                decision.risk_class,
                RiskClass::PreferredPlus | RiskClass::Preferred | RiskClass::Standard
            ),
            "Healthy applicant should receive good rating, got {:?}",
            decision.risk_class
        );
    }

    /// Verifies that applicants age 50+ require medical examination
    #[test]
    fn test_age_50_requires_medical_exam() {
        let service = UnderwritingService::new();
        let application = create_valid_application(55, dec!(500000));

        let decision = service.evaluate(&application).expect("Should evaluate successfully");

        assert!(
            decision.required_documents.iter().any(|d| d.contains("Medical Examination")),
            "Applicants 50+ should require medical examination"
        );
    }

    /// Verifies that high sum assured (500k+) requires financial questionnaire
    #[test]
    fn test_high_sum_requires_financial_questionnaire() {
        let service = UnderwritingService::new();
        let application = create_valid_application(35, dec!(600000));

        let decision = service.evaluate(&application).expect("Should evaluate successfully");

        assert!(
            decision.required_documents.iter().any(|d| d.contains("Financial Questionnaire")),
            "Sum assured over 500k should require financial questionnaire"
        );
    }

    /// Verifies that very high sum assured (1M+) requires APS and blood tests
    #[test]
    fn test_very_high_sum_requires_aps_and_blood_test() {
        let service = UnderwritingService::new();
        let application = create_valid_application(35, dec!(1500000));

        let decision = service.evaluate(&application).expect("Should evaluate successfully");

        assert!(
            decision.required_documents.iter().any(|d| d.contains("Attending Physician")),
            "Sum assured over 1M should require APS"
        );
        assert!(
            decision.required_documents.iter().any(|d| d.contains("Blood Test")),
            "Sum assured over 1M should require blood tests"
        );
    }

    /// Verifies that disclosed medical conditions require medical records
    #[test]
    fn test_medical_conditions_require_records() {
        let service = UnderwritingService::new();
        let birth_date = Utc::now().date_naive() - chrono::Duration::days(35 * 365);

        let application = UnderwritingApplication {
            applicant: ApplicantInfo {
                date_of_birth: birth_date,
                gender: Gender::Male,
                occupation: "Office Worker".to_string(),
                occupation_class: 1,
                country: "US".to_string(),
            },
            medical_history: MedicalHistory {
                height_cm: 175,
                weight_kg: 70.0,
                is_smoker: false,
                is_former_smoker: false,
                conditions: vec![MedicalCondition {
                    code: "E11".to_string(),
                    name: "Type 2 Diabetes".to_string(),
                    diagnosed_date: None,
                    status: ConditionStatus::Controlled,
                    treatment: Some("Metformin 500mg twice daily".to_string()),
                }],
                family_history: vec![],
            },
            lifestyle: create_standard_lifestyle(),
            financial: create_standard_financial_info(),
            coverages: vec![Coverage::death_benefit(Money::new(dec!(500000), Currency::USD))],
        };

        let decision = service.evaluate(&application).expect("Should evaluate successfully");

        assert!(
            decision.required_documents.iter().any(|d| d.contains("Medical Records")),
            "Disclosed conditions should require medical records"
        );
    }

    /// Verifies that high-risk applicants receive substandard or table-rated classification
    #[test]
    fn test_high_risk_applicant_receives_substandard_rating() {
        let service = UnderwritingService::new();
        let birth_date = Utc::now().date_naive() - chrono::Duration::days(35 * 365);

        let application = UnderwritingApplication {
            applicant: ApplicantInfo {
                date_of_birth: birth_date,
                gender: Gender::Male,
                occupation: "Office Worker".to_string(),
                occupation_class: 1,
                country: "US".to_string(),
            },
            medical_history: MedicalHistory {
                height_cm: 175,
                weight_kg: 110.0, // BMI ~36 - obese
                is_smoker: true,  // Active smoker
                is_former_smoker: false,
                conditions: vec![],
                family_history: vec![],
            },
            lifestyle: create_standard_lifestyle(),
            financial: create_standard_financial_info(),
            coverages: vec![Coverage::death_benefit(Money::new(dec!(500000), Currency::USD))],
        };

        let decision = service.evaluate(&application).expect("Should evaluate successfully");

        assert!(
            matches!(
                decision.risk_class,
                RiskClass::Substandard | RiskClass::TableRated(_)
            ),
            "High-risk applicant should receive substandard/table rating, got {:?}",
            decision.risk_class
        );
    }
}

// ============================================================================
// UNDERWRITING SERVICE - CONFIGURATION TESTS
// ============================================================================

mod underwriting_configuration {
    use super::*;

    /// Verifies UnderwritingService can be created with custom rules
    #[test]
    fn test_service_with_custom_rules() {
        let rules = serde_json::json!({
            "max_age": 70,
            "max_sum_assured": 5000000,
            "require_medical_above": 300000
        });

        let service = UnderwritingService::new().with_rules(rules);
        let application = create_valid_application(35, dec!(500000));

        let result = service.evaluate(&application);

        assert!(result.is_ok(), "Service with rules should evaluate applications");
    }

    /// Verifies UnderwritingService::default() works correctly
    #[test]
    fn test_service_default_implementation() {
        let service = UnderwritingService::default();
        let application = create_valid_application(35, dec!(500000));

        let result = service.evaluate(&application);

        assert!(result.is_ok(), "Default service should evaluate applications");
    }
}

// ============================================================================
// RATING SERVICE - CALCULATION TESTS
// ============================================================================

mod rating_calculation {
    use super::*;

    /// Verifies basic premium calculation works
    #[test]
    fn test_basic_premium_calculation() {
        let service = RatingService::new();
        let coverages = vec![Coverage::death_benefit(Money::new(dec!(500000), Currency::USD))];

        let premium = service
            .calculate_premium(&coverages, 35, false, RiskClass::Standard, Currency::USD)
            .expect("Should calculate premium");

        assert!(
            premium.total_per_payment().amount() > dec!(0),
            "Premium should be positive"
        );
    }

    /// Verifies premium calculation for different coverage types
    #[test]
    fn test_premium_for_different_coverage_types() {
        let service = RatingService::new();

        let coverage_types = vec![
            (CoverageType::DeathBenefit, "Death Benefit"),
            (CoverageType::CriticalIllness, "Critical Illness"),
            (CoverageType::TotalPermanentDisability, "TPD"),
            (CoverageType::AccidentalDeath, "Accidental Death"),
            (CoverageType::Hospitalization, "Hospitalization"),
        ];

        for (coverage_type, name) in coverage_types {
            let coverages = vec![Coverage::new(
                coverage_type,
                Money::new(dec!(100000), Currency::USD),
            )];

            let premium = service
                .calculate_premium(&coverages, 35, false, RiskClass::Standard, Currency::USD)
                .expect(&format!("{} premium calculation should succeed", name));

            assert!(
                premium.total_per_payment().amount() > dec!(0),
                "{} premium should be positive",
                name
            );
        }
    }

    /// Verifies premium calculation with coverage loading
    #[test]
    fn test_premium_with_coverage_loading() {
        let service = RatingService::new();

        let mut coverage = Coverage::death_benefit(Money::new(dec!(500000), Currency::USD));
        coverage.loading_percent = Some(dec!(25)); // 25% loading

        let coverages = vec![coverage];

        let premium = service
            .calculate_premium(&coverages, 35, false, RiskClass::Standard, Currency::USD)
            .expect("Should calculate premium with loading");

        assert!(
            premium.total_per_payment().amount() > dec!(0),
            "Premium with loading should be positive"
        );
    }
}

// ============================================================================
// RATING SERVICE - FACTOR TESTS
// ============================================================================

mod rating_factors {
    use super::*;

    /// Verifies that smokers pay higher premiums
    #[test]
    fn test_smoker_premium_higher() {
        let service = RatingService::new();
        let coverages = vec![Coverage::death_benefit(Money::new(dec!(500000), Currency::USD))];

        let non_smoker_premium = service
            .calculate_premium(&coverages, 35, false, RiskClass::Standard, Currency::USD)
            .expect("Non-smoker premium should calculate");

        let smoker_premium = service
            .calculate_premium(&coverages, 35, true, RiskClass::Standard, Currency::USD)
            .expect("Smoker premium should calculate");

        assert!(
            smoker_premium.total_per_payment().amount() > non_smoker_premium.total_per_payment().amount(),
            "Smoker premium ({}) should be higher than non-smoker ({})",
            smoker_premium.total_per_payment().amount(),
            non_smoker_premium.total_per_payment().amount()
        );
    }

    /// Verifies that older applicants pay higher premiums
    #[test]
    fn test_older_applicants_pay_more() {
        let service = RatingService::new();
        let coverages = vec![Coverage::death_benefit(Money::new(dec!(500000), Currency::USD))];

        let young_premium = service
            .calculate_premium(&coverages, 25, false, RiskClass::Standard, Currency::USD)
            .expect("Young applicant premium should calculate");

        let older_premium = service
            .calculate_premium(&coverages, 55, false, RiskClass::Standard, Currency::USD)
            .expect("Older applicant premium should calculate");

        assert!(
            older_premium.total_per_payment().amount() > young_premium.total_per_payment().amount(),
            "Older applicant premium ({}) should be higher than young ({})",
            older_premium.total_per_payment().amount(),
            young_premium.total_per_payment().amount()
        );
    }

    /// Verifies risk class affects premium correctly
    #[test]
    fn test_risk_class_affects_premium() {
        let service = RatingService::new();
        let coverages = vec![Coverage::death_benefit(Money::new(dec!(500000), Currency::USD))];

        let preferred_premium = service
            .calculate_premium(&coverages, 35, false, RiskClass::Preferred, Currency::USD)
            .expect("Preferred premium should calculate");

        let standard_premium = service
            .calculate_premium(&coverages, 35, false, RiskClass::Standard, Currency::USD)
            .expect("Standard premium should calculate");

        let substandard_premium = service
            .calculate_premium(&coverages, 35, false, RiskClass::Substandard, Currency::USD)
            .expect("Substandard premium should calculate");

        // Verify ordering: Preferred < Standard < Substandard
        assert!(
            preferred_premium.total_per_payment().amount() < standard_premium.total_per_payment().amount(),
            "Preferred premium should be less than standard"
        );
        assert!(
            standard_premium.total_per_payment().amount() < substandard_premium.total_per_payment().amount(),
            "Standard premium should be less than substandard"
        );
    }
}

// ============================================================================
// RATING SERVICE - CONFIGURATION TESTS
// ============================================================================

mod rating_configuration {
    use super::*;

    /// Verifies RatingService can be configured with rate tables
    #[test]
    fn test_service_with_rate_tables() {
        let tables = serde_json::json!({
            "mortality_table": "CSO2017",
            "interest_rate": 0.04,
            "expense_loading": 0.10
        });

        let service = RatingService::new().with_rate_tables(tables);
        let coverages = vec![Coverage::death_benefit(Money::new(dec!(500000), Currency::USD))];

        let premium = service
            .calculate_premium(&coverages, 35, false, RiskClass::Standard, Currency::USD)
            .expect("Service with tables should calculate premium");

        assert!(premium.total_per_payment().amount() > dec!(0));
    }

    /// Verifies RatingService::default() works correctly
    #[test]
    fn test_service_default_implementation() {
        let service = RatingService::default();
        let coverages = vec![Coverage::death_benefit(Money::new(dec!(500000), Currency::USD))];

        let premium = service
            .calculate_premium(&coverages, 35, false, RiskClass::Standard, Currency::USD)
            .expect("Default service should calculate premium");

        assert!(premium.total_per_payment().amount() > dec!(0));
    }
}
