//! Comprehensive unit tests for the Money module
//!
//! Tests cover money creation, arithmetic operations, allocation,
//! currency handling, and edge cases.

use core_kernel::{Money, Currency, MoneyError};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

mod creation {
    use super::*;

    #[test]
    fn test_new_creates_money_with_correct_amount() {
        let m = Money::new(dec!(100.50), Currency::USD);
        assert_eq!(m.amount(), dec!(100.50));
        assert_eq!(m.currency(), Currency::USD);
    }

    #[test]
    fn test_new_rounds_to_four_decimal_places() {
        let m = Money::new(dec!(100.123456789), Currency::USD);
        assert_eq!(m.amount(), dec!(100.1235));
    }

    #[test]
    fn test_from_minor_converts_cents_correctly() {
        let m = Money::from_minor(10050, Currency::USD);
        assert_eq!(m.amount(), dec!(100.50));
    }

    #[test]
    fn test_from_minor_handles_jpy_no_decimals() {
        let m = Money::from_minor(10000, Currency::JPY);
        assert_eq!(m.amount(), dec!(10000));
    }

    #[test]
    fn test_zero_creates_zero_amount() {
        let m = Money::zero(Currency::EUR);
        assert!(m.is_zero());
        assert_eq!(m.currency(), Currency::EUR);
    }

    #[test]
    fn test_negative_amount_creation() {
        let m = Money::new(dec!(-100.00), Currency::USD);
        assert!(m.is_negative());
        assert_eq!(m.amount(), dec!(-100.00));
    }
}

mod predicates {
    use super::*;

    #[test]
    fn test_is_zero_true_for_zero_amount() {
        let m = Money::zero(Currency::USD);
        assert!(m.is_zero());
    }

    #[test]
    fn test_is_zero_false_for_positive_amount() {
        let m = Money::new(dec!(0.01), Currency::USD);
        assert!(!m.is_zero());
    }

    #[test]
    fn test_is_positive_true_for_positive_amount() {
        let m = Money::new(dec!(100.00), Currency::USD);
        assert!(m.is_positive());
    }

    #[test]
    fn test_is_positive_false_for_zero() {
        let m = Money::zero(Currency::USD);
        assert!(!m.is_positive());
    }

    #[test]
    fn test_is_positive_false_for_negative() {
        let m = Money::new(dec!(-100.00), Currency::USD);
        assert!(!m.is_positive());
    }

    #[test]
    fn test_is_negative_true_for_negative_amount() {
        let m = Money::new(dec!(-100.00), Currency::USD);
        assert!(m.is_negative());
    }

    #[test]
    fn test_is_negative_false_for_zero() {
        let m = Money::zero(Currency::USD);
        assert!(!m.is_negative());
    }
}

mod arithmetic {
    use super::*;

    #[test]
    fn test_checked_add_same_currency() {
        let a = Money::new(dec!(100.00), Currency::USD);
        let b = Money::new(dec!(50.00), Currency::USD);
        let result = a.checked_add(&b).unwrap();
        assert_eq!(result.amount(), dec!(150.00));
    }

    #[test]
    fn test_checked_add_currency_mismatch() {
        let a = Money::new(dec!(100.00), Currency::USD);
        let b = Money::new(dec!(50.00), Currency::EUR);
        let result = a.checked_add(&b);
        assert!(matches!(result, Err(MoneyError::CurrencyMismatch(_, _))));
    }

    #[test]
    fn test_checked_sub_same_currency() {
        let a = Money::new(dec!(100.00), Currency::USD);
        let b = Money::new(dec!(30.00), Currency::USD);
        let result = a.checked_sub(&b).unwrap();
        assert_eq!(result.amount(), dec!(70.00));
    }

    #[test]
    fn test_checked_sub_can_go_negative() {
        let a = Money::new(dec!(30.00), Currency::USD);
        let b = Money::new(dec!(100.00), Currency::USD);
        let result = a.checked_sub(&b).unwrap();
        assert_eq!(result.amount(), dec!(-70.00));
    }

    #[test]
    fn test_add_operator_same_currency() {
        let a = Money::new(dec!(100.00), Currency::USD);
        let b = Money::new(dec!(50.00), Currency::USD);
        let result = a + b;
        assert_eq!(result.amount(), dec!(150.00));
    }

    #[test]
    fn test_sub_operator_same_currency() {
        let a = Money::new(dec!(100.00), Currency::USD);
        let b = Money::new(dec!(30.00), Currency::USD);
        let result = a - b;
        assert_eq!(result.amount(), dec!(70.00));
    }

    #[test]
    fn test_negation() {
        let m = Money::new(dec!(100.00), Currency::USD);
        let neg = -m;
        assert_eq!(neg.amount(), dec!(-100.00));
    }

    #[test]
    fn test_negation_of_negative() {
        let m = Money::new(dec!(-100.00), Currency::USD);
        let pos = -m;
        assert_eq!(pos.amount(), dec!(100.00));
    }

    #[test]
    fn test_multiply_by_scalar() {
        let m = Money::new(dec!(100.00), Currency::USD);
        let result = m.multiply(dec!(1.5));
        assert_eq!(result.amount(), dec!(150.00));
    }

    #[test]
    fn test_multiply_by_zero() {
        let m = Money::new(dec!(100.00), Currency::USD);
        let result = m.multiply(dec!(0));
        assert!(result.is_zero());
    }

    #[test]
    fn test_multiply_operator() {
        let m = Money::new(dec!(100.00), Currency::USD);
        let result = m * dec!(2);
        assert_eq!(result.amount(), dec!(200.00));
    }

    #[test]
    fn test_divide_by_scalar() {
        let m = Money::new(dec!(100.00), Currency::USD);
        let result = m.divide(dec!(4)).unwrap();
        assert_eq!(result.amount(), dec!(25.00));
    }

    #[test]
    fn test_divide_by_zero_error() {
        let m = Money::new(dec!(100.00), Currency::USD);
        let result = m.divide(dec!(0));
        assert!(matches!(result, Err(MoneyError::DivisionByZero)));
    }

    #[test]
    fn test_divide_operator() {
        let m = Money::new(dec!(100.00), Currency::USD);
        let result = m / dec!(5);
        assert_eq!(result.amount(), dec!(20.00));
    }
}

mod abs_and_rounding {
    use super::*;

    #[test]
    fn test_abs_positive() {
        let m = Money::new(dec!(100.00), Currency::USD);
        assert_eq!(m.abs().amount(), dec!(100.00));
    }

    #[test]
    fn test_abs_negative() {
        let m = Money::new(dec!(-100.00), Currency::USD);
        assert_eq!(m.abs().amount(), dec!(100.00));
    }

    #[test]
    fn test_abs_zero() {
        let m = Money::zero(Currency::USD);
        assert_eq!(m.abs().amount(), dec!(0));
    }

    #[test]
    fn test_round_to_currency_usd() {
        let m = Money::new(dec!(100.1234), Currency::USD);
        let rounded = m.round_to_currency();
        assert_eq!(rounded.amount(), dec!(100.12));
    }

    #[test]
    fn test_round_to_currency_jpy() {
        // JPY has 0 decimal places, so 100.60 rounds up to 101
        let m = Money::new(dec!(100.60), Currency::JPY);
        let rounded = m.round_to_currency();
        assert_eq!(rounded.amount(), dec!(101));
    }

    #[test]
    fn test_round_bankers() {
        let m = Money::new(dec!(100.125), Currency::USD);
        let rounded = m.round_bankers(2);
        // Banker's rounding: 100.125 -> 100.12 (round to even)
        assert_eq!(rounded.amount(), dec!(100.12));
    }

    #[test]
    fn test_round_bankers_odd_rounds_up() {
        let m = Money::new(dec!(100.135), Currency::USD);
        let rounded = m.round_bankers(2);
        // Banker's rounding: 100.135 -> 100.14 (round to even)
        assert_eq!(rounded.amount(), dec!(100.14));
    }
}

mod allocation {
    use super::*;

    #[test]
    fn test_allocate_equal_parts() {
        let m = Money::new(dec!(100.00), Currency::USD);
        let parts = m.allocate(4).unwrap();

        assert_eq!(parts.len(), 4);
        let total: Decimal = parts.iter().map(|p| p.amount()).sum();
        assert_eq!(total, dec!(100.00));
    }

    #[test]
    fn test_allocate_handles_remainder() {
        let m = Money::new(dec!(100.00), Currency::USD);
        let parts = m.allocate(3).unwrap();

        assert_eq!(parts.len(), 3);
        let total: Decimal = parts.iter().map(|p| p.amount()).sum();
        assert_eq!(total, dec!(100.00));

        // First part should get the extra cent
        assert!(parts[0].amount() >= parts[2].amount());
    }

    #[test]
    fn test_allocate_zero_parts_error() {
        let m = Money::new(dec!(100.00), Currency::USD);
        let result = m.allocate(0);
        assert!(matches!(result, Err(MoneyError::InvalidAmount(_))));
    }

    #[test]
    fn test_allocate_single_part() {
        let m = Money::new(dec!(100.00), Currency::USD);
        let parts = m.allocate(1).unwrap();

        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].amount(), dec!(100.00));
    }

    #[test]
    fn test_allocate_by_ratios() {
        let m = Money::new(dec!(100.00), Currency::USD);
        let ratios = vec![dec!(0.5), dec!(0.3), dec!(0.2)];
        let parts = m.allocate_by_ratios(&ratios).unwrap();

        assert_eq!(parts.len(), 3);
        let total: Decimal = parts.iter().map(|p| p.amount()).sum();
        assert_eq!(total, dec!(100.00));
    }

    #[test]
    fn test_allocate_by_ratios_empty_error() {
        let m = Money::new(dec!(100.00), Currency::USD);
        let result = m.allocate_by_ratios(&[]);
        assert!(matches!(result, Err(MoneyError::InvalidAmount(_))));
    }

    #[test]
    fn test_allocate_by_ratios_zero_total_error() {
        let m = Money::new(dec!(100.00), Currency::USD);
        let ratios = vec![dec!(0), dec!(0), dec!(0)];
        let result = m.allocate_by_ratios(&ratios);
        assert!(matches!(result, Err(MoneyError::InvalidAmount(_))));
    }

    #[test]
    fn test_allocate_by_ratios_last_gets_remainder() {
        let m = Money::new(dec!(100.00), Currency::USD);
        let ratios = vec![dec!(1), dec!(1), dec!(1)];
        let parts = m.allocate_by_ratios(&ratios).unwrap();

        let total: Decimal = parts.iter().map(|p| p.amount()).sum();
        assert_eq!(total, dec!(100.00));
    }
}

mod currency {
    use super::*;

    #[test]
    fn test_all_currencies_have_symbols() {
        let currencies = [
            Currency::USD, Currency::EUR, Currency::GBP, Currency::JPY,
            Currency::CHF, Currency::INR, Currency::AUD, Currency::CAD,
            Currency::SGD, Currency::HKD,
        ];

        for currency in currencies {
            assert!(!currency.symbol().is_empty());
            assert!(!currency.code().is_empty());
        }
    }

    #[test]
    fn test_currency_codes() {
        assert_eq!(Currency::USD.code(), "USD");
        assert_eq!(Currency::EUR.code(), "EUR");
        assert_eq!(Currency::GBP.code(), "GBP");
        assert_eq!(Currency::JPY.code(), "JPY");
    }

    #[test]
    fn test_currency_decimal_places() {
        assert_eq!(Currency::USD.decimal_places(), 2);
        assert_eq!(Currency::EUR.decimal_places(), 2);
        assert_eq!(Currency::JPY.decimal_places(), 0);
    }

    #[test]
    fn test_currency_display() {
        assert_eq!(format!("{}", Currency::USD), "USD");
        assert_eq!(format!("{}", Currency::EUR), "EUR");
    }
}

mod display {
    use super::*;

    #[test]
    fn test_money_display_usd() {
        let m = Money::new(dec!(1234.56), Currency::USD);
        let display = format!("{}", m);
        assert!(display.contains("$"));
        assert!(display.contains("1234.56"));
    }

    #[test]
    fn test_money_display_eur() {
        let m = Money::new(dec!(1234.56), Currency::EUR);
        let display = format!("{}", m);
        assert!(display.contains("€"));
    }

    #[test]
    fn test_money_display_jpy() {
        let m = Money::new(dec!(12345), Currency::JPY);
        let display = format!("{}", m);
        assert!(display.contains("¥"));
    }
}

mod rate {
    use core_kernel::money::Rate;
    use super::*;

    #[test]
    fn test_rate_from_decimal() {
        let rate = Rate::new(dec!(0.05));
        assert_eq!(rate.as_decimal(), dec!(0.05));
    }

    #[test]
    fn test_rate_from_percentage() {
        let rate = Rate::from_percentage(dec!(5.0));
        assert_eq!(rate.as_decimal(), dec!(0.05));
    }

    #[test]
    fn test_rate_as_percentage() {
        let rate = Rate::new(dec!(0.05));
        assert_eq!(rate.as_percentage(), dec!(5.0));
    }

    #[test]
    fn test_rate_apply() {
        let rate = Rate::from_percentage(dec!(10.0));
        let amount = Money::new(dec!(1000.00), Currency::USD);
        let result = rate.apply(&amount);
        assert_eq!(result.amount(), dec!(100.00));
    }

    #[test]
    fn test_rate_display() {
        let rate = Rate::from_percentage(dec!(5.0));
        let display = format!("{}", rate);
        assert!(display.contains("5"));
        assert!(display.contains("%"));
    }
}

mod serialization {
    use super::*;
    use serde_json;

    #[test]
    fn test_money_json_roundtrip() {
        let m = Money::new(dec!(100.50), Currency::USD);
        let json = serde_json::to_string(&m).unwrap();
        let deserialized: Money = serde_json::from_str(&json).unwrap();
        assert_eq!(m, deserialized);
    }

    #[test]
    fn test_currency_json_roundtrip() {
        let c = Currency::USD;
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(json, "\"USD\"");
        let deserialized: Currency = serde_json::from_str(&json).unwrap();
        assert_eq!(c, deserialized);
    }
}

mod equality {
    use super::*;

    #[test]
    fn test_money_equality_same_values() {
        let a = Money::new(dec!(100.00), Currency::USD);
        let b = Money::new(dec!(100.00), Currency::USD);
        assert_eq!(a, b);
    }

    #[test]
    fn test_money_inequality_different_amounts() {
        let a = Money::new(dec!(100.00), Currency::USD);
        let b = Money::new(dec!(100.01), Currency::USD);
        assert_ne!(a, b);
    }

    #[test]
    fn test_money_inequality_different_currencies() {
        let a = Money::new(dec!(100.00), Currency::USD);
        let b = Money::new(dec!(100.00), Currency::EUR);
        assert_ne!(a, b);
    }

    #[test]
    fn test_money_hash_equality() {
        use std::collections::HashSet;

        let a = Money::new(dec!(100.00), Currency::USD);
        let b = Money::new(dec!(100.00), Currency::USD);

        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
    }
}
