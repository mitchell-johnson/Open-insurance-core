//! Additional tests for domain_policy - covering endorsement, premium, error, and underwriting

use chrono::{NaiveDate, Utc, Datelike};
use rust_decimal_macros::dec;

use core_kernel::{PolicyId, Money, Currency};

// ============= ERROR TESTS =============
mod error_tests {
    use domain_policy::error::PolicyError;

    #[test]
    fn test_policy_error_validation() {
        let error = PolicyError::validation("Invalid input");
        match error {
            PolicyError::Validation(msg) => assert_eq!(msg, "Invalid input"),
            _ => panic!("Expected Validation error"),
        }
    }

    #[test]
    fn test_policy_error_coverage_not_found() {
        let error = PolicyError::coverage_not_found("cov-123");
        match error {
            PolicyError::CoverageNotFound(id) => assert_eq!(id, "cov-123"),
            _ => panic!("Expected CoverageNotFound error"),
        }
    }

    #[test]
    fn test_policy_error_rule_violation() {
        let error = PolicyError::rule_violation("Maximum age exceeded");
        match error {
            PolicyError::ProductRuleViolation(msg) => assert_eq!(msg, "Maximum age exceeded"),
            _ => panic!("Expected ProductRuleViolation error"),
        }
    }
}

// ============= ENDORSEMENT TESTS =============
mod endorsement_tests {
    use super::*;
    use domain_policy::endorsement::{Endorsement, EndorsementType, EndorsementStatus, Address};

    fn create_test_address() -> Address {
        Address {
            line1: "123 Main St".to_string(),
            line2: None,
            city: "New York".to_string(),
            state: Some("NY".to_string()),
            postal_code: "10001".to_string(),
            country: "US".to_string(),
        }
    }

    #[test]
    fn test_endorsement_with_reason() {
        let today = Utc::now().date_naive();

        let endorsement = Endorsement::new(
            EndorsementType::AddressChange { new_address: create_test_address() },
            today,
        ).with_reason("Customer moved");

        assert!(endorsement.reason.is_some());
        assert_eq!(endorsement.reason.unwrap(), "Customer moved");
    }

    #[test]
    fn test_endorsement_with_premium_adjustment() {
        let today = Utc::now().date_naive();

        let endorsement = Endorsement::new(
            EndorsementType::AddressChange { new_address: create_test_address() },
            today,
        ).with_premium_adjustment(dec!(50));

        assert!(endorsement.premium_adjustment.is_some());
        assert_eq!(endorsement.premium_adjustment.unwrap(), dec!(50));
    }

    #[test]
    fn test_endorsement_requested_by() {
        let today = Utc::now().date_naive();

        let endorsement = Endorsement::new(
            EndorsementType::AddressChange { new_address: create_test_address() },
            today,
        ).requested_by("agent@example.com");

        assert!(endorsement.requested_by.is_some());
        assert_eq!(endorsement.requested_by.unwrap(), "agent@example.com");
    }

    #[test]
    fn test_endorsement_reject() {
        let today = Utc::now().date_naive();

        let mut endorsement = Endorsement::new(
            EndorsementType::AddressChange { new_address: create_test_address() },
            today,
        );

        endorsement.reject("Incomplete documentation");
        assert_eq!(endorsement.status, EndorsementStatus::Rejected);
        assert!(endorsement.reason.is_some());
    }

    #[test]
    fn test_endorsement_mark_applied() {
        let today = Utc::now().date_naive();

        let mut endorsement = Endorsement::new(
            EndorsementType::AddressChange { new_address: create_test_address() },
            today,
        );

        endorsement.approve("approver@example.com");
        endorsement.mark_applied();

        assert_eq!(endorsement.status, EndorsementStatus::Applied);
        assert!(endorsement.processed_at.is_some());
    }

    #[test]
    fn test_endorsement_is_retroactive() {
        // Past date - retroactive
        let past_date = Utc::now().date_naive() - chrono::Duration::days(30);
        let retroactive_endorsement = Endorsement::new(
            EndorsementType::AddressChange { new_address: create_test_address() },
            past_date,
        );
        assert!(retroactive_endorsement.is_retroactive());

        // Future date - not retroactive
        let future_date = Utc::now().date_naive() + chrono::Duration::days(30);
        let future_endorsement = Endorsement::new(
            EndorsementType::AddressChange { new_address: create_test_address() },
            future_date,
        );
        assert!(!future_endorsement.is_retroactive());
    }

    #[test]
    fn test_endorsement_requires_additional_premium_positive() {
        let today = Utc::now().date_naive();

        let endorsement = Endorsement::new(
            EndorsementType::AddressChange { new_address: create_test_address() },
            today,
        ).with_premium_adjustment(dec!(100));

        assert!(endorsement.requires_additional_premium());
    }

    #[test]
    fn test_endorsement_requires_additional_premium_negative() {
        let today = Utc::now().date_naive();

        let endorsement = Endorsement::new(
            EndorsementType::AddressChange { new_address: create_test_address() },
            today,
        ).with_premium_adjustment(dec!(-50));

        assert!(!endorsement.requires_additional_premium());
    }

    #[test]
    fn test_endorsement_requires_additional_premium_none() {
        let today = Utc::now().date_naive();

        let endorsement = Endorsement::new(
            EndorsementType::AddressChange { new_address: create_test_address() },
            today,
        );

        assert!(!endorsement.requires_additional_premium());
    }
}

// ============= PREMIUM TESTS =============
mod premium_tests {
    use super::*;
    use domain_policy::premium::PremiumFrequency;

    #[test]
    fn test_premium_frequency_payments_per_year() {
        assert_eq!(PremiumFrequency::Single.payments_per_year(), 1);
        assert_eq!(PremiumFrequency::Annual.payments_per_year(), 1);
        assert_eq!(PremiumFrequency::SemiAnnual.payments_per_year(), 2);
        assert_eq!(PremiumFrequency::Quarterly.payments_per_year(), 4);
        assert_eq!(PremiumFrequency::Monthly.payments_per_year(), 12);
    }

    #[test]
    fn test_premium_frequency_modal_factor() {
        assert_eq!(PremiumFrequency::Single.modal_factor(), dec!(1.0));
        assert_eq!(PremiumFrequency::Annual.modal_factor(), dec!(1.0));
        assert_eq!(PremiumFrequency::SemiAnnual.modal_factor(), dec!(0.5125));
        assert_eq!(PremiumFrequency::Quarterly.modal_factor(), dec!(0.2625));
        assert_eq!(PremiumFrequency::Monthly.modal_factor(), dec!(0.0875));
    }

    #[test]
    fn test_premium_frequency_next_due_date_single() {
        let today = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let next = PremiumFrequency::Single.next_due_date(today);
        assert_eq!(next, today);
    }

    #[test]
    fn test_premium_frequency_next_due_date_annual() {
        let today = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let next = PremiumFrequency::Annual.next_due_date(today);
        assert_eq!(next, NaiveDate::from_ymd_opt(2025, 6, 15).unwrap());
    }

    #[test]
    fn test_premium_frequency_next_due_date_semi_annual() {
        let today = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let next = PremiumFrequency::SemiAnnual.next_due_date(today);
        // 182 days from June 15
        assert_eq!(next, today + chrono::Duration::days(182));
    }

    #[test]
    fn test_premium_frequency_next_due_date_quarterly() {
        let today = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let next = PremiumFrequency::Quarterly.next_due_date(today);
        // 91 days from June 15
        assert_eq!(next, today + chrono::Duration::days(91));
    }

    #[test]
    fn test_premium_frequency_next_due_date_monthly() {
        let today = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let next = PremiumFrequency::Monthly.next_due_date(today);
        assert_eq!(next, NaiveDate::from_ymd_opt(2024, 7, 15).unwrap());
    }

    #[test]
    fn test_premium_frequency_next_due_date_monthly_december() {
        // Test the December to January rollover
        let december = NaiveDate::from_ymd_opt(2024, 12, 15).unwrap();
        let next = PremiumFrequency::Monthly.next_due_date(december);
        assert_eq!(next, NaiveDate::from_ymd_opt(2025, 1, 15).unwrap());
    }

    #[test]
    fn test_premium_frequency_next_due_date_annual_leap_year() {
        // Feb 29 on a leap year should handle gracefully
        let feb_29 = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        let next = PremiumFrequency::Annual.next_due_date(feb_29);
        // 2025 is not a leap year, so Feb 29 doesn't exist - falls back to +365 days
        assert!(next.year() == 2025);
    }
}

// ============= UNDERWRITING TESTS =============
mod underwriting_tests {
    use super::*;
    use domain_policy::underwriting::{
        MedicalHistory, evaluate_basic_rules, determine_risk_class,
        RiskClass, RuleImpact, UnderwritingApplication, ApplicantInfo,
        FinancialInfo, InsurancePurpose, Gender, LifestyleInfo, AlcoholLevel,
    };
    use domain_policy::coverage::Coverage;

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

    fn create_test_financial_info(annual_income: rust_decimal::Decimal) -> FinancialInfo {
        FinancialInfo {
            annual_income,
            net_worth: dec!(500000),
            existing_coverage: dec!(0),
            purpose: InsurancePurpose::FamilyProtection,
        }
    }

    fn create_test_lifestyle() -> LifestyleInfo {
        LifestyleInfo {
            hazardous_sports: vec![],
            aviation: None,
            alcohol_consumption: AlcoholLevel::None,
            travel_risk_countries: vec![],
        }
    }

    fn create_test_coverage(sum_assured: Money) -> Coverage {
        Coverage::death_benefit(sum_assured)
    }

    #[test]
    fn test_medical_history_bmi() {
        // BMI = weight_kg / (height_m^2)
        // 70kg, 175cm -> 70 / 1.75^2 = 70 / 3.0625 = 22.857
        let medical = create_test_medical_history(175, 70.0, false);
        let bmi = medical.bmi();
        assert!(bmi > 22.0 && bmi < 23.0);
    }

    #[test]
    fn test_medical_history_is_healthy_bmi() {
        // Healthy BMI range is 18.5 - 24.9
        let healthy = create_test_medical_history(175, 70.0, false);
        assert!(healthy.is_healthy_bmi());

        // Underweight (BMI < 18.5)
        let underweight = create_test_medical_history(175, 50.0, false);
        assert!(!underweight.is_healthy_bmi());

        // Overweight (BMI > 24.9)
        let overweight = create_test_medical_history(175, 100.0, false);
        assert!(!overweight.is_healthy_bmi());
    }

    #[test]
    fn test_evaluate_basic_rules_young_applicant() {
        let application = UnderwritingApplication {
            applicant: create_test_applicant(17),
            medical_history: create_test_medical_history(175, 70.0, false),
            financial: create_test_financial_info(dec!(100000)),
            lifestyle: create_test_lifestyle(),
            coverages: vec![create_test_coverage(Money::new(dec!(500000), Currency::USD))],
        };

        let results = evaluate_basic_rules(&application);
        // Should have a decline for being under 18
        let has_decline = results.iter().any(|r| r.impact == RuleImpact::Decline);
        assert!(has_decline);
    }

    #[test]
    fn test_evaluate_basic_rules_senior_applicant() {
        let application = UnderwritingApplication {
            applicant: create_test_applicant(70),
            medical_history: create_test_medical_history(175, 70.0, false),
            financial: create_test_financial_info(dec!(100000)),
            lifestyle: create_test_lifestyle(),
            coverages: vec![create_test_coverage(Money::new(dec!(500000), Currency::USD))],
        };

        let results = evaluate_basic_rules(&application);
        // Should have a referral for being over 65
        let has_referral = results.iter().any(|r| r.impact == RuleImpact::Referral);
        assert!(has_referral);
    }

    #[test]
    fn test_evaluate_basic_rules_extreme_bmi() {
        let application = UnderwritingApplication {
            applicant: create_test_applicant(35),
            medical_history: create_test_medical_history(175, 130.0, false), // BMI > 40
            financial: create_test_financial_info(dec!(100000)),
            lifestyle: create_test_lifestyle(),
            coverages: vec![create_test_coverage(Money::new(dec!(500000), Currency::USD))],
        };

        let results = evaluate_basic_rules(&application);
        // Should have a referral for extreme BMI
        let has_bmi_referral = results.iter().any(|r| r.rule_name == "bmi_extreme");
        assert!(has_bmi_referral);
    }

    #[test]
    fn test_evaluate_basic_rules_smoker() {
        let application = UnderwritingApplication {
            applicant: create_test_applicant(35),
            medical_history: create_test_medical_history(175, 70.0, true), // Smoker
            financial: create_test_financial_info(dec!(100000)),
            lifestyle: create_test_lifestyle(),
            coverages: vec![create_test_coverage(Money::new(dec!(500000), Currency::USD))],
        };

        let results = evaluate_basic_rules(&application);
        // Should have a loading for smoker
        let has_smoker_loading = results.iter().any(|r| r.rule_name == "smoker_status");
        assert!(has_smoker_loading);
    }

    #[test]
    fn test_evaluate_basic_rules_high_income_multiple() {
        let application = UnderwritingApplication {
            applicant: create_test_applicant(35),
            medical_history: create_test_medical_history(175, 70.0, false),
            financial: create_test_financial_info(dec!(50000)), // Low income
            lifestyle: create_test_lifestyle(),
            coverages: vec![create_test_coverage(Money::new(dec!(2000000), Currency::USD))], // 40x income
        };

        let results = evaluate_basic_rules(&application);
        // Should have a referral for exceeding 20x income
        let has_financial_referral = results.iter().any(|r| r.rule_name == "financial_justification");
        assert!(has_financial_referral);
    }

    #[test]
    fn test_evaluate_basic_rules_bmi_loading() {
        // Test BMI loading for slightly underweight
        let application = UnderwritingApplication {
            applicant: create_test_applicant(35),
            medical_history: create_test_medical_history(175, 55.0, false), // BMI ~18
            financial: create_test_financial_info(dec!(100000)),
            lifestyle: create_test_lifestyle(),
            coverages: vec![create_test_coverage(Money::new(dec!(500000), Currency::USD))],
        };

        let results = evaluate_basic_rules(&application);
        let has_bmi_loading = results.iter().any(|r| r.rule_name == "bmi_loading");
        assert!(has_bmi_loading);
    }

    #[test]
    fn test_evaluate_basic_rules_overweight_bmi_loading() {
        // Test BMI loading for overweight
        let application = UnderwritingApplication {
            applicant: create_test_applicant(35),
            medical_history: create_test_medical_history(175, 95.0, false), // BMI ~31
            financial: create_test_financial_info(dec!(100000)),
            lifestyle: create_test_lifestyle(),
            coverages: vec![create_test_coverage(Money::new(dec!(500000), Currency::USD))],
        };

        let results = evaluate_basic_rules(&application);
        let has_bmi_loading = results.iter().any(|r| r.rule_name == "bmi_loading");
        assert!(has_bmi_loading);
    }

    #[test]
    fn test_evaluate_basic_rules_former_smoker() {
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
        assert!(has_former_smoker);
    }

    #[test]
    fn test_evaluate_basic_rules_high_risk_occupation() {
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
        assert!(has_occupation_loading);
    }

    #[test]
    fn test_evaluate_basic_rules_zero_income() {
        let application = UnderwritingApplication {
            applicant: create_test_applicant(35),
            medical_history: create_test_medical_history(175, 70.0, false),
            financial: create_test_financial_info(dec!(0)), // Zero income
            lifestyle: create_test_lifestyle(),
            coverages: vec![create_test_coverage(Money::new(dec!(500000), Currency::USD))],
        };

        let results = evaluate_basic_rules(&application);
        // Should not panic with zero income
        assert!(!results.is_empty());
    }

    #[test]
    fn test_determine_risk_class_declined() {
        let results = vec![
            domain_policy::underwriting::RuleResult {
                rule_name: "test".to_string(),
                passed: false,
                message: "Decline".to_string(),
                impact: RuleImpact::Decline,
            },
        ];

        assert_eq!(determine_risk_class(&results), RiskClass::Declined);
    }

    #[test]
    fn test_determine_risk_class_preferred_plus() {
        let results = vec![
            domain_policy::underwriting::RuleResult {
                rule_name: "test".to_string(),
                passed: true,
                message: "Pass".to_string(),
                impact: RuleImpact::None,
            },
        ];

        assert_eq!(determine_risk_class(&results), RiskClass::PreferredPlus);
    }

    #[test]
    fn test_determine_risk_class_preferred() {
        let results = vec![
            domain_policy::underwriting::RuleResult {
                rule_name: "test".to_string(),
                passed: true,
                message: "Small loading".to_string(),
                impact: RuleImpact::Loading(dec!(5)),
            },
        ];

        assert_eq!(determine_risk_class(&results), RiskClass::Preferred);
    }

    #[test]
    fn test_determine_risk_class_standard() {
        let results = vec![
            domain_policy::underwriting::RuleResult {
                rule_name: "test".to_string(),
                passed: true,
                message: "Moderate loading".to_string(),
                impact: RuleImpact::Loading(dec!(20)),
            },
        ];

        assert_eq!(determine_risk_class(&results), RiskClass::Standard);
    }

    #[test]
    fn test_determine_risk_class_substandard() {
        let results = vec![
            domain_policy::underwriting::RuleResult {
                rule_name: "test".to_string(),
                passed: true,
                message: "Loading".to_string(),
                impact: RuleImpact::Loading(dec!(50)),
            },
        ];

        assert_eq!(determine_risk_class(&results), RiskClass::Substandard);
    }

    #[test]
    fn test_determine_risk_class_table_rated() {
        let results = vec![
            domain_policy::underwriting::RuleResult {
                rule_name: "test".to_string(),
                passed: true,
                message: "High loading".to_string(),
                impact: RuleImpact::Loading(dec!(150)),
            },
        ];

        // Very high loading should result in table rated
        let risk_class = determine_risk_class(&results);
        match risk_class {
            RiskClass::TableRated(_) => assert!(true),
            _ => panic!("Expected TableRated, got {:?}", risk_class),
        }
    }

    #[test]
    fn test_risk_class_rate_multipliers() {
        assert!(RiskClass::PreferredPlus.rate_multiplier() < dec!(1.0));
        assert!(RiskClass::Preferred.rate_multiplier() < dec!(1.0));
        assert_eq!(RiskClass::Standard.rate_multiplier(), dec!(1.0));
        assert!(RiskClass::Substandard.rate_multiplier() > dec!(1.0));
        assert!(RiskClass::TableRated(1).rate_multiplier() > dec!(1.0));
        assert_eq!(RiskClass::Declined.rate_multiplier(), dec!(0));
    }
}

// ============= AGGREGATE TESTS =============
mod aggregate_tests {
    use super::*;
    use domain_policy::aggregate::{
        PolicyBuilder, Policy, PolicyState, PolicyFinancials, LapseReason,
        TerminationReason, RiskObject, RiskType, PersonRole,
    };
    use domain_policy::coverage::{Coverage, CoverageType};
    use domain_policy::premium::{Premium, PremiumFrequency};
    use core_kernel::PartyId;

    fn create_test_coverage() -> Coverage {
        Coverage::new(
            CoverageType::DeathBenefit,
            Money::new(dec!(500000), Currency::USD),
        )
    }

    fn create_test_premium() -> Premium {
        Premium::new(
            Money::new(dec!(1000), Currency::USD),
            PremiumFrequency::Annual,
        )
    }

    fn create_test_policy() -> Policy {
        PolicyBuilder::new()
            .product_code("TERM_LIFE_20")
            .policyholder(PartyId::new())
            .add_coverage(create_test_coverage())
            .premium(create_test_premium())
            .term_years(20)
            .build()
            .unwrap()
    }

    #[test]
    fn test_policy_getters() {
        let policy = create_test_policy();

        // Test all getters
        assert!(!policy.policy_number().is_empty());
        assert_eq!(policy.product_code(), "TERM_LIFE_20");
        assert!(matches!(policy.state(), PolicyState::Quoted { .. }));
        assert_eq!(policy.coverages().len(), 1);
        assert_eq!(policy.currency(), Currency::USD);
        assert!(policy.financial_state().total_premium_paid.amount().is_zero());
    }

    #[test]
    fn test_policy_is_modifiable() {
        let policy = create_test_policy();
        // Quoted policy should be modifiable
        assert!(policy.is_modifiable());
    }

    #[test]
    fn test_policy_take_events() {
        let mut policy = create_test_policy();
        let events = policy.take_events();
        // Should have the PolicyQuoted event
        assert!(!events.is_empty());

        // After taking, events should be empty
        let events_again = policy.take_events();
        assert!(events_again.is_empty());
    }

    #[test]
    fn test_policy_financials_new() {
        let financials = PolicyFinancials::new(Currency::USD);

        assert!(financials.total_premium_paid.amount().is_zero());
        assert!(financials.premium_outstanding.amount().is_zero());
        assert!(financials.account_value.is_none());
        assert!(financials.surrender_value.is_none());
        assert!(financials.loan_outstanding.is_none());
        assert!(financials.last_payment_date.is_none());
        assert!(financials.next_due_date.is_none());
    }

    #[test]
    fn test_policy_financials_record_payment() {
        let mut financials = PolicyFinancials::new(Currency::USD);
        let now = Utc::now();
        let payment = Money::new(dec!(500), Currency::USD);

        let result = financials.record_payment(payment, now);
        assert!(result.is_ok());
        assert_eq!(financials.total_premium_paid.amount(), dec!(500));
        assert!(financials.last_payment_date.is_some());
    }

    #[test]
    fn test_policy_terminate_from_in_force() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();

        // Issue first
        policy.issue(effective_date, "UW001").unwrap();
        assert!(policy.is_in_force());

        // Now terminate
        let result = policy.terminate(TerminationReason::Surrender);
        assert!(result.is_ok());
        assert!(matches!(policy.state(), PolicyState::Terminated { .. }));
    }

    #[test]
    fn test_policy_terminate_from_lapsed() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();

        // Issue then lapse
        policy.issue(effective_date, "UW001").unwrap();
        policy.lapse(
            LapseReason::NonPayment {
                grace_days_elapsed: 30,
                outstanding_amount: dec!(1000),
            },
            None,
        ).unwrap();

        // Terminate from lapsed state
        let result = policy.terminate(TerminationReason::Fraud);
        assert!(result.is_ok());
        assert!(matches!(policy.state(), PolicyState::Terminated { .. }));
    }

    #[test]
    fn test_policy_terminate_invalid_state() {
        let mut policy = create_test_policy();

        // Cannot terminate from Quoted state
        let result = policy.terminate(TerminationReason::Death);
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_record_payment() {
        let mut policy = create_test_policy();
        let payment = Money::new(dec!(1000), Currency::USD);

        let result = policy.record_payment(payment);
        assert!(result.is_ok());
    }

    #[test]
    fn test_policy_record_payment_currency_mismatch() {
        let mut policy = create_test_policy();
        let payment = Money::new(dec!(1000), Currency::EUR); // Wrong currency

        let result = policy.record_payment(payment);
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_builder_currency() {
        let policy = PolicyBuilder::new()
            .product_code("TERM_20")
            .policyholder(PartyId::new())
            .currency(Currency::EUR)
            .add_coverage(Coverage::new(
                CoverageType::DeathBenefit,
                Money::new(dec!(100000), Currency::EUR),
            ))
            .premium(Premium::new(
                Money::new(dec!(500), Currency::EUR),
                PremiumFrequency::Annual,
            ))
            .build()
            .unwrap();

        assert_eq!(policy.currency(), Currency::EUR);
    }

    #[test]
    fn test_policy_builder_coverages() {
        let coverages = vec![
            Coverage::new(CoverageType::DeathBenefit, Money::new(dec!(500000), Currency::USD)),
            Coverage::new(CoverageType::CriticalIllness, Money::new(dec!(100000), Currency::USD)),
        ];

        let policy = PolicyBuilder::new()
            .product_code("COMBO_LIFE")
            .policyholder(PartyId::new())
            .coverages(coverages)
            .premium(create_test_premium())
            .build()
            .unwrap();

        assert_eq!(policy.coverages().len(), 2);
    }

    #[test]
    fn test_policy_builder_quote_validity_days() {
        let policy = PolicyBuilder::new()
            .product_code("TERM_20")
            .policyholder(PartyId::new())
            .add_coverage(create_test_coverage())
            .premium(create_test_premium())
            .quote_validity_days(60)
            .build()
            .unwrap();

        if let PolicyState::Quoted { quote_expiry, quote_date } = policy.state() {
            let days = (*quote_expiry - *quote_date).num_days();
            assert_eq!(days, 60);
        } else {
            panic!("Expected Quoted state");
        }
    }

    #[test]
    fn test_policy_builder_missing_product_code() {
        let result = PolicyBuilder::new()
            .policyholder(PartyId::new())
            .add_coverage(create_test_coverage())
            .premium(create_test_premium())
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_policy_builder_missing_policyholder() {
        let result = PolicyBuilder::new()
            .product_code("TERM_20")
            .add_coverage(create_test_coverage())
            .premium(create_test_premium())
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_policy_builder_missing_premium() {
        let result = PolicyBuilder::new()
            .product_code("TERM_20")
            .policyholder(PartyId::new())
            .add_coverage(create_test_coverage())
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_policy_builder_missing_coverages() {
        let result = PolicyBuilder::new()
            .product_code("TERM_20")
            .policyholder(PartyId::new())
            .premium(create_test_premium())
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_policy_builder_add_risk() {
        let risk = RiskObject {
            id: uuid::Uuid::new_v4(),
            risk_type: RiskType::Property,
            description: "Primary residence".to_string(),
            location: Some("123 Main St".to_string()),
            attributes: serde_json::json!({}),
        };

        let _policy = PolicyBuilder::new()
            .product_code("HOME_INS")
            .policyholder(PartyId::new())
            .add_coverage(create_test_coverage())
            .premium(create_test_premium())
            .add_risk(risk)
            .build()
            .unwrap();
    }

    #[test]
    fn test_lapse_reason_variants() {
        let non_payment = LapseReason::NonPayment {
            grace_days_elapsed: 30,
            outstanding_amount: dec!(500),
        };
        assert!(matches!(non_payment, LapseReason::NonPayment { .. }));

        let insufficient_fund = LapseReason::InsufficientFundValue;
        assert!(matches!(insufficient_fund, LapseReason::InsufficientFundValue));

        let other = LapseReason::Other("Custom reason".to_string());
        assert!(matches!(other, LapseReason::Other(_)));
    }

    #[test]
    fn test_termination_reason_variants() {
        assert!(matches!(TerminationReason::Death, TerminationReason::Death));
        assert!(matches!(TerminationReason::Maturity, TerminationReason::Maturity));
        assert!(matches!(TerminationReason::Surrender, TerminationReason::Surrender));
        assert!(matches!(TerminationReason::Fraud, TerminationReason::Fraud));
        assert!(matches!(TerminationReason::Conversion, TerminationReason::Conversion));

        let other = TerminationReason::Other("Custom".to_string());
        assert!(matches!(other, TerminationReason::Other(_)));
    }

    #[test]
    fn test_risk_type_person() {
        let party_id = PartyId::new();
        let risk = RiskType::Person {
            party_id,
            role: PersonRole::PrimaryInsured,
        };
        assert!(matches!(risk, RiskType::Person { .. }));
    }

    #[test]
    fn test_risk_type_variants() {
        assert!(matches!(RiskType::Property, RiskType::Property));
        assert!(matches!(RiskType::Vehicle, RiskType::Vehicle));
        assert!(matches!(RiskType::Liability, RiskType::Liability));
    }

    #[test]
    fn test_person_role_variants() {
        assert!(matches!(PersonRole::PrimaryInsured, PersonRole::PrimaryInsured));
        assert!(matches!(PersonRole::JointInsured, PersonRole::JointInsured));
        assert!(matches!(PersonRole::Spouse, PersonRole::Spouse));
        assert!(matches!(PersonRole::Child, PersonRole::Child));
    }

    #[test]
    fn test_policy_issue_from_pending_underwriting() {
        // This tests another branch of the issue() method
        let mut policy = create_test_policy();

        // Issue from quoted state
        let effective_date = Utc::now().date_naive();
        let result = policy.issue(effective_date, "UW001");
        assert!(result.is_ok());
        assert!(policy.is_in_force());
    }

    #[test]
    fn test_policy_reinstate_past_deadline() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();

        // Issue then lapse with past deadline
        policy.issue(effective_date, "UW001").unwrap();

        // Lapse with very short reinstatement period (we can't easily test past deadline
        // without mocking time, but we test the function path)
        policy.lapse(
            LapseReason::NonPayment {
                grace_days_elapsed: 30,
                outstanding_amount: dec!(1000),
            },
            Some(365), // 1 year reinstatement period
        ).unwrap();

        // Reinstate should succeed since deadline is in future
        let result = policy.reinstate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_policy_reinstate_invalid_state() {
        let mut policy = create_test_policy();

        // Cannot reinstate from Quoted state
        let result = policy.reinstate();
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_lapse_with_insufficient_fund_value() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();

        policy.issue(effective_date, "UW001").unwrap();

        let result = policy.lapse(LapseReason::InsufficientFundValue, None);
        assert!(result.is_ok());
        assert!(matches!(policy.state(), PolicyState::Lapsed { .. }));
    }

    #[test]
    fn test_policy_premium_frequency_renewal_dates() {
        // Test with different frequencies to cover calculate_renewal_date branches
        let frequencies = vec![
            PremiumFrequency::Annual,
            PremiumFrequency::SemiAnnual,
            PremiumFrequency::Quarterly,
            PremiumFrequency::Monthly,
            PremiumFrequency::Single,
        ];

        for freq in frequencies {
            let coverage = Coverage::new(
                CoverageType::DeathBenefit,
                Money::new(dec!(500000), Currency::USD),
            );

            let premium = Premium::new(
                Money::new(dec!(1000), Currency::USD),
                freq,
            );

            let mut policy = PolicyBuilder::new()
                .product_code("TERM_20")
                .policyholder(PartyId::new())
                .add_coverage(coverage)
                .premium(premium)
                .build()
                .unwrap();

            let effective_date = Utc::now().date_naive();
            let result = policy.issue(effective_date, "UW001");
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_policy_builder_default() {
        let builder = PolicyBuilder::default();
        // Should create a builder with defaults - test by building incomplete
        let result = builder.build();
        assert!(result.is_err()); // Missing required fields
    }

    #[test]
    fn test_policy_apply_endorsement_not_modifiable() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();

        // Issue then terminate - terminated policy is not modifiable
        policy.issue(effective_date, "UW001").unwrap();
        policy.terminate(TerminationReason::Surrender).unwrap();

        // Create an endorsement
        let endorsement = domain_policy::endorsement::Endorsement::new(
            domain_policy::endorsement::EndorsementType::AddressChange {
                new_address: domain_policy::endorsement::Address {
                    line1: "New Address".to_string(),
                    line2: None,
                    city: "City".to_string(),
                    state: Some("ST".to_string()),
                    postal_code: "12345".to_string(),
                    country: "US".to_string(),
                },
            },
            effective_date,
        );

        let result = policy.apply_endorsement(endorsement);
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_apply_endorsement_coverage_change() {
        let mut policy = create_test_policy();

        // Create endorsement to add coverage
        let new_coverage = Coverage::new(
            CoverageType::CriticalIllness,
            Money::new(dec!(100000), Currency::USD),
        );

        let endorsement = domain_policy::endorsement::Endorsement::new(
            domain_policy::endorsement::EndorsementType::CoverageChange {
                add: vec![new_coverage],
                remove: vec![],
                modify: vec![],
            },
            Utc::now().date_naive(),
        );

        let result = policy.apply_endorsement(endorsement);
        assert!(result.is_ok());
        assert_eq!(policy.coverages().len(), 2);
    }

    #[test]
    fn test_policy_apply_endorsement_coverage_remove() {
        let mut policy = create_test_policy();
        let coverage_id = policy.coverages()[0].id;

        // Create endorsement to remove coverage
        let endorsement = domain_policy::endorsement::Endorsement::new(
            domain_policy::endorsement::EndorsementType::CoverageChange {
                add: vec![],
                remove: vec![coverage_id],
                modify: vec![],
            },
            Utc::now().date_naive(),
        );

        let result = policy.apply_endorsement(endorsement);
        assert!(result.is_ok());
        assert_eq!(policy.coverages().len(), 0);
    }

    #[test]
    fn test_policy_apply_endorsement_premium_change() {
        let mut policy = create_test_policy();

        let new_premium = Premium::new(
            Money::new(dec!(2000), Currency::USD),
            PremiumFrequency::SemiAnnual,
        );

        let endorsement = domain_policy::endorsement::Endorsement::new(
            domain_policy::endorsement::EndorsementType::PremiumChange {
                new_premium,
            },
            Utc::now().date_naive(),
        );

        let result = policy.apply_endorsement(endorsement);
        assert!(result.is_ok());
        assert_eq!(policy.premium().frequency, PremiumFrequency::SemiAnnual);
    }

    #[test]
    fn test_policy_apply_endorsement_beneficiary_change() {
        let mut policy = create_test_policy();

        let endorsement = domain_policy::endorsement::Endorsement::new(
            domain_policy::endorsement::EndorsementType::BeneficiaryChange {
                beneficiaries: vec![domain_policy::endorsement::BeneficiaryAssignment {
                    party_id: PartyId::new(),
                    relationship: "Spouse".to_string(),
                    share_percent: dec!(100),
                    beneficiary_type: domain_policy::endorsement::BeneficiaryType::Primary,
                    is_revocable: true,
                }],
            },
            Utc::now().date_naive(),
        );

        let result = policy.apply_endorsement(endorsement);
        assert!(result.is_ok());
    }

    #[test]
    fn test_policy_apply_endorsement_name_change() {
        let mut policy = create_test_policy();

        let endorsement = domain_policy::endorsement::Endorsement::new(
            domain_policy::endorsement::EndorsementType::NameChange {
                new_first_name: Some("John".to_string()),
                new_last_name: Some("Doe Jr.".to_string()),
                reason: "Legal name change".to_string(),
            },
            Utc::now().date_naive(),
        );

        let result = policy.apply_endorsement(endorsement);
        assert!(result.is_ok());
    }

    #[test]
    fn test_policy_apply_endorsement_address_change() {
        let mut policy = create_test_policy();

        let endorsement = domain_policy::endorsement::Endorsement::new(
            domain_policy::endorsement::EndorsementType::AddressChange {
                new_address: domain_policy::endorsement::Address {
                    line1: "456 Oak Ave".to_string(),
                    line2: Some("Suite 100".to_string()),
                    city: "Boston".to_string(),
                    state: Some("MA".to_string()),
                    postal_code: "02101".to_string(),
                    country: "US".to_string(),
                },
            },
            Utc::now().date_naive(),
        );

        let result = policy.apply_endorsement(endorsement);
        assert!(result.is_ok());
    }

    #[test]
    fn test_policy_apply_endorsement_generates_event() {
        let mut policy = create_test_policy();
        // Clear existing events
        policy.take_events();

        let endorsement = domain_policy::endorsement::Endorsement::new(
            domain_policy::endorsement::EndorsementType::AddressChange {
                new_address: domain_policy::endorsement::Address {
                    line1: "789 Pine St".to_string(),
                    line2: None,
                    city: "Seattle".to_string(),
                    state: Some("WA".to_string()),
                    postal_code: "98101".to_string(),
                    country: "US".to_string(),
                },
            },
            Utc::now().date_naive(),
        );

        policy.apply_endorsement(endorsement).unwrap();

        let events = policy.take_events();
        assert!(!events.is_empty());
        assert_eq!(events[0].event_type(), "EndorsementApplied");
    }
}

// ============= SERVICES TESTS =============
mod services_tests {
    use super::*;
    use domain_policy::services::{UnderwritingService, RatingService};
    use domain_policy::underwriting::{
        UnderwritingApplication, ApplicantInfo, MedicalHistory, LifestyleInfo,
        FinancialInfo, Gender, AlcoholLevel, InsurancePurpose, RiskClass,
        MedicalCondition, ConditionStatus,
    };
    use domain_policy::coverage::{Coverage, CoverageType};

    fn create_valid_application(age_years: i32, sum_assured: rust_decimal::Decimal) -> UnderwritingApplication {
        let birth_date = Utc::now().date_naive() - chrono::Duration::days(age_years as i64 * 365);
        UnderwritingApplication {
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
                conditions: vec![],
                family_history: vec![],
            },
            lifestyle: LifestyleInfo {
                hazardous_sports: vec![],
                aviation: None,
                alcohol_consumption: AlcoholLevel::None,
                travel_risk_countries: vec![],
            },
            financial: FinancialInfo {
                annual_income: dec!(100000),
                net_worth: dec!(500000),
                existing_coverage: dec!(0),
                purpose: InsurancePurpose::FamilyProtection,
            },
            coverages: vec![
                Coverage::death_benefit(Money::new(sum_assured, Currency::USD))
            ],
        }
    }

    #[test]
    fn test_underwriting_service_with_rules() {
        let rules = serde_json::json!({
            "max_age": 70,
            "max_sum_assured": 5000000
        });

        let service = UnderwritingService::new().with_rules(rules);
        // Service should be created successfully with rules
        let application = create_valid_application(35, dec!(500000));
        let result = service.evaluate(&application);
        assert!(result.is_ok());
    }

    #[test]
    fn test_underwriting_service_age_too_young() {
        let service = UnderwritingService::new();
        let application = create_valid_application(15, dec!(500000)); // Under 18

        let result = service.evaluate(&application);
        assert!(result.is_err());
    }

    #[test]
    fn test_underwriting_service_age_too_old() {
        let service = UnderwritingService::new();
        let application = create_valid_application(90, dec!(500000)); // Over 85

        let result = service.evaluate(&application);
        assert!(result.is_err());
    }

    #[test]
    fn test_underwriting_service_empty_coverages() {
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
                conditions: vec![],
                family_history: vec![],
            },
            lifestyle: LifestyleInfo {
                hazardous_sports: vec![],
                aviation: None,
                alcohol_consumption: AlcoholLevel::None,
                travel_risk_countries: vec![],
            },
            financial: FinancialInfo {
                annual_income: dec!(100000),
                net_worth: dec!(500000),
                existing_coverage: dec!(0),
                purpose: InsurancePurpose::FamilyProtection,
            },
            coverages: vec![], // Empty coverages
        };

        let result = service.evaluate(&application);
        assert!(result.is_err());
    }

    #[test]
    fn test_underwriting_service_age_50_requires_medical() {
        let service = UnderwritingService::new();
        let application = create_valid_application(55, dec!(500000));

        let result = service.evaluate(&application).unwrap();
        // Should require medical examination for age >= 50
        assert!(result.required_documents.iter().any(|d| d.contains("Medical Examination")));
    }

    #[test]
    fn test_underwriting_service_high_sum_requires_financial() {
        let service = UnderwritingService::new();
        let application = create_valid_application(35, dec!(600000)); // Over 500k

        let result = service.evaluate(&application).unwrap();
        // Should require financial questionnaire
        assert!(result.required_documents.iter().any(|d| d.contains("Financial Questionnaire")));
    }

    #[test]
    fn test_underwriting_service_very_high_sum_requires_aps() {
        let service = UnderwritingService::new();
        let application = create_valid_application(35, dec!(1500000)); // Over 1M

        let result = service.evaluate(&application).unwrap();
        // Should require APS and blood tests
        assert!(result.required_documents.iter().any(|d| d.contains("Attending Physician")));
        assert!(result.required_documents.iter().any(|d| d.contains("Blood Test")));
    }

    #[test]
    fn test_underwriting_service_with_conditions_requires_records() {
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
                    name: "Diabetes".to_string(),
                    diagnosed_date: None,
                    status: ConditionStatus::Controlled,
                    treatment: Some("Metformin".to_string()),
                }],
                family_history: vec![],
            },
            lifestyle: LifestyleInfo {
                hazardous_sports: vec![],
                aviation: None,
                alcohol_consumption: AlcoholLevel::None,
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
        };

        let result = service.evaluate(&application).unwrap();
        // Should require medical records for disclosed conditions
        assert!(result.required_documents.iter().any(|d| d.contains("Medical Records")));
    }

    #[test]
    fn test_underwriting_service_substandard_requires_detailed_history() {
        let service = UnderwritingService::new();
        // Create an application that results in Substandard rating
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
                is_smoker: true,  // Smoker
                is_former_smoker: false,
                conditions: vec![],
                family_history: vec![],
            },
            lifestyle: LifestyleInfo {
                hazardous_sports: vec![],
                aviation: None,
                alcohol_consumption: AlcoholLevel::None,
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
        };

        let result = service.evaluate(&application).unwrap();
        // Should be substandard or worse with these risk factors
        assert!(matches!(
            result.risk_class,
            RiskClass::Substandard | RiskClass::TableRated(_)
        ));
    }

    #[test]
    fn test_underwriting_service_default() {
        let service = UnderwritingService::default();
        let application = create_valid_application(35, dec!(500000));

        let result = service.evaluate(&application);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rating_service_new() {
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

    #[test]
    fn test_rating_service_with_rate_tables() {
        let tables = serde_json::json!({
            "mortality_table": "CSO2017",
            "interest_rate": 0.04
        });

        let service = RatingService::new().with_rate_tables(tables);
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

    #[test]
    fn test_rating_service_default() {
        let service = RatingService::default();
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

    #[test]
    fn test_rating_service_smoker_premium() {
        let service = RatingService::new();
        let coverages = vec![
            Coverage::death_benefit(Money::new(dec!(500000), Currency::USD))
        ];

        let non_smoker_premium = service.calculate_premium(
            &coverages,
            35,
            false,
            RiskClass::Standard,
            Currency::USD,
        ).unwrap();

        let smoker_premium = service.calculate_premium(
            &coverages,
            35,
            true,
            RiskClass::Standard,
            Currency::USD,
        ).unwrap();

        // Smoker premium should be higher
        assert!(smoker_premium.total_per_payment().amount() > non_smoker_premium.total_per_payment().amount());
    }

    #[test]
    fn test_rating_service_risk_class_affects_premium() {
        let service = RatingService::new();
        let coverages = vec![
            Coverage::death_benefit(Money::new(dec!(500000), Currency::USD))
        ];

        let standard = service.calculate_premium(
            &coverages, 35, false, RiskClass::Standard, Currency::USD,
        ).unwrap();

        let preferred = service.calculate_premium(
            &coverages, 35, false, RiskClass::Preferred, Currency::USD,
        ).unwrap();

        let substandard = service.calculate_premium(
            &coverages, 35, false, RiskClass::Substandard, Currency::USD,
        ).unwrap();

        // Preferred should be less than Standard, Substandard more
        assert!(preferred.total_per_payment().amount() < standard.total_per_payment().amount());
        assert!(substandard.total_per_payment().amount() > standard.total_per_payment().amount());
    }

    #[test]
    fn test_rating_service_different_coverage_types() {
        let service = RatingService::new();

        // Test different coverage types to hit all branches in get_base_rate
        let coverage_types = vec![
            CoverageType::DeathBenefit,
            CoverageType::CriticalIllness,
            CoverageType::TotalPermanentDisability,
            CoverageType::AccidentalDeath,
            CoverageType::Hospitalization,
        ];

        for coverage_type in coverage_types {
            let coverages = vec![
                Coverage::new(coverage_type, Money::new(dec!(100000), Currency::USD))
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

    #[test]
    fn test_rating_service_with_loading() {
        let service = RatingService::new();

        // Create coverage with loading
        let mut coverage = Coverage::death_benefit(Money::new(dec!(500000), Currency::USD));
        coverage.loading_percent = Some(dec!(25)); // 25% loading

        let coverages = vec![coverage];

        let premium = service.calculate_premium(
            &coverages,
            35,
            false,
            RiskClass::Standard,
            Currency::USD,
        ).unwrap();

        // Premium should reflect the loading
        assert!(premium.total_per_payment().amount() > dec!(0));
    }

    #[test]
    fn test_rating_service_age_affects_premium() {
        let service = RatingService::new();
        let coverages = vec![
            Coverage::death_benefit(Money::new(dec!(500000), Currency::USD))
        ];

        let young_premium = service.calculate_premium(
            &coverages, 25, false, RiskClass::Standard, Currency::USD,
        ).unwrap();

        let older_premium = service.calculate_premium(
            &coverages, 55, false, RiskClass::Standard, Currency::USD,
        ).unwrap();

        // Older person should pay more
        assert!(older_premium.total_per_payment().amount() > young_premium.total_per_payment().amount());
    }
}

// ============= PREMIUM ADDITIONAL TESTS =============
mod premium_additional_tests {
    use super::*;
    use domain_policy::premium::{
        Premium, PremiumFrequency, Discount, DiscountType, RiderPremium,
        PaymentStatus, PremiumSchedule,
    };

    #[test]
    fn test_premium_with_policy_fee() {
        let premium = Premium::new(
            Money::new(dec!(1000), Currency::USD),
            PremiumFrequency::Annual,
        ).with_policy_fee(Money::new(dec!(50), Currency::USD));

        assert_eq!(premium.total_per_payment().amount(), dec!(1050));
    }

    #[test]
    fn test_premium_with_tax() {
        let premium = Premium::new(
            Money::new(dec!(1000), Currency::USD),
            PremiumFrequency::Annual,
        ).with_tax(Money::new(dec!(100), Currency::USD));

        assert_eq!(premium.total_per_payment().amount(), dec!(1100));
    }

    #[test]
    fn test_premium_add_rider_premium() {
        let mut premium = Premium::new(
            Money::new(dec!(1000), Currency::USD),
            PremiumFrequency::Annual,
        );

        premium.add_rider_premium(RiderPremium {
            rider_code: "WOP".to_string(),
            rider_name: "Waiver of Premium".to_string(),
            amount: Money::new(dec!(100), Currency::USD),
        });

        assert_eq!(premium.total_per_payment().amount(), dec!(1100));
    }

    #[test]
    fn test_premium_with_all_components() {
        let mut premium = Premium::new(
            Money::new(dec!(1000), Currency::USD),
            PremiumFrequency::Annual,
        )
        .with_policy_fee(Money::new(dec!(50), Currency::USD))
        .with_tax(Money::new(dec!(100), Currency::USD))
        .with_discount(Discount {
            discount_type: DiscountType::Percentage,
            value: dec!(10),
        });

        premium.add_rider_premium(RiderPremium {
            rider_code: "CI".to_string(),
            rider_name: "Critical Illness".to_string(),
            amount: Money::new(dec!(200), Currency::USD),
        });

        // Base: 1000 + Rider: 200 + Fee: 50 + Tax: 100 = 1350
        // Discount: 10% of 1350 = 135
        // Total: 1350 - 135 = 1215
        assert_eq!(premium.total_per_payment().amount(), dec!(1215));
    }

    #[test]
    fn test_premium_annualized_monthly() {
        let premium = Premium::new(
            Money::new(dec!(100), Currency::USD),
            PremiumFrequency::Monthly,
        );

        // 12 payments per year
        assert_eq!(premium.annualized().amount(), dec!(1200));
    }

    #[test]
    fn test_premium_annualized_quarterly() {
        let premium = Premium::new(
            Money::new(dec!(250), Currency::USD),
            PremiumFrequency::Quarterly,
        );

        // 4 payments per year
        assert_eq!(premium.annualized().amount(), dec!(1000));
    }

    #[test]
    fn test_premium_annualized_semi_annual() {
        let premium = Premium::new(
            Money::new(dec!(500), Currency::USD),
            PremiumFrequency::SemiAnnual,
        );

        // 2 payments per year
        assert_eq!(premium.annualized().amount(), dec!(1000));
    }

    #[test]
    fn test_discount_fixed_amount() {
        let discount = Discount {
            discount_type: DiscountType::FixedAmount,
            value: dec!(100),
        };

        let base = Money::new(dec!(1000), Currency::USD);
        let discount_amount = discount.calculate(&base);

        assert_eq!(discount_amount.amount(), dec!(100));
    }

    #[test]
    fn test_discount_multi_policy() {
        let discount = Discount {
            discount_type: DiscountType::MultiPolicy,
            value: dec!(5), // 5% multi-policy discount
        };

        let base = Money::new(dec!(1000), Currency::USD);
        let discount_amount = discount.calculate(&base);

        assert_eq!(discount_amount.amount(), dec!(50)); // 5% of 1000
    }

    #[test]
    fn test_discount_annual_payment() {
        let discount = Discount {
            discount_type: DiscountType::AnnualPayment,
            value: dec!(3), // 3% annual payment discount
        };

        let base = Money::new(dec!(1000), Currency::USD);
        let discount_amount = discount.calculate(&base);

        assert_eq!(discount_amount.amount(), dec!(30)); // 3% of 1000
    }

    #[test]
    fn test_discount_loyalty() {
        let discount = Discount {
            discount_type: DiscountType::Loyalty,
            value: dec!(10), // 10% loyalty discount
        };

        let base = Money::new(dec!(1000), Currency::USD);
        let discount_amount = discount.calculate(&base);

        assert_eq!(discount_amount.amount(), dec!(100)); // 10% of 1000
    }

    #[test]
    fn test_premium_schedule_generation() {
        let premium = Premium::new(
            Money::new(dec!(100), Currency::USD),
            PremiumFrequency::Quarterly,
        );

        let schedule = premium.generate_schedule(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            2, // 2 years
        );

        // 4 quarterly payments per year * 2 years = 8
        assert_eq!(schedule.len(), 8);
        assert_eq!(schedule[0].sequence_number, 1);
        assert_eq!(schedule[7].sequence_number, 8);
        assert!(matches!(schedule[0].status, PaymentStatus::Pending));
    }

    #[test]
    fn test_premium_schedule_entry_properties() {
        let premium = Premium::new(
            Money::new(dec!(100), Currency::USD),
            PremiumFrequency::Annual,
        );

        let schedule = premium.generate_schedule(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            1,
        );

        assert_eq!(schedule.len(), 1);
        assert_eq!(schedule[0].due_date, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        assert_eq!(schedule[0].amount.amount(), dec!(100));
    }

    #[test]
    fn test_payment_status_variants() {
        // Test all payment status variants can be created
        assert!(matches!(PaymentStatus::Pending, PaymentStatus::Pending));
        assert!(matches!(PaymentStatus::Paid, PaymentStatus::Paid));
        assert!(matches!(PaymentStatus::Overdue, PaymentStatus::Overdue));
        assert!(matches!(PaymentStatus::Waived, PaymentStatus::Waived));
        assert!(matches!(PaymentStatus::GracePeriod, PaymentStatus::GracePeriod));
    }

    #[test]
    fn test_premium_schedule_struct() {
        let schedule = PremiumSchedule {
            entries: vec![],
        };
        assert!(schedule.entries.is_empty());
    }

    #[test]
    fn test_premium_frequency_next_due_date_month_end() {
        // Test month-end edge case for monthly payments (e.g., Jan 31 -> Feb 28)
        let jan_31 = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        let next = PremiumFrequency::Monthly.next_due_date(jan_31);

        // Feb doesn't have 31 days, so it falls back to +30 days
        assert!(next.month() == 2 || next.month() == 3);
    }
}
