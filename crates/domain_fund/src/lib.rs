//! Fund Management Domain
//!
//! This crate implements fund management for Unit-Linked Insurance Plans (ULIPs),
//! including unit registries, NAV calculations, and fund operations.
//!
//! # Key Concepts
//!
//! - **Fund**: An investment vehicle that policyholders can allocate to
//! - **NAV**: Net Asset Value per unit, updated daily
//! - **Unit Holding**: A policyholder's units in a specific fund
//! - **Unit Transaction**: Any operation that affects unit holdings
//!
//! # Unit Precision
//!
//! Units are stored with 6 decimal places to ensure precision:
//! - Premium: $1,000
//! - NAV: $15.45
//! - Units: 64.724919

pub mod fund;
pub mod nav;
pub mod unit_holding;
pub mod unit_transaction;
pub mod allocation;
pub mod error;

pub use fund::{Fund, FundType, RiskLevel};
pub use nav::{Nav, NavHistory};
pub use unit_holding::UnitHolding;
pub use unit_transaction::{UnitTransaction, TransactionType};
pub use allocation::{Allocation, AllocationStrategy};
pub use error::FundError;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Standard unit precision (6 decimal places)
pub const UNIT_PRECISION: u32 = 6;

/// Rounds a value to standard unit precision
///
/// # Arguments
///
/// * `value` - The value to round
///
/// # Returns
///
/// The value rounded to 6 decimal places
pub fn round_units(value: Decimal) -> Decimal {
    value.round_dp(UNIT_PRECISION)
}

/// Calculates units from an amount and NAV
///
/// # Arguments
///
/// * `amount` - The monetary amount to convert
/// * `nav` - The Net Asset Value per unit
///
/// # Returns
///
/// The number of units (rounded to 6 decimal places)
///
/// # Example
///
/// ```rust
/// use domain_fund::calculate_units;
/// use rust_decimal_macros::dec;
///
/// let units = calculate_units(dec!(1000), dec!(15.45));
/// assert!(units > dec!(64) && units < dec!(65));
/// ```
pub fn calculate_units(amount: Decimal, nav: Decimal) -> Decimal {
    if nav.is_zero() {
        return Decimal::ZERO;
    }
    round_units(amount / nav)
}

/// Calculates the value of units at a given NAV
///
/// # Arguments
///
/// * `units` - Number of units
/// * `nav` - The Net Asset Value per unit
///
/// # Returns
///
/// The monetary value of the units
pub fn calculate_value(units: Decimal, nav: Decimal) -> Decimal {
    (units * nav).round_dp(2) // Round to currency precision
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_units() {
        let units = calculate_units(dec!(1000), dec!(15.45));
        assert_eq!(units.round_dp(4), dec!(64.7249));
    }

    #[test]
    fn test_calculate_value() {
        let value = calculate_value(dec!(64.724919), dec!(15.45));
        assert_eq!(value, dec!(1000.00));
    }

    #[test]
    fn test_units_precision() {
        let units = round_units(dec!(123.456789012345));
        assert_eq!(units, dec!(123.456789));
    }
}
