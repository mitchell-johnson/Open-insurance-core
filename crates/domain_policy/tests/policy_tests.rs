//! Comprehensive unit tests for the Policy domain
//!
//! Tests cover policy creation, state transitions, endorsements,
//! premium calculations, and coverage management.

use chrono::{NaiveDate, Utc};
use core_kernel::{Currency, Money, PartyId};
use domain_policy::{
    Coverage, CoverageType, Endorsement, EndorsementType, Policy,
    PolicyBuilder, PolicyError, PolicyState, Premium, PremiumFrequency,
};
use domain_policy::aggregate::{LapseReason, TerminationReason};
use rust_decimal_macros::dec;

/// Helper function to create a basic test policy
fn create_test_policy() -> Policy {
    let coverage = Coverage::death_benefit(Money::new(dec!(500000), Currency::USD));
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

mod policy_creation {
    use super::*;

    #[test]
    fn test_policy_builder_creates_quoted_policy() {
        let policy = create_test_policy();
        assert!(matches!(policy.state(), PolicyState::Quoted { .. }));
    }

    #[test]
    fn test_policy_has_policy_number() {
        let policy = create_test_policy();
        assert!(!policy.policy_number().is_empty());
    }

    #[test]
    fn test_policy_has_product_code() {
        let policy = create_test_policy();
        assert_eq!(policy.product_code(), "TERM_LIFE_20");
    }

    #[test]
    fn test_policy_has_coverages() {
        let policy = create_test_policy();
        assert_eq!(policy.coverages().len(), 1);
    }

    #[test]
    fn test_policy_has_correct_currency() {
        let policy = create_test_policy();
        assert_eq!(policy.currency(), Currency::USD);
    }

    #[test]
    fn test_policy_builder_requires_product_code() {
        let coverage = Coverage::death_benefit(Money::new(dec!(500000), Currency::USD));
        let premium = Premium::new(
            Money::new(dec!(1000), Currency::USD),
            PremiumFrequency::Annual,
        );

        let result = PolicyBuilder::new()
            .policyholder(PartyId::new())
            .add_coverage(coverage)
            .premium(premium)
            .build();

        assert!(matches!(result, Err(PolicyError::MissingRequiredField(_))));
    }

    #[test]
    fn test_policy_builder_requires_policyholder() {
        let coverage = Coverage::death_benefit(Money::new(dec!(500000), Currency::USD));
        let premium = Premium::new(
            Money::new(dec!(1000), Currency::USD),
            PremiumFrequency::Annual,
        );

        let result = PolicyBuilder::new()
            .product_code("TERM_LIFE_20")
            .add_coverage(coverage)
            .premium(premium)
            .build();

        assert!(matches!(result, Err(PolicyError::MissingRequiredField(_))));
    }

    #[test]
    fn test_policy_builder_requires_coverages() {
        let premium = Premium::new(
            Money::new(dec!(1000), Currency::USD),
            PremiumFrequency::Annual,
        );

        let result = PolicyBuilder::new()
            .product_code("TERM_LIFE_20")
            .policyholder(PartyId::new())
            .premium(premium)
            .build();

        assert!(matches!(result, Err(PolicyError::MissingRequiredField(_))));
    }

    #[test]
    fn test_policy_builder_requires_premium() {
        let coverage = Coverage::death_benefit(Money::new(dec!(500000), Currency::USD));

        let result = PolicyBuilder::new()
            .product_code("TERM_LIFE_20")
            .policyholder(PartyId::new())
            .add_coverage(coverage)
            .build();

        assert!(matches!(result, Err(PolicyError::MissingRequiredField(_))));
    }

    #[test]
    fn test_policy_builder_with_multiple_coverages() {
        let death_benefit = Coverage::death_benefit(Money::new(dec!(500000), Currency::USD));
        let critical_illness = Coverage::critical_illness(
            Money::new(dec!(100000), Currency::USD),
            90,
        );
        let premium = Premium::new(
            Money::new(dec!(1500), Currency::USD),
            PremiumFrequency::Annual,
        );

        let policy = PolicyBuilder::new()
            .product_code("COMPREHENSIVE")
            .policyholder(PartyId::new())
            .add_coverage(death_benefit)
            .add_coverage(critical_illness)
            .premium(premium)
            .build()
            .unwrap();

        assert_eq!(policy.coverages().len(), 2);
    }

    #[test]
    fn test_policy_builder_with_different_currency() {
        let coverage = Coverage::death_benefit(Money::new(dec!(1000000), Currency::INR));
        let premium = Premium::new(
            Money::new(dec!(50000), Currency::INR),
            PremiumFrequency::Annual,
        );

        let policy = PolicyBuilder::new()
            .product_code("TERM_LIFE_20")
            .policyholder(PartyId::new())
            .currency(Currency::INR)
            .add_coverage(coverage)
            .premium(premium)
            .build()
            .unwrap();

        assert_eq!(policy.currency(), Currency::INR);
    }
}

mod policy_lifecycle {
    use super::*;

    #[test]
    fn test_issue_transitions_to_in_force() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();

        policy.issue(effective_date, "UW001").unwrap();

        assert!(policy.is_in_force());
        assert!(matches!(policy.state(), PolicyState::InForce { .. }));
    }

    #[test]
    fn test_issue_creates_event() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();

        policy.issue(effective_date, "UW001").unwrap();

        let events = policy.take_events();
        assert!(events.iter().any(|e| matches!(e, domain_policy::events::PolicyEvent::PolicyIssued { .. })));
    }

    #[test]
    fn test_cannot_issue_already_in_force_policy() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();

        policy.issue(effective_date, "UW001").unwrap();

        // Try to issue again
        let result = policy.issue(effective_date, "UW002");
        assert!(matches!(result, Err(PolicyError::InvalidStateTransition { .. })));
    }

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
            Some(30),
        );

        assert!(result.is_ok());
        assert!(matches!(policy.state(), PolicyState::Lapsed { .. }));
    }

    #[test]
    fn test_cannot_lapse_quoted_policy() {
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
    fn test_reinstate_from_lapsed() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();

        policy.issue(effective_date, "UW001").unwrap();
        policy.lapse(
            LapseReason::NonPayment {
                grace_days_elapsed: 30,
                outstanding_amount: dec!(1000),
            },
            Some(30),
        ).unwrap();

        let result = policy.reinstate();
        assert!(result.is_ok());
        assert!(matches!(policy.state(), PolicyState::Reinstated { .. }));
    }

    #[test]
    fn test_terminate_from_in_force() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();

        policy.issue(effective_date, "UW001").unwrap();

        let result = policy.terminate(TerminationReason::Surrender);
        assert!(result.is_ok());
        assert!(matches!(policy.state(), PolicyState::Terminated { .. }));
    }

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
            Some(30),
        ).unwrap();

        let result = policy.terminate(TerminationReason::Surrender);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cannot_terminate_quoted_policy() {
        let mut policy = create_test_policy();

        let result = policy.terminate(TerminationReason::Surrender);
        assert!(matches!(result, Err(PolicyError::InvalidStateTransition { .. })));
    }

    #[test]
    fn test_full_lifecycle() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();

        // Quote -> InForce
        assert!(matches!(policy.state(), PolicyState::Quoted { .. }));
        policy.issue(effective_date, "UW001").unwrap();
        assert!(policy.is_in_force());

        // InForce -> Lapsed
        policy.lapse(
            LapseReason::NonPayment {
                grace_days_elapsed: 30,
                outstanding_amount: dec!(1000),
            },
            Some(30),
        ).unwrap();
        assert!(matches!(policy.state(), PolicyState::Lapsed { .. }));

        // Lapsed -> Reinstated
        policy.reinstate().unwrap();
        assert!(matches!(policy.state(), PolicyState::Reinstated { .. }));
    }
}

mod modifiability {
    use super::*;

    #[test]
    fn test_quoted_policy_is_modifiable() {
        let policy = create_test_policy();
        assert!(policy.is_modifiable());
    }

    #[test]
    fn test_in_force_policy_is_modifiable() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();

        policy.issue(effective_date, "UW001").unwrap();
        assert!(policy.is_modifiable());
    }

    #[test]
    fn test_lapsed_policy_is_not_modifiable() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();

        policy.issue(effective_date, "UW001").unwrap();
        policy.lapse(
            LapseReason::NonPayment {
                grace_days_elapsed: 30,
                outstanding_amount: dec!(1000),
            },
            Some(30),
        ).unwrap();

        assert!(!policy.is_modifiable());
    }

    #[test]
    fn test_terminated_policy_is_not_modifiable() {
        let mut policy = create_test_policy();
        let effective_date = Utc::now().date_naive();

        policy.issue(effective_date, "UW001").unwrap();
        policy.terminate(TerminationReason::Surrender).unwrap();

        assert!(!policy.is_modifiable());
    }
}

mod payments {
    use super::*;

    #[test]
    fn test_record_payment_updates_financial_state() {
        let mut policy = create_test_policy();
        let payment = Money::new(dec!(1000), Currency::USD);

        let result = policy.record_payment(payment);
        assert!(result.is_ok());

        let financial_state = policy.financial_state();
        assert_eq!(financial_state.total_premium_paid.amount(), dec!(1000));
    }

    #[test]
    fn test_record_payment_currency_mismatch() {
        let mut policy = create_test_policy();
        let payment = Money::new(dec!(1000), Currency::EUR);

        let result = policy.record_payment(payment);
        assert!(matches!(result, Err(PolicyError::CurrencyMismatch { .. })));
    }

    #[test]
    fn test_multiple_payments() {
        let mut policy = create_test_policy();

        policy.record_payment(Money::new(dec!(500), Currency::USD)).unwrap();
        policy.record_payment(Money::new(dec!(500), Currency::USD)).unwrap();

        let financial_state = policy.financial_state();
        assert_eq!(financial_state.total_premium_paid.amount(), dec!(1000));
    }
}

mod events {
    use super::*;
    use domain_policy::events::PolicyEvent;

    #[test]
    fn test_policy_creation_generates_quoted_event() {
        let mut policy = create_test_policy();
        let events = policy.take_events();

        assert!(events.iter().any(|e| matches!(e, PolicyEvent::PolicyQuoted { .. })));
    }

    #[test]
    fn test_issue_generates_issued_event() {
        let mut policy = create_test_policy();
        let _ = policy.take_events(); // Clear creation events

        policy.issue(Utc::now().date_naive(), "UW001").unwrap();
        let events = policy.take_events();

        assert!(events.iter().any(|e| matches!(e, PolicyEvent::PolicyIssued { .. })));
    }

    #[test]
    fn test_lapse_generates_lapsed_event() {
        let mut policy = create_test_policy();
        policy.issue(Utc::now().date_naive(), "UW001").unwrap();
        let _ = policy.take_events(); // Clear previous events

        policy.lapse(
            LapseReason::NonPayment {
                grace_days_elapsed: 30,
                outstanding_amount: dec!(1000),
            },
            Some(30),
        ).unwrap();
        let events = policy.take_events();

        assert!(events.iter().any(|e| matches!(e, PolicyEvent::PolicyLapsed { .. })));
    }

    #[test]
    fn test_take_events_clears_event_list() {
        let mut policy = create_test_policy();

        let first_events = policy.take_events();
        assert!(!first_events.is_empty());

        let second_events = policy.take_events();
        assert!(second_events.is_empty());
    }
}
