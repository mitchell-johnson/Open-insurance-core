//! Comprehensive tests for domain_fund

use chrono::{NaiveDate, Utc, Days};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use core_kernel::{FundId, PolicyId, NavId};

use domain_fund::fund::{Fund, FundType, RiskLevel};
use domain_fund::allocation::{Allocation, AllocationStrategy};
use domain_fund::nav::{Nav, NavHistory};
use domain_fund::unit_holding::UnitHolding;
use domain_fund::unit_transaction::{UnitTransaction, TransactionType};

// ============================================================================
// Fund Tests
// ============================================================================

mod fund_creation_tests {
    use super::*;

    #[test]
    fn test_fund_new() {
        let fund = Fund::new("EQF001", "Equity Growth Fund", FundType::Equity, RiskLevel::High);

        assert_eq!(fund.code, "EQF001");
        assert_eq!(fund.name, "Equity Growth Fund");
        assert_eq!(fund.fund_type, FundType::Equity);
        assert_eq!(fund.risk_level, RiskLevel::High);
        assert!(fund.is_active);
    }

    #[test]
    fn test_fund_with_different_risk_levels() {
        let fund = Fund::new("BND001", "Bond Fund", FundType::Bond, RiskLevel::Low);

        assert_eq!(fund.risk_level, RiskLevel::Low);
    }

    #[test]
    fn test_fund_with_management_fee() {
        let fund = Fund::new("IDX001", "Index Fund", FundType::Index, RiskLevel::Medium)
            .with_management_fee(dec!(0.50));

        assert_eq!(fund.management_fee, dec!(0.50));
    }

    #[test]
    fn test_all_fund_types() {
        let types = vec![
            FundType::Equity,
            FundType::Bond,
            FundType::Balanced,
            FundType::MoneyMarket,
            FundType::Index,
            FundType::Sector,
        ];

        for fund_type in types {
            // Just verify the type can be created
            assert!(matches!(fund_type, FundType::Equity | FundType::Bond | FundType::Balanced | FundType::MoneyMarket | FundType::Index | FundType::Sector));
        }
    }

    #[test]
    fn test_all_risk_levels() {
        let levels = vec![
            RiskLevel::Low,
            RiskLevel::MediumLow,
            RiskLevel::Medium,
            RiskLevel::MediumHigh,
            RiskLevel::High,
        ];

        for level in levels {
            assert!(matches!(level, RiskLevel::Low | RiskLevel::MediumLow | RiskLevel::Medium | RiskLevel::MediumHigh | RiskLevel::High));
        }
    }
}

// ============================================================================
// Allocation Tests
// ============================================================================

mod allocation_tests {
    use super::*;

    #[test]
    fn test_allocation_strategy_new_valid() {
        let fund1 = FundId::new_v7();
        let fund2 = FundId::new_v7();

        let allocations = vec![
            Allocation { fund_id: fund1, percentage: dec!(60) },
            Allocation { fund_id: fund2, percentage: dec!(40) },
        ];

        let strategy = AllocationStrategy::new(allocations);
        assert!(strategy.is_ok());
    }

    #[test]
    fn test_allocation_strategy_invalid_total() {
        let fund1 = FundId::new_v7();
        let fund2 = FundId::new_v7();

        let allocations = vec![
            Allocation { fund_id: fund1, percentage: dec!(60) },
            Allocation { fund_id: fund2, percentage: dec!(30) },
        ];

        let strategy = AllocationStrategy::new(allocations);
        assert!(strategy.is_err());
    }

    #[test]
    fn test_allocation_strategy_invalid_percentage() {
        let fund1 = FundId::new_v7();

        let allocations = vec![
            Allocation { fund_id: fund1, percentage: dec!(101) },
        ];

        let strategy = AllocationStrategy::new(allocations);
        assert!(strategy.is_err());
    }

    #[test]
    fn test_allocation_strategy_negative_percentage() {
        let fund1 = FundId::new_v7();
        let fund2 = FundId::new_v7();

        let allocations = vec![
            Allocation { fund_id: fund1, percentage: dec!(-10) },
            Allocation { fund_id: fund2, percentage: dec!(110) },
        ];

        let strategy = AllocationStrategy::new(allocations);
        assert!(strategy.is_err());
    }

    #[test]
    fn test_allocation_calculate_amounts() {
        let fund1 = FundId::new_v7();
        let fund2 = FundId::new_v7();

        let allocations = vec![
            Allocation { fund_id: fund1, percentage: dec!(60) },
            Allocation { fund_id: fund2, percentage: dec!(40) },
        ];

        let strategy = AllocationStrategy::new(allocations).unwrap();
        let amounts = strategy.calculate_amounts(dec!(1000));

        assert_eq!(amounts.len(), 2);

        let total: Decimal = amounts.iter().map(|(_, a)| *a).sum();
        assert_eq!(total, dec!(1000));
    }

    #[test]
    fn test_allocation_single_fund() {
        let fund = FundId::new_v7();

        let allocations = vec![
            Allocation { fund_id: fund, percentage: dec!(100) },
        ];

        let strategy = AllocationStrategy::new(allocations);
        assert!(strategy.is_ok());
    }
}

// ============================================================================
// NAV Tests
// ============================================================================

mod nav_tests {
    use super::*;

    #[test]
    fn test_nav_new() {
        let fund_id = FundId::new_v7();
        let nav_date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        let nav = Nav::new(fund_id, nav_date, dec!(10.50), "USD");

        assert_eq!(nav.fund_id, fund_id);
        assert_eq!(nav.nav_date, nav_date);
        assert_eq!(nav.value, dec!(10.50));
        assert!(nav.aum.is_none());
        assert!(nav.bid_price.is_none());
        assert!(nav.offer_price.is_none());
    }

    #[test]
    fn test_nav_with_dual_pricing() {
        let fund_id = FundId::new_v7();
        let nav_date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        let nav = Nav::new(fund_id, nav_date, dec!(10.50), "USD")
            .with_dual_pricing(dec!(10.45), dec!(10.55));

        assert_eq!(nav.bid_price, Some(dec!(10.45)));
        assert_eq!(nav.offer_price, Some(dec!(10.55)));
    }

    #[test]
    fn test_nav_with_aum() {
        let fund_id = FundId::new_v7();
        let nav_date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        let nav = Nav::new(fund_id, nav_date, dec!(10.50), "USD")
            .with_aum(dec!(1000000));

        assert_eq!(nav.aum, Some(dec!(1000000)));
    }

    #[test]
    fn test_nav_history_new() {
        let fund_id = FundId::new_v7();
        let history = NavHistory::new(fund_id);

        assert_eq!(history.fund_id, fund_id);
        assert!(history.navs.is_empty());
    }

    #[test]
    fn test_nav_history_add() {
        let fund_id = FundId::new_v7();
        let mut history = NavHistory::new(fund_id);

        let nav1 = Nav::new(fund_id, NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(), dec!(10.00), "USD");
        let nav2 = Nav::new(fund_id, NaiveDate::from_ymd_opt(2024, 1, 16).unwrap(), dec!(10.50), "USD");

        history.add(nav1);
        history.add(nav2);

        assert_eq!(history.navs.len(), 2);
    }

    #[test]
    fn test_nav_history_latest() {
        let fund_id = FundId::new_v7();
        let mut history = NavHistory::new(fund_id);

        let nav1 = Nav::new(fund_id, NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(), dec!(10.00), "USD");
        let nav2 = Nav::new(fund_id, NaiveDate::from_ymd_opt(2024, 1, 16).unwrap(), dec!(10.50), "USD");

        history.add(nav2);
        history.add(nav1);

        let latest = history.latest().unwrap();
        assert_eq!(latest.value, dec!(10.50));
    }

    #[test]
    fn test_nav_history_at_date() {
        let fund_id = FundId::new_v7();
        let mut history = NavHistory::new(fund_id);

        let date1 = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2024, 1, 16).unwrap();

        history.add(Nav::new(fund_id, date1, dec!(10.00), "USD"));
        history.add(Nav::new(fund_id, date2, dec!(10.50), "USD"));

        let nav = history.at_date(date1).unwrap();
        assert_eq!(nav.value, dec!(10.00));
    }

    #[test]
    fn test_nav_history_at_date_not_found() {
        let fund_id = FundId::new_v7();
        let history = NavHistory::new(fund_id);

        let result = history.at_date(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap());
        assert!(result.is_none());
    }

    #[test]
    fn test_nav_history_calculate_return() {
        let fund_id = FundId::new_v7();
        let mut history = NavHistory::new(fund_id);

        let date1 = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();

        history.add(Nav::new(fund_id, date1, dec!(10.00), "USD"));
        history.add(Nav::new(fund_id, date2, dec!(11.00), "USD"));

        let return_pct = history.calculate_return(date1, date2).unwrap();
        assert_eq!(return_pct, dec!(0.1)); // 10% return
    }

    #[test]
    fn test_nav_history_calculate_return_zero_start() {
        let fund_id = FundId::new_v7();
        let mut history = NavHistory::new(fund_id);

        let date1 = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();

        history.add(Nav::new(fund_id, date1, Decimal::ZERO, "USD"));
        history.add(Nav::new(fund_id, date2, dec!(11.00), "USD"));

        let result = history.calculate_return(date1, date2);
        assert!(result.is_none());
    }

    #[test]
    fn test_nav_history_calculate_return_missing_dates() {
        let fund_id = FundId::new_v7();
        let history = NavHistory::new(fund_id);

        let date1 = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();

        let result = history.calculate_return(date1, date2);
        assert!(result.is_none());
    }
}

// ============================================================================
// Unit Holding Tests
// ============================================================================

mod unit_holding_tests {
    use super::*;

    #[test]
    fn test_unit_holding_new() {
        let policy_id = PolicyId::new_v7();
        let fund_id = FundId::new_v7();

        let holding = UnitHolding::new(policy_id, fund_id);

        assert_eq!(holding.policy_id, policy_id);
        assert_eq!(holding.fund_id, fund_id);
        assert_eq!(holding.units, Decimal::ZERO);
    }

    #[test]
    fn test_unit_holding_add_units() {
        let mut holding = UnitHolding::new(PolicyId::new_v7(), FundId::new_v7());

        holding.add_units(dec!(100.123456));

        assert_eq!(holding.units, dec!(100.123456));
    }

    #[test]
    fn test_unit_holding_add_units_multiple() {
        let mut holding = UnitHolding::new(PolicyId::new_v7(), FundId::new_v7());

        holding.add_units(dec!(100));
        holding.add_units(dec!(50.5));

        assert_eq!(holding.units, dec!(150.5));
    }

    #[test]
    fn test_unit_holding_remove_units() {
        let mut holding = UnitHolding::new(PolicyId::new_v7(), FundId::new_v7());
        holding.add_units(dec!(100));

        let result = holding.remove_units(dec!(30));

        assert!(result.is_ok());
        assert_eq!(holding.units, dec!(70));
    }

    #[test]
    fn test_unit_holding_remove_units_insufficient() {
        let mut holding = UnitHolding::new(PolicyId::new_v7(), FundId::new_v7());
        holding.add_units(dec!(100));

        let result = holding.remove_units(dec!(150));

        assert!(result.is_err());
        assert_eq!(holding.units, dec!(100)); // Unchanged
    }

    #[test]
    fn test_unit_holding_value_at_nav() {
        let mut holding = UnitHolding::new(PolicyId::new_v7(), FundId::new_v7());
        holding.add_units(dec!(100));

        let value = holding.value_at_nav(dec!(10.50));

        assert_eq!(value, dec!(1050));
    }
}

// ============================================================================
// Unit Transaction Tests
// ============================================================================

mod unit_transaction_tests {
    use super::*;

    #[test]
    fn test_unit_transaction_new() {
        let policy_id = PolicyId::new_v7();
        let fund_id = FundId::new_v7();

        let txn = UnitTransaction::new(
            policy_id,
            fund_id,
            TransactionType::Allocation,
            dec!(100),
            dec!(10.50),
        );

        assert_eq!(txn.policy_id, policy_id);
        assert_eq!(txn.fund_id, fund_id);
        assert_eq!(txn.transaction_type, TransactionType::Allocation);
        assert_eq!(txn.units, dec!(100));
        assert_eq!(txn.nav, dec!(10.50));
        assert_eq!(txn.value, dec!(1050)); // 100 * 10.50
    }

    #[test]
    fn test_unit_transaction_with_reference() {
        let txn = UnitTransaction::new(
            PolicyId::new_v7(),
            FundId::new_v7(),
            TransactionType::Allocation,
            dec!(100),
            dec!(10.00),
        ).with_reference("PREM-12345");

        assert_eq!(txn.reference, Some("PREM-12345".to_string()));
    }

    #[test]
    fn test_all_transaction_types() {
        let types = vec![
            TransactionType::Allocation,
            TransactionType::Redemption,
            TransactionType::SwitchIn,
            TransactionType::SwitchOut,
            TransactionType::MortalityCharge,
            TransactionType::PolicyFee,
            TransactionType::ManagementFee,
            TransactionType::Bonus,
        ];

        for txn_type in types {
            // Verify all transaction types can be matched
            assert!(matches!(txn_type,
                TransactionType::Allocation |
                TransactionType::Redemption |
                TransactionType::SwitchIn |
                TransactionType::SwitchOut |
                TransactionType::MortalityCharge |
                TransactionType::PolicyFee |
                TransactionType::ManagementFee |
                TransactionType::Bonus
            ));
        }
    }

    #[test]
    fn test_unit_transaction_negative_units() {
        let txn = UnitTransaction::new(
            PolicyId::new_v7(),
            FundId::new_v7(),
            TransactionType::Redemption,
            dec!(-50),
            dec!(10.00),
        );

        assert_eq!(txn.units, dec!(-50));
        assert_eq!(txn.value, dec!(-500));
    }
}

// ============================================================================
// Library Function Tests
// ============================================================================

mod lib_tests {
    use super::*;
    use domain_fund::{calculate_units, calculate_value, round_units};

    #[test]
    fn test_calculate_units() {
        let units = calculate_units(dec!(1000), dec!(10));
        assert_eq!(units, dec!(100));
    }

    #[test]
    fn test_calculate_value() {
        let value = calculate_value(dec!(100), dec!(10.50));
        assert_eq!(value, dec!(1050));
    }

    #[test]
    fn test_round_units() {
        let rounded = round_units(dec!(100.1234567890));
        // Should round to 6 decimal places
        assert_eq!(rounded, dec!(100.123457));
    }
}
