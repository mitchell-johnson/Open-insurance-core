//! Tests for domain_policy coverage types

use rust_decimal_macros::dec;
use uuid::Uuid;

use core_kernel::{Money, Currency};

use domain_policy::coverage::{
    Coverage, CoverageType, CoverageModification,
    Benefit, BenefitType, BenefitAmount,
    Exclusion, ExclusionType,
};

// ============================================================================
// Coverage Type Tests
// ============================================================================

mod coverage_type_tests {
    use super::*;

    #[test]
    fn test_all_coverage_types() {
        let types = vec![
            CoverageType::DeathBenefit,
            CoverageType::AccidentalDeath,
            CoverageType::TotalPermanentDisability,
            CoverageType::CriticalIllness,
            CoverageType::Hospitalization,
            CoverageType::WaiverOfPremium,
            CoverageType::TermRider,
            CoverageType::WholeLifeRider,
            CoverageType::IncomeBenefit,
            CoverageType::MaturityBenefit,
            CoverageType::Custom("CustomCoverage".to_string()),
        ];

        for coverage_type in types {
            let json = serde_json::to_string(&coverage_type).unwrap();
            assert!(!json.is_empty());
        }
    }
}

// ============================================================================
// Benefit Tests
// ============================================================================

mod benefit_tests {
    use super::*;

    #[test]
    fn test_benefit_types() {
        let types = vec![
            BenefitType::LumpSum,
            BenefitType::Income,
            BenefitType::Reimbursement,
            BenefitType::PerDiem,
        ];

        for benefit_type in types {
            let json = serde_json::to_string(&benefit_type).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_benefit_amount_fixed() {
        let amount = BenefitAmount::Fixed(Money::new(dec!(100000), Currency::USD));

        let json = serde_json::to_string(&amount).unwrap();
        let deserialized: BenefitAmount = serde_json::from_str(&json).unwrap();

        match deserialized {
            BenefitAmount::Fixed(m) => assert_eq!(m.amount(), dec!(100000)),
            _ => panic!("Expected Fixed amount"),
        }
    }

    #[test]
    fn test_benefit_amount_percentage() {
        let amount = BenefitAmount::PercentageOfSumAssured(dec!(50));

        let json = serde_json::to_string(&amount).unwrap();
        assert!(json.contains("50"));
    }

    #[test]
    fn test_benefit_amount_multiple() {
        let amount = BenefitAmount::MultipleOfPremium(dec!(10));

        let json = serde_json::to_string(&amount).unwrap();
        assert!(json.contains("10"));
    }

    #[test]
    fn test_benefit_amount_formula() {
        let amount = BenefitAmount::Formula("SUM_ASSURED * 1.5".to_string());

        let json = serde_json::to_string(&amount).unwrap();
        assert!(json.contains("SUM_ASSURED"));
    }

    #[test]
    fn test_benefit_creation() {
        let benefit = Benefit {
            benefit_type: BenefitType::LumpSum,
            amount: BenefitAmount::Fixed(Money::new(dec!(100000), Currency::USD)),
            waiting_period_days: Some(90),
            max_claim_period_days: Some(365),
        };

        assert_eq!(benefit.benefit_type, BenefitType::LumpSum);
        assert_eq!(benefit.waiting_period_days, Some(90));
        assert_eq!(benefit.max_claim_period_days, Some(365));
    }
}

// ============================================================================
// Exclusion Tests
// ============================================================================

mod exclusion_tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_exclusion_types() {
        let types = vec![
            ExclusionType::Standard,
            ExclusionType::Underwriting,
            ExclusionType::Regulatory,
        ];

        for exclusion_type in types {
            let json = serde_json::to_string(&exclusion_type).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_exclusion_creation() {
        let exclusion = Exclusion {
            code: "EXC001".to_string(),
            description: "Pre-existing conditions".to_string(),
            exclusion_type: ExclusionType::Standard,
            effective_date: Some(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
        };

        assert_eq!(exclusion.code, "EXC001");
        assert_eq!(exclusion.exclusion_type, ExclusionType::Standard);
    }
}

// ============================================================================
// Coverage Tests
// ============================================================================

mod coverage_tests {
    use super::*;

    #[test]
    fn test_coverage_new() {
        let coverage = Coverage::new(
            CoverageType::DeathBenefit,
            Money::new(dec!(500000), Currency::USD),
        );

        assert_eq!(coverage.coverage_type, CoverageType::DeathBenefit);
        assert_eq!(coverage.sum_assured.amount(), dec!(500000));
        assert!(coverage.is_active);
        assert_eq!(coverage.benefits.len(), 1);
    }

    #[test]
    fn test_coverage_death_benefit() {
        let coverage = Coverage::death_benefit(Money::new(dec!(1000000), Currency::USD));

        assert_eq!(coverage.coverage_type, CoverageType::DeathBenefit);
        assert_eq!(coverage.sum_assured.amount(), dec!(1000000));
    }

    #[test]
    fn test_coverage_critical_illness() {
        let coverage = Coverage::critical_illness(Money::new(dec!(100000), Currency::USD), 90);

        assert_eq!(coverage.coverage_type, CoverageType::CriticalIllness);
        assert_eq!(coverage.benefits[0].waiting_period_days, Some(90));
    }

    #[test]
    fn test_coverage_hospitalization() {
        let coverage = Coverage::hospitalization(Money::new(dec!(500), Currency::USD), 365);

        assert_eq!(coverage.coverage_type, CoverageType::Hospitalization);
        assert_eq!(coverage.benefits[0].benefit_type, BenefitType::PerDiem);
        assert_eq!(coverage.benefits[0].max_claim_period_days, Some(365));
    }

    #[test]
    fn test_coverage_add_exclusion() {
        let mut coverage = Coverage::death_benefit(Money::new(dec!(500000), Currency::USD));

        coverage.add_exclusion(Exclusion {
            code: "WAR".to_string(),
            description: "War exclusion".to_string(),
            exclusion_type: ExclusionType::Standard,
            effective_date: None,
        });

        assert_eq!(coverage.exclusions.len(), 1);
    }

    #[test]
    fn test_coverage_with_loading() {
        let coverage = Coverage::death_benefit(Money::new(dec!(500000), Currency::USD))
            .with_loading(dec!(25));

        assert_eq!(coverage.loading_percent, Some(dec!(25)));
    }

    #[test]
    fn test_coverage_is_covered_matching() {
        let coverage = Coverage::death_benefit(Money::new(dec!(500000), Currency::USD));

        assert!(coverage.is_covered(&CoverageType::DeathBenefit));
    }

    #[test]
    fn test_coverage_is_covered_not_matching() {
        let coverage = Coverage::death_benefit(Money::new(dec!(500000), Currency::USD));

        assert!(!coverage.is_covered(&CoverageType::CriticalIllness));
    }

    #[test]
    fn test_coverage_is_covered_inactive() {
        let mut coverage = Coverage::death_benefit(Money::new(dec!(500000), Currency::USD));
        coverage.is_active = false;

        assert!(!coverage.is_covered(&CoverageType::DeathBenefit));
    }

    #[test]
    fn test_coverage_effective_sum_assured() {
        let coverage = Coverage::death_benefit(Money::new(dec!(500000), Currency::USD));

        assert_eq!(coverage.effective_sum_assured().amount(), dec!(500000));
    }

    #[test]
    fn test_coverage_effective_sum_assured_with_loading() {
        let coverage = Coverage::death_benefit(Money::new(dec!(500000), Currency::USD))
            .with_loading(dec!(25));

        // Loading doesn't change sum assured
        assert_eq!(coverage.effective_sum_assured().amount(), dec!(500000));
    }
}

// ============================================================================
// Coverage Modification Tests
// ============================================================================

mod coverage_modification_tests {
    use super::*;

    #[test]
    fn test_coverage_modification_sum_assured() {
        let mut coverage = Coverage::death_benefit(Money::new(dec!(500000), Currency::USD));
        let coverage_id = coverage.id;

        let modification = CoverageModification {
            coverage_id,
            new_sum_assured: Some(Money::new(dec!(750000), Currency::USD)),
            new_benefits: None,
            new_exclusions: None,
        };

        let result = coverage.apply_modification(&modification);

        assert!(result.is_ok());
        assert_eq!(coverage.sum_assured.amount(), dec!(750000));
    }

    #[test]
    fn test_coverage_modification_benefits() {
        let mut coverage = Coverage::death_benefit(Money::new(dec!(500000), Currency::USD));
        let coverage_id = coverage.id;

        let new_benefits = vec![Benefit {
            benefit_type: BenefitType::Income,
            amount: BenefitAmount::PercentageOfSumAssured(dec!(10)),
            waiting_period_days: None,
            max_claim_period_days: Some(120),
        }];

        let modification = CoverageModification {
            coverage_id,
            new_sum_assured: None,
            new_benefits: Some(new_benefits),
            new_exclusions: None,
        };

        let result = coverage.apply_modification(&modification);

        assert!(result.is_ok());
        assert_eq!(coverage.benefits[0].benefit_type, BenefitType::Income);
    }

    #[test]
    fn test_coverage_modification_exclusions() {
        let mut coverage = Coverage::death_benefit(Money::new(dec!(500000), Currency::USD));
        let coverage_id = coverage.id;

        let new_exclusions = vec![
            Exclusion {
                code: "EXC001".to_string(),
                description: "Test exclusion".to_string(),
                exclusion_type: ExclusionType::Underwriting,
                effective_date: None,
            },
        ];

        let modification = CoverageModification {
            coverage_id,
            new_sum_assured: None,
            new_benefits: None,
            new_exclusions: Some(new_exclusions),
        };

        let result = coverage.apply_modification(&modification);

        assert!(result.is_ok());
        assert_eq!(coverage.exclusions.len(), 1);
    }

    #[test]
    fn test_coverage_modification_all_fields() {
        let mut coverage = Coverage::death_benefit(Money::new(dec!(500000), Currency::USD));
        let coverage_id = coverage.id;

        let modification = CoverageModification {
            coverage_id,
            new_sum_assured: Some(Money::new(dec!(1000000), Currency::USD)),
            new_benefits: Some(vec![Benefit {
                benefit_type: BenefitType::LumpSum,
                amount: BenefitAmount::Fixed(Money::new(dec!(1000000), Currency::USD)),
                waiting_period_days: None,
                max_claim_period_days: None,
            }]),
            new_exclusions: Some(vec![]),
        };

        let result = coverage.apply_modification(&modification);

        assert!(result.is_ok());
        assert_eq!(coverage.sum_assured.amount(), dec!(1000000));
    }
}
