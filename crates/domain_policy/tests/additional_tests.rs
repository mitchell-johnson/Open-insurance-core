//! Additional Domain Policy Tests
//!
//! This module contains supplementary tests covering:
//! - Error construction and handling
//! - Endorsement builder methods
//! - Underwriting rule evaluation
//! - Medical history calculations
//!
//! These tests complement the main test modules:
//! - `aggregate_tests.rs` - Policy aggregate tests
//! - `services_tests.rs` - Service layer tests
//! - `premium_tests.rs` - Premium calculation tests
//!
//! # Test Organization
//!
//! - `error_tests` - PolicyError construction and variants
//! - `endorsement_tests` - Endorsement builder and state methods
//! - `underwriting_tests` - Underwriting rule evaluation
//! - `medical_tests` - BMI and health assessment calculations

use chrono::{Datelike, NaiveDate, Utc};
use core_kernel::{Currency, Money};
use rust_decimal_macros::dec;

// ============================================================================
// ERROR TESTS
// ============================================================================

mod error_tests {
    use domain_policy::error::PolicyError;

    /// Verifies PolicyError::validation constructor
    #[test]
    fn test_policy_error_validation() {
        let error = PolicyError::validation("Invalid input data");

        match error {
            PolicyError::Validation(msg) => {
                assert_eq!(msg, "Invalid input data");
            }
            _ => panic!("Expected Validation error variant"),
        }
    }

    /// Verifies PolicyError::coverage_not_found constructor
    #[test]
    fn test_policy_error_coverage_not_found() {
        let error = PolicyError::coverage_not_found("cov-123-abc");

        match error {
            PolicyError::CoverageNotFound(id) => {
                assert_eq!(id, "cov-123-abc");
            }
            _ => panic!("Expected CoverageNotFound error variant"),
        }
    }

    /// Verifies PolicyError::rule_violation constructor
    #[test]
    fn test_policy_error_rule_violation() {
        let error = PolicyError::rule_violation("Maximum coverage age exceeded");

        match error {
            PolicyError::ProductRuleViolation(msg) => {
                assert_eq!(msg, "Maximum coverage age exceeded");
            }
            _ => panic!("Expected ProductRuleViolation error variant"),
        }
    }
}

// ============================================================================
// ENDORSEMENT BUILDER TESTS
// ============================================================================

mod endorsement_tests {
    use super::*;
    use domain_policy::endorsement::{Address, Endorsement, EndorsementStatus, EndorsementType};

    /// Creates a test address for endorsement testing
    fn create_test_address() -> Address {
        Address {
            line1: "123 Main Street".to_string(),
            line2: Some("Suite 100".to_string()),
            city: "New York".to_string(),
            state: Some("NY".to_string()),
            postal_code: "10001".to_string(),
            country: "US".to_string(),
        }
    }

    /// Creates a standard test endorsement
    fn create_test_endorsement() -> Endorsement {
        Endorsement::new(
            EndorsementType::AddressChange {
                new_address: create_test_address(),
            },
            Utc::now().date_naive(),
        )
    }

    /// Verifies with_reason builder method
    #[test]
    fn test_endorsement_with_reason() {
        let endorsement = create_test_endorsement()
            .with_reason("Customer relocated for employment");

        assert!(endorsement.reason.is_some(), "Reason should be set");
        assert_eq!(
            endorsement.reason.unwrap(),
            "Customer relocated for employment"
        );
    }

    /// Verifies with_premium_adjustment builder method
    #[test]
    fn test_endorsement_with_premium_adjustment() {
        let endorsement = create_test_endorsement()
            .with_premium_adjustment(dec!(75.50));

        assert!(
            endorsement.premium_adjustment.is_some(),
            "Premium adjustment should be set"
        );
        assert_eq!(endorsement.premium_adjustment.unwrap(), dec!(75.50));
    }

    /// Verifies requested_by builder method
    #[test]
    fn test_endorsement_requested_by() {
        let endorsement = create_test_endorsement()
            .requested_by("agent@insurance.com");

        assert!(endorsement.requested_by.is_some());
        assert_eq!(endorsement.requested_by.unwrap(), "agent@insurance.com");
    }

    /// Verifies reject method transitions endorsement to Rejected status
    #[test]
    fn test_endorsement_reject() {
        let mut endorsement = create_test_endorsement();

        endorsement.reject("Missing supporting documentation");

        assert_eq!(
            endorsement.status,
            EndorsementStatus::Rejected,
            "Status should be Rejected"
        );
        assert!(
            endorsement.reason.is_some(),
            "Rejection reason should be set"
        );
    }

    /// Verifies mark_applied transitions endorsement to Applied status
    #[test]
    fn test_endorsement_mark_applied() {
        let mut endorsement = create_test_endorsement();

        // Must approve before marking as applied
        endorsement.approve("approver@insurance.com");
        endorsement.mark_applied();

        assert_eq!(
            endorsement.status,
            EndorsementStatus::Applied,
            "Status should be Applied"
        );
        assert!(
            endorsement.processed_at.is_some(),
            "Processed timestamp should be set"
        );
    }

    /// Verifies is_retroactive correctly identifies past effective dates
    #[test]
    fn test_endorsement_is_retroactive_past_date() {
        let past_date = Utc::now().date_naive() - chrono::Duration::days(30);
        let endorsement = Endorsement::new(
            EndorsementType::AddressChange {
                new_address: create_test_address(),
            },
            past_date,
        );

        assert!(
            endorsement.is_retroactive(),
            "Past effective date should be retroactive"
        );
    }

    /// Verifies is_retroactive correctly identifies future effective dates
    #[test]
    fn test_endorsement_is_retroactive_future_date() {
        let future_date = Utc::now().date_naive() + chrono::Duration::days(30);
        let endorsement = Endorsement::new(
            EndorsementType::AddressChange {
                new_address: create_test_address(),
            },
            future_date,
        );

        assert!(
            !endorsement.is_retroactive(),
            "Future effective date should not be retroactive"
        );
    }

    /// Verifies requires_additional_premium with positive adjustment
    #[test]
    fn test_endorsement_requires_additional_premium_positive() {
        let endorsement = create_test_endorsement()
            .with_premium_adjustment(dec!(100));

        assert!(
            endorsement.requires_additional_premium(),
            "Positive adjustment should require additional premium"
        );
    }

    /// Verifies requires_additional_premium with negative adjustment (refund)
    #[test]
    fn test_endorsement_requires_additional_premium_negative() {
        let endorsement = create_test_endorsement()
            .with_premium_adjustment(dec!(-50));

        assert!(
            !endorsement.requires_additional_premium(),
            "Negative adjustment should not require additional premium"
        );
    }

    /// Verifies requires_additional_premium with no adjustment
    #[test]
    fn test_endorsement_requires_additional_premium_none() {
        let endorsement = create_test_endorsement();

        assert!(
            !endorsement.requires_additional_premium(),
            "No adjustment should not require additional premium"
        );
    }
}

// ============================================================================
// UNDERWRITING RULE TESTS
// ============================================================================

mod underwriting_tests {
    use super::*;
    use domain_policy::coverage::Coverage;
    use domain_policy::underwriting::{
        determine_risk_class, evaluate_basic_rules, AlcoholLevel, ApplicantInfo,
        FinancialInfo, Gender, InsurancePurpose, LifestyleInfo, MedicalHistory,
        RiskClass, RuleImpact, RuleResult, UnderwritingApplication,
    };

    /// Creates an applicant of specified age
    fn create_test_applicant(age_years: i32) -> ApplicantInfo {
        let birth_date = Utc::now().date_naive() - chrono::Duration::days(age_years as i64 * 365);
        ApplicantInfo {
            date_of_birth: birth_date,
            gender: Gender::Male,
            occupation: "Software Engineer".to_string(),
            occupation_class: 1,
            country: "US".to_string(),
        }
    }

    /// Creates medical history with specified parameters
    fn create_test_medical_history(height_cm: u32, weight_kg: f32, is_smoker: bool) -> MedicalHistory {
        MedicalHistory {
            height_cm,
            weight_kg,
            is_smoker,
            is_former_smoker: false,
            conditions: vec![],
            family_history: vec![],
        }
    }

    /// Creates standard financial info
    fn create_test_financial_info(annual_income: rust_decimal::Decimal) -> FinancialInfo {
        FinancialInfo {
            annual_income,
            net_worth: dec!(500000),
            existing_coverage: dec!(0),
            purpose: InsurancePurpose::FamilyProtection,
        }
    }

    /// Creates standard lifestyle info
    fn create_test_lifestyle() -> LifestyleInfo {
        LifestyleInfo {
            hazardous_sports: vec![],
            aviation: None,
            alcohol_consumption: AlcoholLevel::None,
            travel_risk_countries: vec![],
        }
    }

    /// Creates test coverage
    fn create_test_coverage(sum_assured: Money) -> Coverage {
        Coverage::death_benefit(sum_assured)
    }

    // -------------------------------------------------------------------------
    // BMI TESTS
    // -------------------------------------------------------------------------

    /// Verifies BMI calculation is accurate
    #[test]
    fn test_medical_history_bmi_calculation() {
        // BMI = weight_kg / (height_m^2)
        // 70kg, 175cm -> 70 / 1.75^2 = 70 / 3.0625 ≈ 22.86
        let medical = create_test_medical_history(175, 70.0, false);
        let bmi = medical.bmi();

        assert!(
            bmi > 22.0 && bmi < 23.0,
            "BMI for 70kg/175cm should be ~22.86, got {}",
            bmi
        );
    }

    /// Verifies healthy BMI range detection
    #[test]
    fn test_medical_history_is_healthy_bmi() {
        // Healthy BMI range is 18.5 - 24.9
        let healthy = create_test_medical_history(175, 70.0, false);
        assert!(healthy.is_healthy_bmi(), "BMI ~22.9 should be healthy");

        // Underweight (BMI < 18.5)
        let underweight = create_test_medical_history(175, 50.0, false);
        assert!(!underweight.is_healthy_bmi(), "BMI ~16.3 should be underweight");

        // Overweight (BMI > 24.9)
        let overweight = create_test_medical_history(175, 100.0, false);
        assert!(!overweight.is_healthy_bmi(), "BMI ~32.7 should be overweight");
    }

    // -------------------------------------------------------------------------
    // BASIC RULES EVALUATION TESTS
    // -------------------------------------------------------------------------

    /// Verifies young applicants (under 18) trigger decline
    #[test]
    fn test_evaluate_rules_young_applicant_declined() {
        let application = UnderwritingApplication {
            applicant: create_test_applicant(17),
            medical_history: create_test_medical_history(175, 70.0, false),
            financial: create_test_financial_info(dec!(100000)),
            lifestyle: create_test_lifestyle(),
            coverages: vec![create_test_coverage(Money::new(dec!(500000), Currency::USD))],
        };

        let results = evaluate_basic_rules(&application);

        let has_decline = results.iter().any(|r| r.impact == RuleImpact::Decline);
        assert!(has_decline, "Under 18 should trigger decline");
    }

    /// Verifies senior applicants (65+) trigger referral
    #[test]
    fn test_evaluate_rules_senior_applicant_referral() {
        let application = UnderwritingApplication {
            applicant: create_test_applicant(70),
            medical_history: create_test_medical_history(175, 70.0, false),
            financial: create_test_financial_info(dec!(100000)),
            lifestyle: create_test_lifestyle(),
            coverages: vec![create_test_coverage(Money::new(dec!(500000), Currency::USD))],
        };

        let results = evaluate_basic_rules(&application);

        let has_referral = results.iter().any(|r| r.impact == RuleImpact::Referral);
        assert!(has_referral, "Age 70 should trigger referral");
    }

    /// Verifies extreme BMI triggers referral
    #[test]
    fn test_evaluate_rules_extreme_bmi_referral() {
        let application = UnderwritingApplication {
            applicant: create_test_applicant(35),
            medical_history: create_test_medical_history(175, 130.0, false), // BMI > 40
            financial: create_test_financial_info(dec!(100000)),
            lifestyle: create_test_lifestyle(),
            coverages: vec![create_test_coverage(Money::new(dec!(500000), Currency::USD))],
        };

        let results = evaluate_basic_rules(&application);

        let has_bmi_referral = results.iter().any(|r| r.rule_name == "bmi_extreme");
        assert!(has_bmi_referral, "Extreme BMI should trigger referral");
    }

    /// Verifies smokers receive loading
    #[test]
    fn test_evaluate_rules_smoker_loading() {
        let application = UnderwritingApplication {
            applicant: create_test_applicant(35),
            medical_history: create_test_medical_history(175, 70.0, true), // Smoker
            financial: create_test_financial_info(dec!(100000)),
            lifestyle: create_test_lifestyle(),
            coverages: vec![create_test_coverage(Money::new(dec!(500000), Currency::USD))],
        };

        let results = evaluate_basic_rules(&application);

        let has_smoker_loading = results.iter().any(|r| r.rule_name == "smoker_status");
        assert!(has_smoker_loading, "Smoker should receive loading");
    }

    /// Verifies high coverage multiple triggers referral
    #[test]
    fn test_evaluate_rules_high_income_multiple_referral() {
        let application = UnderwritingApplication {
            applicant: create_test_applicant(35),
            medical_history: create_test_medical_history(175, 70.0, false),
            financial: create_test_financial_info(dec!(50000)), // Low income
            lifestyle: create_test_lifestyle(),
            coverages: vec![create_test_coverage(Money::new(dec!(2000000), Currency::USD))], // 40x income
        };

        let results = evaluate_basic_rules(&application);

        let has_financial_referral = results.iter().any(|r| r.rule_name == "financial_justification");
        assert!(has_financial_referral, ">20x income should trigger referral");
    }

    /// Verifies unhealthy BMI triggers loading
    #[test]
    fn test_evaluate_rules_bmi_loading() {
        // Test underweight BMI
        let application = UnderwritingApplication {
            applicant: create_test_applicant(35),
            medical_history: create_test_medical_history(175, 55.0, false), // BMI ~18
            financial: create_test_financial_info(dec!(100000)),
            lifestyle: create_test_lifestyle(),
            coverages: vec![create_test_coverage(Money::new(dec!(500000), Currency::USD))],
        };

        let results = evaluate_basic_rules(&application);

        let has_bmi_loading = results.iter().any(|r| r.rule_name == "bmi_loading");
        assert!(has_bmi_loading, "Unhealthy BMI should receive loading");
    }

    /// Verifies overweight BMI triggers loading
    #[test]
    fn test_evaluate_rules_overweight_bmi_loading() {
        let application = UnderwritingApplication {
            applicant: create_test_applicant(35),
            medical_history: create_test_medical_history(175, 95.0, false), // BMI ~31
            financial: create_test_financial_info(dec!(100000)),
            lifestyle: create_test_lifestyle(),
            coverages: vec![create_test_coverage(Money::new(dec!(500000), Currency::USD))],
        };

        let results = evaluate_basic_rules(&application);

        let has_bmi_loading = results.iter().any(|r| r.rule_name == "bmi_loading");
        assert!(has_bmi_loading, "Overweight BMI should receive loading");
    }

    /// Verifies former smokers receive reduced loading
    #[test]
    fn test_evaluate_rules_former_smoker() {
        let mut medical = create_test_medical_history(175, 70.0, false);
        medical.is_former_smoker = true;

        let application = UnderwritingApplication {
            applicant: create_test_applicant(35),
            medical_history: medical,
            financial: create_test_financial_info(dec!(100000)),
            lifestyle: create_test_lifestyle(),
            coverages: vec![create_test_coverage(Money::new(dec!(500000), Currency::USD))],
        };

        let results = evaluate_basic_rules(&application);

        let has_former_smoker = results.iter().any(|r| r.rule_name == "former_smoker");
        assert!(has_former_smoker, "Former smoker should be flagged");
    }

    /// Verifies high-risk occupations receive loading
    #[test]
    fn test_evaluate_rules_high_risk_occupation() {
        let mut applicant = create_test_applicant(35);
        applicant.occupation_class = 4; // High risk

        let application = UnderwritingApplication {
            applicant,
            medical_history: create_test_medical_history(175, 70.0, false),
            financial: create_test_financial_info(dec!(100000)),
            lifestyle: create_test_lifestyle(),
            coverages: vec![create_test_coverage(Money::new(dec!(500000), Currency::USD))],
        };

        let results = evaluate_basic_rules(&application);

        let has_occupation_loading = results.iter().any(|r| r.rule_name == "occupation_hazard");
        assert!(has_occupation_loading, "High-risk occupation should receive loading");
    }

    /// Verifies zero income is handled gracefully
    #[test]
    fn test_evaluate_rules_zero_income_handled() {
        let application = UnderwritingApplication {
            applicant: create_test_applicant(35),
            medical_history: create_test_medical_history(175, 70.0, false),
            financial: create_test_financial_info(dec!(0)), // Zero income
            lifestyle: create_test_lifestyle(),
            coverages: vec![create_test_coverage(Money::new(dec!(500000), Currency::USD))],
        };

        let results = evaluate_basic_rules(&application);

        // Should not panic with zero income
        assert!(!results.is_empty(), "Should return rules even with zero income");
    }

    // -------------------------------------------------------------------------
    // RISK CLASS DETERMINATION TESTS
    // -------------------------------------------------------------------------

    /// Verifies decline impact results in Declined risk class
    #[test]
    fn test_determine_risk_class_declined() {
        let results = vec![RuleResult {
            rule_name: "age_check".to_string(),
            passed: false,
            message: "Applicant is under minimum age".to_string(),
            impact: RuleImpact::Decline,
        }];

        let risk_class = determine_risk_class(&results);

        assert_eq!(risk_class, RiskClass::Declined);
    }

    /// Verifies no issues results in PreferredPlus
    #[test]
    fn test_determine_risk_class_preferred_plus() {
        let results = vec![RuleResult {
            rule_name: "health_check".to_string(),
            passed: true,
            message: "All health criteria met".to_string(),
            impact: RuleImpact::None,
        }];

        let risk_class = determine_risk_class(&results);

        assert_eq!(risk_class, RiskClass::PreferredPlus);
    }

    /// Verifies small loading results in Preferred
    #[test]
    fn test_determine_risk_class_preferred() {
        let results = vec![RuleResult {
            rule_name: "bmi_check".to_string(),
            passed: true,
            message: "BMI slightly elevated".to_string(),
            impact: RuleImpact::Loading(dec!(5)),
        }];

        let risk_class = determine_risk_class(&results);

        assert_eq!(risk_class, RiskClass::Preferred);
    }

    /// Verifies moderate loading results in Standard
    #[test]
    fn test_determine_risk_class_standard() {
        let results = vec![RuleResult {
            rule_name: "occupation_check".to_string(),
            passed: true,
            message: "Moderate risk occupation".to_string(),
            impact: RuleImpact::Loading(dec!(20)),
        }];

        let risk_class = determine_risk_class(&results);

        assert_eq!(risk_class, RiskClass::Standard);
    }

    /// Verifies significant loading results in Substandard
    #[test]
    fn test_determine_risk_class_substandard() {
        let results = vec![RuleResult {
            rule_name: "smoking_status".to_string(),
            passed: true,
            message: "Active smoker".to_string(),
            impact: RuleImpact::Loading(dec!(50)),
        }];

        let risk_class = determine_risk_class(&results);

        assert_eq!(risk_class, RiskClass::Substandard);
    }

    /// Verifies very high loading results in TableRated
    #[test]
    fn test_determine_risk_class_table_rated() {
        let results = vec![RuleResult {
            rule_name: "multiple_factors".to_string(),
            passed: true,
            message: "Multiple risk factors".to_string(),
            impact: RuleImpact::Loading(dec!(150)),
        }];

        let risk_class = determine_risk_class(&results);

        assert!(
            matches!(risk_class, RiskClass::TableRated(_)),
            "Expected TableRated, got {:?}",
            risk_class
        );
    }

    // -------------------------------------------------------------------------
    // RISK CLASS RATE MULTIPLIER TESTS
    // -------------------------------------------------------------------------

    /// Verifies rate multipliers for all risk classes
    #[test]
    fn test_risk_class_rate_multipliers() {
        // PreferredPlus gets the best rate (lowest multiplier)
        assert!(
            RiskClass::PreferredPlus.rate_multiplier() < dec!(1.0),
            "PreferredPlus should have multiplier < 1.0"
        );

        // Preferred also gets a discount
        assert!(
            RiskClass::Preferred.rate_multiplier() < dec!(1.0),
            "Preferred should have multiplier < 1.0"
        );

        // Standard is the base rate
        assert_eq!(
            RiskClass::Standard.rate_multiplier(),
            dec!(1.0),
            "Standard should have multiplier = 1.0"
        );

        // Substandard pays more
        assert!(
            RiskClass::Substandard.rate_multiplier() > dec!(1.0),
            "Substandard should have multiplier > 1.0"
        );

        // TableRated pays even more
        assert!(
            RiskClass::TableRated(1).rate_multiplier() > dec!(1.0),
            "TableRated should have multiplier > 1.0"
        );

        // Declined has zero multiplier (cannot be rated)
        assert_eq!(
            RiskClass::Declined.rate_multiplier(),
            dec!(0),
            "Declined should have multiplier = 0"
        );
    }
}
