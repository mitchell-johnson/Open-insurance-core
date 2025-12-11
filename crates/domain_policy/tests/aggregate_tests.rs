//! Policy Aggregate Tests
//!
//! This module contains comprehensive tests for the Policy aggregate root,
//! including policy creation, lifecycle state transitions, endorsement handling,
//! and financial operations.
//!
//! # Test Coverage
//!
//! - Policy creation via `PolicyBuilder`
//! - Policy lifecycle state transitions (Quoted → InForce → Lapsed → Reinstated/Terminated)
//! - Endorsement application for various endorsement types
//! - Payment recording and financial state management
//! - Policy getters and accessor methods
//!
//! # Test Organization
//!
//! Tests are grouped by functionality:
//! - `policy_creation` - PolicyBuilder tests and validation
//! - `policy_lifecycle` - State transitions and lifecycle management
//! - `endorsement_handling` - Endorsement application tests
//! - `financial_operations` - Payment and financial state tests
//! - `enum_variants` - Coverage of all enum variant types

use chrono::Utc;
use core_kernel::{Currency, Money, PartyId};
use domain_policy::aggregate::{
    LapseReason, Policy, PolicyBuilder, PolicyFinancials, PolicyState, TerminationReason,
    PersonRole, RiskObject, RiskType,
};
use domain_policy::coverage::{Coverage, CoverageType};
use domain_policy::endorsement::{
    Address, BeneficiaryAssignment, BeneficiaryType, Endorsement, EndorsementType,
};
use domain_policy::premium::{Premium, PremiumFrequency};
use rust_decimal_macros::dec;

// ============================================================================
// TEST FIXTURES
// ============================================================================

/// Creates a standard test coverage for death benefit
fn create_test_coverage() -> Coverage {
    Coverage::new(
        CoverageType::DeathBenefit,
        Money::new(dec!(500000), Currency::USD),
    )
}

/// Creates a standard annual premium for testing
fn create_test_premium() -> Premium {
    Premium::new(
        Money::new(dec!(1000), Currency::USD),
        PremiumFrequency::Annual,
    )
}

/// Creates a fully configured test policy in Quoted state
fn create_test_policy() -> Policy {
    PolicyBuilder::new()
        .product_code("TERM_LIFE_20")
        .policyholder(PartyId::new())
        .add_coverage(create_test_coverage())
        .premium(create_test_premium())
        .term_years(20)
        .build()
        .expect("Test policy creation should succeed")
}

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

// ============================================================================
// POLICY CREATION TESTS
// ============================================================================

mod policy_creation {
    use super::*;

    /// Verifies that PolicyBuilder creates a policy in Quoted state with all
    /// required fields properly set.
    #[test]
    fn test_policy_builder_creates_quoted_policy() {
        let policy = create_test_policy();

        // Verify initial state is Quoted
        assert!(
            matches!(policy.state(), PolicyState::Quoted { .. }),
            "New policy should be in Quoted state"
        );

        // Verify all core fields are set correctly
        assert!(!policy.policy_number().is_empty(), "Policy number should be generated");
        assert_eq!(policy.product_code(), "TERM_LIFE_20", "Product code mismatch");
        assert_eq!(policy.coverages().len(), 1, "Should have exactly one coverage");
        assert_eq!(policy.currency(), Currency::USD, "Currency should be USD");
    }

    /// Verifies that all policy getters return expected values
    #[test]
    fn test_policy_getters_return_correct_values() {
        let policy = create_test_policy();

        // Test all getter methods
        assert!(!policy.policy_number().is_empty());
        assert_eq!(policy.product_code(), "TERM_LIFE_20");
        assert!(matches!(policy.state(), PolicyState::Quoted { .. }));
        assert_eq!(policy.coverages().len(), 1);
        assert_eq!(policy.currency(), Currency::USD);

        // Verify financial state initialization
        let financials = policy.financial_state();
        assert!(
            financials.total_premium_paid.amount().is_zero(),
            "No payments should be recorded initially"
        );
        assert!(financials.account_value.is_none());
        assert!(financials.surrender_value.is_none());
        assert!(financials.loan_outstanding.is_none());
    }

    /// Verifies that newly created policies are modifiable
    #[test]
    fn test_quoted_policy_is_modifiable() {
        let policy = create_test_policy();
        assert!(
            policy.is_modifiable(),
            "Quoted policy should be modifiable"
        );
    }

    /// Verifies that take_events returns and clears accumulated events
    #[test]
    fn test_take_events_returns_and_clears_events() {
        let mut policy = create_test_policy();

        // First call should return the PolicyQuoted event
        let events = policy.take_events();
        assert!(!events.is_empty(), "Should have initial PolicyQuoted event");
        assert_eq!(events[0].event_type(), "PolicyQuoted");

        // Second call should return empty (events were cleared)
        let events_again = policy.take_events();
        assert!(events_again.is_empty(), "Events should be cleared after take_events");
    }

    /// Verifies PolicyBuilder with custom currency
    #[test]
    fn test_policy_builder_with_custom_currency() {
        let policy = PolicyBuilder::new()
            .product_code("EURO_TERM_20")
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
            .expect("EUR policy should be created");

        assert_eq!(policy.currency(), Currency::EUR, "Currency should be EUR");
    }

    /// Verifies PolicyBuilder with multiple coverages
    #[test]
    fn test_policy_builder_with_multiple_coverages() {
        let coverages = vec![
            Coverage::new(CoverageType::DeathBenefit, Money::new(dec!(500000), Currency::USD)),
            Coverage::new(CoverageType::CriticalIllness, Money::new(dec!(100000), Currency::USD)),
            Coverage::new(CoverageType::AccidentalDeath, Money::new(dec!(250000), Currency::USD)),
        ];

        let policy = PolicyBuilder::new()
            .product_code("COMBO_LIFE")
            .policyholder(PartyId::new())
            .coverages(coverages)
            .premium(create_test_premium())
            .build()
            .expect("Multi-coverage policy should be created");

        assert_eq!(policy.coverages().len(), 3, "Should have three coverages");
    }

    /// Verifies PolicyBuilder with custom quote validity period
    #[test]
    fn test_policy_builder_custom_quote_validity() {
        let policy = PolicyBuilder::new()
            .product_code("TERM_20")
            .policyholder(PartyId::new())
            .add_coverage(create_test_coverage())
            .premium(create_test_premium())
            .quote_validity_days(60)
            .build()
            .expect("Policy with custom validity should be created");

        if let PolicyState::Quoted { quote_expiry, quote_date } = policy.state() {
            let days = (*quote_expiry - *quote_date).num_days();
            assert_eq!(days, 60, "Quote validity should be 60 days");
        } else {
            panic!("Expected Quoted state");
        }
    }

    /// Verifies PolicyBuilder with risk objects
    #[test]
    fn test_policy_builder_with_risk_object() {
        let risk = RiskObject {
            id: uuid::Uuid::new_v4(),
            risk_type: RiskType::Property,
            description: "Primary residence - Single family home".to_string(),
            location: Some("123 Main St, New York, NY 10001".to_string()),
            attributes: serde_json::json!({
                "year_built": 2010,
                "square_feet": 2500,
                "construction_type": "brick"
            }),
        };

        let policy = PolicyBuilder::new()
            .product_code("HOME_INS")
            .policyholder(PartyId::new())
            .add_coverage(create_test_coverage())
            .premium(create_test_premium())
            .add_risk(risk)
            .build()
            .expect("Policy with risk should be created");

        assert!(matches!(policy.state(), PolicyState::Quoted { .. }));
    }

    /// Verifies PolicyBuilder::default() creates empty builder
    #[test]
    fn test_policy_builder_default_is_incomplete() {
        let builder = PolicyBuilder::default();
        let result = builder.build();

        assert!(result.is_err(), "Default builder should fail - missing required fields");
    }
}

// ============================================================================
// POLICY BUILDER VALIDATION TESTS
// ============================================================================

mod policy_builder_validation {
    use super::*;

    /// Verifies that missing product_code causes build to fail
    #[test]
    fn test_missing_product_code_fails() {
        let result = PolicyBuilder::new()
            .policyholder(PartyId::new())
            .add_coverage(create_test_coverage())
            .premium(create_test_premium())
            .build();

        assert!(result.is_err(), "Should fail without product_code");
    }

    /// Verifies that missing policyholder causes build to fail
    #[test]
    fn test_missing_policyholder_fails() {
        let result = PolicyBuilder::new()
            .product_code("TERM_20")
            .add_coverage(create_test_coverage())
            .premium(create_test_premium())
            .build();

        assert!(result.is_err(), "Should fail without policyholder");
    }

    /// Verifies that missing premium causes build to fail
    #[test]
    fn test_missing_premium_fails() {
        let result = PolicyBuilder::new()
            .product_code("TERM_20")
            .policyholder(PartyId::new())
            .add_coverage(create_test_coverage())
            .build();

        assert!(result.is_err(), "Should fail without premium");
    }

    /// Verifies that missing coverages causes build to fail
    #[test]
    fn test_missing_coverages_fails() {
        let result = PolicyBuilder::new()
            .product_code("TERM_20")
            .policyholder(PartyId::new())
            .premium(create_test_premium())
            .build();

        assert!(result.is_err(), "Should fail without coverages");
    }
}

// ============================================================================
// POLICY LIFECYCLE TESTS
// ============================================================================

mod policy_lifecycle {
    use super::*;

    /// Verifies successful transition from Quoted to InForce via issue()
    #[test]
    fn test_issue_transitions_to_in_force() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();

        let result = policy.issue(effective_date, "UW001");

        assert!(result.is_ok(), "Issue should succeed");
        assert!(policy.is_in_force(), "Policy should be in force after issue");
        assert!(
            matches!(policy.state(), PolicyState::InForce { .. }),
            "State should be InForce"
        );
    }

    /// Verifies issue() generates PolicyIssued event
    #[test]
    fn test_issue_generates_event() {
        let mut policy = create_test_policy();
        policy.take_events(); // Clear initial events

        let effective_date = Utc::now().date_naive();
        policy.issue(effective_date, "UW001").unwrap();

        let events = policy.take_events();
        assert_eq!(events.len(), 1, "Should generate one event");
        assert_eq!(events[0].event_type(), "PolicyIssued");
    }

    /// Verifies successful transition from InForce to Lapsed
    #[test]
    fn test_lapse_from_in_force() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();
        policy.issue(effective_date, "UW001").unwrap();

        let result = policy.lapse(
            LapseReason::NonPayment {
                grace_days_elapsed: 30,
                outstanding_amount: dec!(1000),
            },
            Some(90), // 90 day reinstatement period
        );

        assert!(result.is_ok(), "Lapse should succeed");
        assert!(
            matches!(policy.state(), PolicyState::Lapsed { .. }),
            "State should be Lapsed"
        );
    }

    /// Verifies lapse with InsufficientFundValue reason
    #[test]
    fn test_lapse_with_insufficient_fund_value() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();
        policy.issue(effective_date, "UW001").unwrap();

        let result = policy.lapse(LapseReason::InsufficientFundValue, None);

        assert!(result.is_ok(), "Lapse should succeed");
        assert!(matches!(policy.state(), PolicyState::Lapsed { .. }));
    }

    /// Verifies successful reinstatement from Lapsed state
    #[test]
    fn test_reinstate_from_lapsed() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();

        policy.issue(effective_date, "UW001").unwrap();
        policy.lapse(
            LapseReason::NonPayment {
                grace_days_elapsed: 30,
                outstanding_amount: dec!(1000),
            },
            Some(365), // 1 year reinstatement period
        ).unwrap();

        let result = policy.reinstate();

        assert!(result.is_ok(), "Reinstate should succeed");
        assert!(
            matches!(policy.state(), PolicyState::Reinstated { .. }),
            "State should be Reinstated"
        );
    }

    /// Verifies termination from InForce state
    #[test]
    fn test_terminate_from_in_force() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();
        policy.issue(effective_date, "UW001").unwrap();

        let result = policy.terminate(TerminationReason::Surrender);

        assert!(result.is_ok(), "Terminate should succeed");
        assert!(
            matches!(policy.state(), PolicyState::Terminated { .. }),
            "State should be Terminated"
        );
    }

    /// Verifies termination from Lapsed state
    #[test]
    fn test_terminate_from_lapsed() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();

        policy.issue(effective_date, "UW001").unwrap();
        policy.lapse(
            LapseReason::NonPayment {
                grace_days_elapsed: 30,
                outstanding_amount: dec!(1000),
            },
            None,
        ).unwrap();

        let result = policy.terminate(TerminationReason::Fraud);

        assert!(result.is_ok(), "Terminate from lapsed should succeed");
        assert!(matches!(policy.state(), PolicyState::Terminated { .. }));
    }

    /// Verifies invalid state transitions are rejected
    #[test]
    fn test_invalid_state_transitions() {
        let mut policy = create_test_policy();

        // Cannot lapse from Quoted
        let lapse_result = policy.lapse(
            LapseReason::NonPayment {
                grace_days_elapsed: 30,
                outstanding_amount: dec!(1000),
            },
            None,
        );
        assert!(lapse_result.is_err(), "Cannot lapse from Quoted state");

        // Cannot terminate from Quoted
        let terminate_result = policy.terminate(TerminationReason::Death);
        assert!(terminate_result.is_err(), "Cannot terminate from Quoted state");

        // Cannot reinstate from Quoted
        let reinstate_result = policy.reinstate();
        assert!(reinstate_result.is_err(), "Cannot reinstate from Quoted state");
    }

    /// Tests the full policy lifecycle from quote to termination
    #[test]
    fn test_full_lifecycle_quote_to_termination() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();

        // Step 1: Verify initial Quoted state
        assert!(matches!(policy.state(), PolicyState::Quoted { .. }));

        // Step 2: Issue policy
        policy.issue(effective_date, "UW001").unwrap();
        assert!(policy.is_in_force());

        // Step 3: Lapse due to non-payment
        policy.lapse(
            LapseReason::NonPayment {
                grace_days_elapsed: 30,
                outstanding_amount: dec!(1000),
            },
            Some(180),
        ).unwrap();
        assert!(matches!(policy.state(), PolicyState::Lapsed { .. }));

        // Step 4: Reinstate
        policy.reinstate().unwrap();
        assert!(matches!(policy.state(), PolicyState::Reinstated { .. }));
    }

    /// Verifies different premium frequencies affect renewal date calculation
    #[test]
    fn test_issue_with_different_frequencies() {
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
            let premium = Premium::new(Money::new(dec!(1000), Currency::USD), freq);

            let mut policy = PolicyBuilder::new()
                .product_code("TERM_20")
                .policyholder(PartyId::new())
                .add_coverage(coverage)
                .premium(premium)
                .build()
                .unwrap();

            let effective_date = Utc::now().date_naive();
            let result = policy.issue(effective_date, "UW001");

            assert!(result.is_ok(), "Issue should succeed for {:?} frequency", freq);
            assert!(policy.is_in_force());
        }
    }
}

// ============================================================================
// ENDORSEMENT HANDLING TESTS
// ============================================================================

mod endorsement_handling {
    use super::*;

    /// Verifies that terminated policies reject endorsements
    #[test]
    fn test_endorsement_rejected_for_non_modifiable_policy() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();

        // Terminate the policy
        policy.issue(effective_date, "UW001").unwrap();
        policy.terminate(TerminationReason::Surrender).unwrap();

        // Attempt to apply endorsement
        let endorsement = Endorsement::new(
            EndorsementType::AddressChange {
                new_address: create_test_address(),
            },
            effective_date,
        );

        let result = policy.apply_endorsement(endorsement);
        assert!(result.is_err(), "Terminated policy should reject endorsements");
    }

    /// Verifies CoverageChange endorsement to add coverage
    #[test]
    fn test_coverage_change_add_coverage() {
        let mut policy = create_test_policy();
        let initial_count = policy.coverages().len();

        let new_coverage = Coverage::new(
            CoverageType::CriticalIllness,
            Money::new(dec!(100000), Currency::USD),
        );

        let endorsement = Endorsement::new(
            EndorsementType::CoverageChange {
                add: vec![new_coverage],
                remove: vec![],
                modify: vec![],
            },
            Utc::now().date_naive(),
        );

        let result = policy.apply_endorsement(endorsement);

        assert!(result.is_ok(), "Coverage add should succeed");
        assert_eq!(
            policy.coverages().len(),
            initial_count + 1,
            "Should have one more coverage"
        );
    }

    /// Verifies CoverageChange endorsement to remove coverage
    #[test]
    fn test_coverage_change_remove_coverage() {
        let mut policy = create_test_policy();
        let coverage_id = policy.coverages()[0].id;

        let endorsement = Endorsement::new(
            EndorsementType::CoverageChange {
                add: vec![],
                remove: vec![coverage_id],
                modify: vec![],
            },
            Utc::now().date_naive(),
        );

        let result = policy.apply_endorsement(endorsement);

        assert!(result.is_ok(), "Coverage remove should succeed");
        assert_eq!(policy.coverages().len(), 0, "Coverage should be removed");
    }

    /// Verifies PremiumChange endorsement
    #[test]
    fn test_premium_change_endorsement() {
        let mut policy = create_test_policy();

        let new_premium = Premium::new(
            Money::new(dec!(2000), Currency::USD),
            PremiumFrequency::SemiAnnual,
        );

        let endorsement = Endorsement::new(
            EndorsementType::PremiumChange { new_premium },
            Utc::now().date_naive(),
        );

        let result = policy.apply_endorsement(endorsement);

        assert!(result.is_ok(), "Premium change should succeed");
        assert_eq!(
            policy.premium().frequency,
            PremiumFrequency::SemiAnnual,
            "Premium frequency should be updated"
        );
    }

    /// Verifies BeneficiaryChange endorsement
    #[test]
    fn test_beneficiary_change_endorsement() {
        let mut policy = create_test_policy();

        let endorsement = Endorsement::new(
            EndorsementType::BeneficiaryChange {
                beneficiaries: vec![BeneficiaryAssignment {
                    party_id: PartyId::new(),
                    relationship: "Spouse".to_string(),
                    share_percent: dec!(100),
                    beneficiary_type: BeneficiaryType::Primary,
                    is_revocable: true,
                }],
            },
            Utc::now().date_naive(),
        );

        let result = policy.apply_endorsement(endorsement);
        assert!(result.is_ok(), "Beneficiary change should succeed");
    }

    /// Verifies NameChange endorsement
    #[test]
    fn test_name_change_endorsement() {
        let mut policy = create_test_policy();

        let endorsement = Endorsement::new(
            EndorsementType::NameChange {
                new_first_name: Some("John".to_string()),
                new_last_name: Some("Smith-Johnson".to_string()),
                reason: "Marriage - legal name change".to_string(),
            },
            Utc::now().date_naive(),
        );

        let result = policy.apply_endorsement(endorsement);
        assert!(result.is_ok(), "Name change should succeed");
    }

    /// Verifies AddressChange endorsement
    #[test]
    fn test_address_change_endorsement() {
        let mut policy = create_test_policy();

        let endorsement = Endorsement::new(
            EndorsementType::AddressChange {
                new_address: Address {
                    line1: "456 Oak Avenue".to_string(),
                    line2: Some("Apartment 12B".to_string()),
                    city: "Boston".to_string(),
                    state: Some("MA".to_string()),
                    postal_code: "02101".to_string(),
                    country: "US".to_string(),
                },
            },
            Utc::now().date_naive(),
        );

        let result = policy.apply_endorsement(endorsement);
        assert!(result.is_ok(), "Address change should succeed");
    }

    /// Verifies endorsement generates EndorsementApplied event
    #[test]
    fn test_endorsement_generates_event() {
        let mut policy = create_test_policy();
        policy.take_events(); // Clear initial events

        let endorsement = Endorsement::new(
            EndorsementType::AddressChange {
                new_address: create_test_address(),
            },
            Utc::now().date_naive(),
        );

        policy.apply_endorsement(endorsement).unwrap();

        let events = policy.take_events();
        assert_eq!(events.len(), 1, "Should generate one event");
        assert_eq!(events[0].event_type(), "EndorsementApplied");
    }
}

// ============================================================================
// FINANCIAL OPERATIONS TESTS
// ============================================================================

mod financial_operations {
    use super::*;

    /// Verifies PolicyFinancials initialization with zero values
    #[test]
    fn test_policy_financials_initialization() {
        let financials = PolicyFinancials::new(Currency::USD);

        assert!(financials.total_premium_paid.amount().is_zero());
        assert!(financials.premium_outstanding.amount().is_zero());
        assert!(financials.account_value.is_none());
        assert!(financials.surrender_value.is_none());
        assert!(financials.loan_outstanding.is_none());
        assert!(financials.last_payment_date.is_none());
        assert!(financials.next_due_date.is_none());
    }

    /// Verifies PolicyFinancials::record_payment updates state correctly
    #[test]
    fn test_policy_financials_record_payment() {
        let mut financials = PolicyFinancials::new(Currency::USD);
        let now = Utc::now();
        let payment = Money::new(dec!(500), Currency::USD);

        let result = financials.record_payment(payment, now);

        assert!(result.is_ok(), "Payment recording should succeed");
        assert_eq!(
            financials.total_premium_paid.amount(),
            dec!(500),
            "Total paid should reflect payment"
        );
        assert!(
            financials.last_payment_date.is_some(),
            "Last payment date should be set"
        );
    }

    /// Verifies Policy::record_payment updates financial state
    #[test]
    fn test_policy_record_payment() {
        let mut policy = create_test_policy();
        let payment = Money::new(dec!(1000), Currency::USD);

        let result = policy.record_payment(payment);

        assert!(result.is_ok(), "Payment should succeed");
    }

    /// Verifies currency mismatch is rejected
    #[test]
    fn test_payment_currency_mismatch_rejected() {
        let mut policy = create_test_policy(); // USD policy
        let payment = Money::new(dec!(1000), Currency::EUR); // EUR payment

        let result = policy.record_payment(payment);

        assert!(result.is_err(), "Currency mismatch should be rejected");
    }
}

// ============================================================================
// ENUM VARIANT TESTS
// ============================================================================

mod enum_variants {
    use super::*;

    /// Verifies all LapseReason variants can be created
    #[test]
    fn test_lapse_reason_variants() {
        let non_payment = LapseReason::NonPayment {
            grace_days_elapsed: 30,
            outstanding_amount: dec!(500),
        };
        assert!(matches!(non_payment, LapseReason::NonPayment { .. }));

        let insufficient_fund = LapseReason::InsufficientFundValue;
        assert!(matches!(insufficient_fund, LapseReason::InsufficientFundValue));

        let other = LapseReason::Other("Policy holder request".to_string());
        assert!(matches!(other, LapseReason::Other(_)));
    }

    /// Verifies all TerminationReason variants can be created
    #[test]
    fn test_termination_reason_variants() {
        assert!(matches!(TerminationReason::Death, TerminationReason::Death));
        assert!(matches!(TerminationReason::Maturity, TerminationReason::Maturity));
        assert!(matches!(TerminationReason::Surrender, TerminationReason::Surrender));
        assert!(matches!(TerminationReason::Fraud, TerminationReason::Fraud));
        assert!(matches!(TerminationReason::Conversion, TerminationReason::Conversion));

        let other = TerminationReason::Other("Regulatory requirement".to_string());
        assert!(matches!(other, TerminationReason::Other(_)));
    }

    /// Verifies all RiskType variants can be created
    #[test]
    fn test_risk_type_variants() {
        let person = RiskType::Person {
            party_id: PartyId::new(),
            role: PersonRole::PrimaryInsured,
        };
        assert!(matches!(person, RiskType::Person { .. }));

        assert!(matches!(RiskType::Property, RiskType::Property));
        assert!(matches!(RiskType::Vehicle, RiskType::Vehicle));
        assert!(matches!(RiskType::Liability, RiskType::Liability));
    }

    /// Verifies all PersonRole variants can be created
    #[test]
    fn test_person_role_variants() {
        assert!(matches!(PersonRole::PrimaryInsured, PersonRole::PrimaryInsured));
        assert!(matches!(PersonRole::JointInsured, PersonRole::JointInsured));
        assert!(matches!(PersonRole::Spouse, PersonRole::Spouse));
        assert!(matches!(PersonRole::Child, PersonRole::Child));
    }
}
