//! Custom Test Assertions
//!
//! Provides specialized assertion helpers for domain types that give
//! more meaningful error messages than standard assertions.

use core_kernel::{Money, Currency, ValidPeriod};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Asserts that two Money values are approximately equal within a tolerance
///
/// # Arguments
///
/// * `actual` - The actual Money value
/// * `expected` - The expected Money value
/// * `tolerance` - The allowed difference in the amount
///
/// # Panics
///
/// Panics if the currencies don't match or the amounts differ by more than tolerance
pub fn assert_money_approx_eq(actual: &Money, expected: &Money, tolerance: Decimal) {
    assert_eq!(
        actual.currency(),
        expected.currency(),
        "Currency mismatch: actual={}, expected={}",
        actual.currency(),
        expected.currency()
    );

    let diff = (actual.amount() - expected.amount()).abs();
    assert!(
        diff <= tolerance,
        "Money amounts differ by more than tolerance: actual={}, expected={}, diff={}, tolerance={}",
        actual.amount(),
        expected.amount(),
        diff,
        tolerance
    );
}

/// Asserts that a Money value is positive
pub fn assert_money_positive(money: &Money) {
    assert!(
        money.is_positive(),
        "Expected positive money, got {} {}",
        money.currency().symbol(),
        money.amount()
    );
}

/// Asserts that a Money value is zero
pub fn assert_money_zero(money: &Money) {
    assert!(
        money.is_zero(),
        "Expected zero money, got {} {}",
        money.currency().symbol(),
        money.amount()
    );
}

/// Asserts that a Money value is negative
pub fn assert_money_negative(money: &Money) {
    assert!(
        money.is_negative(),
        "Expected negative money, got {} {}",
        money.currency().symbol(),
        money.amount()
    );
}

/// Asserts that money values sum to a total
///
/// # Arguments
///
/// * `parts` - The money values that should sum to total
/// * `total` - The expected total
///
/// # Panics
///
/// Panics if the sum doesn't equal the total
pub fn assert_money_sum_equals(parts: &[Money], total: &Money) {
    let sum = parts.iter().fold(Money::zero(total.currency()), |acc, m| {
        acc.checked_add(m).expect("Currency mismatch in sum")
    });

    assert_eq!(
        sum.amount(),
        total.amount(),
        "Sum of parts ({}) doesn't equal total ({})",
        sum.amount(),
        total.amount()
    );
}

/// Asserts that a ValidPeriod contains a specific timestamp
pub fn assert_period_contains<T: std::fmt::Debug + chrono::TimeZone>(
    period: &ValidPeriod,
    timestamp: chrono::DateTime<T>,
) where
    T::Offset: std::fmt::Display,
{
    let utc_timestamp = timestamp.with_timezone(&chrono::Utc);
    assert!(
        period.contains(utc_timestamp),
        "Period {:?} does not contain timestamp {}",
        period,
        utc_timestamp
    );
}

/// Asserts that a ValidPeriod does not contain a specific timestamp
pub fn assert_period_excludes<T: std::fmt::Debug + chrono::TimeZone>(
    period: &ValidPeriod,
    timestamp: chrono::DateTime<T>,
) where
    T::Offset: std::fmt::Display,
{
    let utc_timestamp = timestamp.with_timezone(&chrono::Utc);
    assert!(
        !period.contains(utc_timestamp),
        "Period {:?} unexpectedly contains timestamp {}",
        period,
        utc_timestamp
    );
}

/// Asserts that two ValidPeriods overlap
pub fn assert_periods_overlap(period1: &ValidPeriod, period2: &ValidPeriod) {
    assert!(
        period1.overlaps(period2),
        "Periods {:?} and {:?} do not overlap",
        period1,
        period2
    );
}

/// Asserts that two ValidPeriods do not overlap
pub fn assert_periods_disjoint(period1: &ValidPeriod, period2: &ValidPeriod) {
    assert!(
        !period1.overlaps(period2),
        "Periods {:?} and {:?} unexpectedly overlap",
        period1,
        period2
    );
}

/// Asserts that a decimal value is within a range
pub fn assert_decimal_in_range(value: Decimal, min: Decimal, max: Decimal) {
    assert!(
        value >= min && value <= max,
        "Decimal {} is not in range [{}, {}]",
        value,
        min,
        max
    );
}

/// Asserts that a decimal value is approximately equal to another
pub fn assert_decimal_approx_eq(actual: Decimal, expected: Decimal, tolerance: Decimal) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tolerance,
        "Decimals differ by more than tolerance: actual={}, expected={}, diff={}, tolerance={}",
        actual,
        expected,
        diff,
        tolerance
    );
}

/// Asserts that unit calculations are precise to 6 decimal places
pub fn assert_units_precise(units: Decimal) {
    let scale = units.scale();
    assert!(
        scale <= 6,
        "Units {} exceed maximum precision of 6 decimal places (scale={})",
        units,
        scale
    );
}

/// Asserts that a result is Ok and returns the value
#[macro_export]
macro_rules! assert_ok {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(e) => panic!("Expected Ok, got Err: {:?}", e),
        }
    };
    ($result:expr, $msg:expr) => {
        match $result {
            Ok(value) => value,
            Err(e) => panic!("{}: {:?}", $msg, e),
        }
    };
}

/// Asserts that a result is Err and returns the error
#[macro_export]
macro_rules! assert_err {
    ($result:expr) => {
        match $result {
            Ok(value) => panic!("Expected Err, got Ok: {:?}", value),
            Err(e) => e,
        }
    };
    ($result:expr, $msg:expr) => {
        match $result {
            Ok(value) => panic!("{}: got Ok({:?})", $msg, value),
            Err(e) => e,
        }
    };
}

/// Asserts that an error matches a specific variant
#[macro_export]
macro_rules! assert_err_variant {
    ($result:expr, $pattern:pat) => {
        match $result {
            Ok(value) => panic!("Expected Err matching {}, got Ok({:?})", stringify!($pattern), value),
            Err(ref e) => {
                assert!(
                    matches!(e, $pattern),
                    "Error {:?} does not match pattern {}",
                    e,
                    stringify!($pattern)
                );
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn test_assert_money_approx_eq_passes() {
        let m1 = Money::new(dec!(100.001), Currency::USD);
        let m2 = Money::new(dec!(100.002), Currency::USD);
        assert_money_approx_eq(&m1, &m2, dec!(0.01));
    }

    #[test]
    #[should_panic(expected = "Currency mismatch")]
    fn test_assert_money_approx_eq_currency_mismatch() {
        let m1 = Money::new(dec!(100.00), Currency::USD);
        let m2 = Money::new(dec!(100.00), Currency::EUR);
        assert_money_approx_eq(&m1, &m2, dec!(0.01));
    }

    #[test]
    fn test_assert_money_positive() {
        let m = Money::new(dec!(100.00), Currency::USD);
        assert_money_positive(&m);
    }

    #[test]
    #[should_panic(expected = "Expected positive money")]
    fn test_assert_money_positive_fails_for_zero() {
        let m = Money::zero(Currency::USD);
        assert_money_positive(&m);
    }

    #[test]
    fn test_assert_money_sum_equals() {
        let parts = vec![
            Money::new(dec!(33.34), Currency::USD),
            Money::new(dec!(33.33), Currency::USD),
            Money::new(dec!(33.33), Currency::USD),
        ];
        let total = Money::new(dec!(100.00), Currency::USD);
        assert_money_sum_equals(&parts, &total);
    }

    #[test]
    fn test_assert_period_contains() {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap();
        let period = ValidPeriod::bounded(start, end).unwrap();

        let mid = Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();
        assert_period_contains(&period, mid);
    }

    #[test]
    fn test_assert_decimal_approx_eq() {
        let a = dec!(100.001);
        let b = dec!(100.002);
        assert_decimal_approx_eq(a, b, dec!(0.01));
    }
}
