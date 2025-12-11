//! Integration Tests for Open Insurance Core
//!
//! These tests verify cross-domain workflows and end-to-end scenarios
//! that involve multiple crates working together.

use chrono::{NaiveDate, Utc};
use core_kernel::{Currency, Money, PartyId, PolicyId, ClaimId};
use rust_decimal_macros::dec;

mod policy_to_claim_workflow {
    use super::*;
    use domain_policy::{
        Coverage, CoverageType, PolicyBuilder, PolicyState, Premium, PremiumFrequency,
    };

    /// Tests that a policy can be created and issued successfully
    #[test]
    fn test_create_and_issue_policy() {
        // Create coverage
        let death_benefit = Coverage::death_benefit(
            Money::new(dec!(500000), Currency::USD),
        );

        // Create premium
        let premium = Premium::new(
            Money::new(dec!(1200), Currency::USD),
            PremiumFrequency::Annual,
        );

        // Build policy
        let mut policy = PolicyBuilder::new()
            .product_code("TERM_LIFE_20")
            .policyholder(PartyId::new())
            .add_coverage(death_benefit)
            .premium(premium)
            .term_years(20)
            .build()
            .expect("Failed to create policy");

        // Verify initial state
        assert!(matches!(policy.state(), PolicyState::Quoted { .. }));

        // Issue the policy
        let effective_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        policy.issue(effective_date, "UW001").expect("Failed to issue policy");

        // Verify issued state
        assert!(policy.is_in_force());
        assert!(matches!(policy.state(), PolicyState::InForce { .. }));
    }

    /// Tests that payments can be recorded on a policy
    #[test]
    fn test_policy_payment_recording() {
        let death_benefit = Coverage::death_benefit(
            Money::new(dec!(500000), Currency::USD),
        );

        let premium = Premium::new(
            Money::new(dec!(1200), Currency::USD),
            PremiumFrequency::Annual,
        );

        let mut policy = PolicyBuilder::new()
            .product_code("TERM_LIFE_20")
            .policyholder(PartyId::new())
            .add_coverage(death_benefit)
            .premium(premium)
            .build()
            .unwrap();

        // Record a payment
        let payment_amount = Money::new(dec!(1200), Currency::USD);
        policy.record_payment(payment_amount).expect("Failed to record payment");

        // Verify payment was recorded
        let financials = policy.financial_state();
        assert_eq!(financials.total_premium_paid.amount(), dec!(1200));
    }
}

mod premium_calculations {
    use super::*;
    use domain_policy::{Premium, PremiumFrequency};
    use domain_policy::premium::{Discount, DiscountType};

    /// Tests annual premium calculation
    #[test]
    fn test_annual_premium() {
        let premium = Premium::new(
            Money::new(dec!(1200), Currency::USD),
            PremiumFrequency::Annual,
        );

        assert_eq!(premium.total_per_payment().amount(), dec!(1200));
        assert_eq!(premium.annualized().amount(), dec!(1200));
    }

    /// Tests monthly premium calculation
    #[test]
    fn test_monthly_premium() {
        let premium = Premium::new(
            Money::new(dec!(100), Currency::USD),
            PremiumFrequency::Monthly,
        );

        assert_eq!(premium.total_per_payment().amount(), dec!(100));
        assert_eq!(premium.annualized().amount(), dec!(1200));
    }

    /// Tests premium with percentage discount
    #[test]
    fn test_premium_with_percentage_discount() {
        let premium = Premium::new(
            Money::new(dec!(1000), Currency::USD),
            PremiumFrequency::Annual,
        ).with_discount(Discount {
            discount_type: DiscountType::Percentage,
            value: dec!(10), // 10% discount
        });

        assert_eq!(premium.total_per_payment().amount(), dec!(900));
    }

    /// Tests premium schedule generation
    #[test]
    fn test_premium_schedule_generation() {
        let premium = Premium::new(
            Money::new(dec!(100), Currency::USD),
            PremiumFrequency::Monthly,
        );

        let start_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let schedule = premium.generate_schedule(start_date, 1);

        assert_eq!(schedule.len(), 12);
        assert_eq!(schedule[0].due_date, start_date);
        assert_eq!(schedule[0].amount.amount(), dec!(100));
    }
}

mod coverage_tests {
    use super::*;
    use domain_policy::{Coverage, CoverageType};
    use domain_policy::coverage::{Exclusion, ExclusionType};

    /// Tests death benefit coverage creation
    #[test]
    fn test_death_benefit_coverage() {
        let coverage = Coverage::death_benefit(
            Money::new(dec!(500000), Currency::USD),
        );

        assert_eq!(coverage.coverage_type, CoverageType::DeathBenefit);
        assert_eq!(coverage.sum_assured.amount(), dec!(500000));
        assert!(coverage.is_active);
    }

    /// Tests critical illness coverage with waiting period
    #[test]
    fn test_critical_illness_coverage() {
        let coverage = Coverage::critical_illness(
            Money::new(dec!(100000), Currency::USD),
            90, // 90 day waiting period
        );

        assert_eq!(coverage.coverage_type, CoverageType::CriticalIllness);
        assert!(coverage.benefits[0].waiting_period_days.is_some());
        assert_eq!(coverage.benefits[0].waiting_period_days.unwrap(), 90);
    }

    /// Tests adding exclusions to coverage
    #[test]
    fn test_coverage_with_exclusions() {
        let mut coverage = Coverage::death_benefit(
            Money::new(dec!(500000), Currency::USD),
        );

        coverage.add_exclusion(Exclusion {
            code: "SUICIDE".to_string(),
            description: "Suicide within first 2 years".to_string(),
            exclusion_type: ExclusionType::Standard,
            effective_date: None,
        });

        assert_eq!(coverage.exclusions.len(), 1);
        assert_eq!(coverage.exclusions[0].code, "SUICIDE");
    }

    /// Tests coverage loading
    #[test]
    fn test_coverage_with_loading() {
        let coverage = Coverage::death_benefit(
            Money::new(dec!(500000), Currency::USD),
        ).with_loading(dec!(25)); // 25% loading

        assert_eq!(coverage.loading_percent, Some(dec!(25)));
    }
}

mod money_operations {
    use super::*;
    use core_kernel::money::Rate;

    /// Tests money allocation for premium splitting
    #[test]
    fn test_money_allocation() {
        let total = Money::new(dec!(1000), Currency::USD);
        let parts = total.allocate(3).unwrap();

        assert_eq!(parts.len(), 3);

        let sum: rust_decimal::Decimal = parts.iter().map(|p| p.amount()).sum();
        assert_eq!(sum, dec!(1000));
    }

    /// Tests rate application
    #[test]
    fn test_rate_application() {
        let rate = Rate::from_percentage(dec!(5));
        let principal = Money::new(dec!(10000), Currency::USD);
        let interest = rate.apply(&principal);

        assert_eq!(interest.amount(), dec!(500));
    }

    /// Tests money arithmetic
    #[test]
    fn test_money_arithmetic() {
        let a = Money::new(dec!(1000), Currency::USD);
        let b = Money::new(dec!(500), Currency::USD);

        let sum = a.checked_add(&b).unwrap();
        assert_eq!(sum.amount(), dec!(1500));

        let diff = a.checked_sub(&b).unwrap();
        assert_eq!(diff.amount(), dec!(500));
    }

    /// Tests currency mismatch prevention
    #[test]
    fn test_currency_mismatch() {
        let usd = Money::new(dec!(1000), Currency::USD);
        let eur = Money::new(dec!(1000), Currency::EUR);

        let result = usd.checked_add(&eur);
        assert!(result.is_err());
    }
}

mod temporal_operations {
    use super::*;
    use core_kernel::{ValidPeriod, BiTemporalRecord};
    use chrono::TimeZone;

    /// Tests valid period creation and containment
    #[test]
    fn test_valid_period_containment() {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap();

        let period = ValidPeriod::bounded(start, end).unwrap();

        // Test mid-year date is contained
        let mid_year = Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();
        assert!(period.contains(mid_year));

        // Test date before period is not contained
        let before = Utc.with_ymd_and_hms(2023, 12, 31, 0, 0, 0).unwrap();
        assert!(!period.contains(before));
    }

    /// Tests period overlap detection
    #[test]
    fn test_period_overlap() {
        let p1 = ValidPeriod::bounded(
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2024, 6, 30, 0, 0, 0).unwrap(),
        ).unwrap();

        let p2 = ValidPeriod::bounded(
            Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2024, 12, 31, 0, 0, 0).unwrap(),
        ).unwrap();

        assert!(p1.overlaps(&p2));
    }

    /// Tests bi-temporal record creation
    #[test]
    fn test_bitemporal_record() {
        let record = BiTemporalRecord::effective_now("Policy data");

        assert!(record.is_current());
        assert!(record.valid_at(Utc::now()));
    }
}

mod identifier_operations {
    use super::*;
    use std::str::FromStr;

    /// Tests policy ID generation and parsing
    #[test]
    fn test_policy_id_roundtrip() {
        let id = PolicyId::new();
        let string = id.to_string();
        let parsed: PolicyId = string.parse().unwrap();

        assert_eq!(id, parsed);
    }

    /// Tests claim ID uniqueness
    #[test]
    fn test_claim_id_uniqueness() {
        let id1 = ClaimId::new();
        let id2 = ClaimId::new();

        assert_ne!(id1, id2);
    }

    /// Tests party ID display format
    #[test]
    fn test_party_id_display() {
        let id = PartyId::new();
        let display = id.to_string();

        assert!(display.starts_with("PTY-"));
    }
}

mod cross_domain_scenarios {
    use super::*;
    use domain_policy::{Coverage, CoverageType, PolicyBuilder, Premium, PremiumFrequency};

    /// Tests a complete policy issuance workflow
    #[test]
    fn test_complete_policy_workflow() {
        // 1. Create policyholder
        let policyholder_id = PartyId::new();

        // 2. Create coverages
        let death_benefit = Coverage::death_benefit(
            Money::new(dec!(500000), Currency::USD),
        );

        let critical_illness = Coverage::critical_illness(
            Money::new(dec!(100000), Currency::USD),
            90,
        );

        // 3. Create premium
        let premium = Premium::new(
            Money::new(dec!(150), Currency::USD),
            PremiumFrequency::Monthly,
        );

        // 4. Build and issue policy
        let mut policy = PolicyBuilder::new()
            .product_code("COMPREHENSIVE_LIFE")
            .policyholder(policyholder_id)
            .add_coverage(death_benefit)
            .add_coverage(critical_illness)
            .premium(premium)
            .term_years(20)
            .build()
            .expect("Failed to build policy");

        // 5. Issue the policy
        let effective_date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        policy.issue(effective_date, "UW001").expect("Failed to issue policy");

        // 6. Record first payment
        let first_payment = Money::new(dec!(150), Currency::USD);
        policy.record_payment(first_payment).expect("Failed to record payment");

        // Verify final state
        assert!(policy.is_in_force());
        assert_eq!(policy.coverages().len(), 2);
        assert_eq!(policy.financial_state().total_premium_paid.amount(), dec!(150));
    }

    /// Tests premium calculation for different frequencies
    #[test]
    fn test_premium_frequency_calculations() {
        let base_annual = dec!(1200);

        // Annual premium
        let annual = Premium::new(
            Money::new(base_annual, Currency::USD),
            PremiumFrequency::Annual,
        );
        assert_eq!(annual.annualized().amount(), base_annual);

        // Monthly premium (should sum to same annual amount)
        let monthly = Premium::new(
            Money::new(dec!(100), Currency::USD),
            PremiumFrequency::Monthly,
        );
        assert_eq!(monthly.annualized().amount(), base_annual);

        // Quarterly premium
        let quarterly = Premium::new(
            Money::new(dec!(300), Currency::USD),
            PremiumFrequency::Quarterly,
        );
        assert_eq!(quarterly.annualized().amount(), base_annual);
    }
}
