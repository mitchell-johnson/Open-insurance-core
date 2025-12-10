//! Money types with precise decimal arithmetic
//!
//! This module provides a type-safe representation of monetary values
//! using rust_decimal for precise calculations without floating-point errors.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Sub, Mul, Div, Neg};
use thiserror::Error;

/// Currency codes following ISO 4217
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Currency {
    USD,
    EUR,
    GBP,
    JPY,
    CHF,
    INR,
    AUD,
    CAD,
    SGD,
    HKD,
}

impl Currency {
    /// Returns the number of decimal places for this currency
    pub fn decimal_places(&self) -> u32 {
        match self {
            Currency::JPY => 0,
            _ => 2,
        }
    }

    /// Returns the currency symbol
    pub fn symbol(&self) -> &'static str {
        match self {
            Currency::USD => "$",
            Currency::EUR => "€",
            Currency::GBP => "£",
            Currency::JPY => "¥",
            Currency::CHF => "CHF",
            Currency::INR => "₹",
            Currency::AUD => "A$",
            Currency::CAD => "C$",
            Currency::SGD => "S$",
            Currency::HKD => "HK$",
        }
    }

    /// Returns the ISO 4217 code
    pub fn code(&self) -> &'static str {
        match self {
            Currency::USD => "USD",
            Currency::EUR => "EUR",
            Currency::GBP => "GBP",
            Currency::JPY => "JPY",
            Currency::CHF => "CHF",
            Currency::INR => "INR",
            Currency::AUD => "AUD",
            Currency::CAD => "CAD",
            Currency::SGD => "SGD",
            Currency::HKD => "HKD",
        }
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// Errors that can occur during money operations
#[derive(Debug, Error, PartialEq, Eq)]
pub enum MoneyError {
    #[error("Currency mismatch: cannot operate on {0} and {1}")]
    CurrencyMismatch(String, String),

    #[error("Invalid amount: {0}")]
    InvalidAmount(String),

    #[error("Division by zero")]
    DivisionByZero,

    #[error("Overflow during calculation")]
    Overflow,
}

/// A monetary amount with associated currency
///
/// Money uses rust_decimal for precise arithmetic without floating-point errors.
/// Currency amounts are stored with 4 decimal places internally to handle
/// exchange rate calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Money {
    amount: Decimal,
    currency: Currency,
}

impl Money {
    /// Creates a new Money value
    pub fn new(amount: Decimal, currency: Currency) -> Self {
        Self {
            amount: amount.round_dp(4),
            currency,
        }
    }

    /// Creates Money from an integer amount in minor units (e.g., cents)
    pub fn from_minor(minor_units: i64, currency: Currency) -> Self {
        let divisor = Decimal::new(10_i64.pow(currency.decimal_places()), 0);
        Self::new(Decimal::new(minor_units, 0) / divisor, currency)
    }

    /// Creates a zero amount in the specified currency
    pub fn zero(currency: Currency) -> Self {
        Self {
            amount: dec!(0),
            currency,
        }
    }

    /// Returns the amount
    pub fn amount(&self) -> Decimal {
        self.amount
    }

    /// Returns the currency
    pub fn currency(&self) -> Currency {
        self.currency
    }

    /// Returns true if the amount is zero
    pub fn is_zero(&self) -> bool {
        self.amount.is_zero()
    }

    /// Returns true if the amount is positive
    pub fn is_positive(&self) -> bool {
        self.amount.is_sign_positive() && !self.amount.is_zero()
    }

    /// Returns true if the amount is negative
    pub fn is_negative(&self) -> bool {
        self.amount.is_sign_negative()
    }

    /// Returns the absolute value
    pub fn abs(&self) -> Self {
        Self {
            amount: self.amount.abs(),
            currency: self.currency,
        }
    }

    /// Rounds to the currency's standard decimal places
    pub fn round_to_currency(&self) -> Self {
        Self {
            amount: self.amount.round_dp(self.currency.decimal_places()),
            currency: self.currency,
        }
    }

    /// Rounds using banker's rounding (round half to even)
    pub fn round_bankers(&self, dp: u32) -> Self {
        Self {
            amount: self.amount.round_dp_with_strategy(
                dp,
                rust_decimal::RoundingStrategy::MidpointNearestEven,
            ),
            currency: self.currency,
        }
    }

    /// Checked addition that returns an error on currency mismatch
    pub fn checked_add(&self, other: &Money) -> Result<Money, MoneyError> {
        if self.currency != other.currency {
            return Err(MoneyError::CurrencyMismatch(
                self.currency.to_string(),
                other.currency.to_string(),
            ));
        }
        Ok(Self::new(self.amount + other.amount, self.currency))
    }

    /// Checked subtraction that returns an error on currency mismatch
    pub fn checked_sub(&self, other: &Money) -> Result<Money, MoneyError> {
        if self.currency != other.currency {
            return Err(MoneyError::CurrencyMismatch(
                self.currency.to_string(),
                other.currency.to_string(),
            ));
        }
        Ok(Self::new(self.amount - other.amount, self.currency))
    }

    /// Multiplies by a scalar (e.g., for rate calculations)
    pub fn multiply(&self, factor: Decimal) -> Self {
        Self::new(self.amount * factor, self.currency)
    }

    /// Divides by a scalar
    pub fn divide(&self, divisor: Decimal) -> Result<Self, MoneyError> {
        if divisor.is_zero() {
            return Err(MoneyError::DivisionByZero);
        }
        Ok(Self::new(self.amount / divisor, self.currency))
    }

    /// Allocates the money into n equal parts, handling remainders
    /// The remainder is distributed among the first allocations
    pub fn allocate(&self, n: u32) -> Result<Vec<Money>, MoneyError> {
        if n == 0 {
            return Err(MoneyError::InvalidAmount("Cannot allocate to zero parts".to_string()));
        }

        let dp = self.currency.decimal_places();
        let total_minor = self.amount * Decimal::new(10_i64.pow(dp), 0);
        let total_minor = total_minor.round().mantissa();

        let base_amount = total_minor / n as i128;
        let remainder = (total_minor % n as i128) as u32;

        let mut allocations = Vec::with_capacity(n as usize);
        for i in 0..n {
            let minor = if i < remainder {
                base_amount + 1
            } else {
                base_amount
            };
            allocations.push(Money::from_minor(minor as i64, self.currency));
        }

        Ok(allocations)
    }

    /// Allocates money according to given ratios
    /// Returns allocations in the same order as the ratios
    pub fn allocate_by_ratios(&self, ratios: &[Decimal]) -> Result<Vec<Money>, MoneyError> {
        if ratios.is_empty() {
            return Err(MoneyError::InvalidAmount("Empty ratios".to_string()));
        }

        let total_ratio: Decimal = ratios.iter().sum();
        if total_ratio.is_zero() {
            return Err(MoneyError::InvalidAmount("Total ratio is zero".to_string()));
        }

        let dp = self.currency.decimal_places();
        let mut allocated = Money::zero(self.currency);
        let mut allocations = Vec::with_capacity(ratios.len());

        for (i, ratio) in ratios.iter().enumerate() {
            if i == ratios.len() - 1 {
                // Last allocation gets the remainder to ensure sum equals original
                let remainder = self.checked_sub(&allocated)?;
                allocations.push(remainder);
            } else {
                let allocation = Self::new(
                    (self.amount * *ratio / total_ratio).round_dp(dp),
                    self.currency,
                );
                allocated = allocated.checked_add(&allocation)?;
                allocations.push(allocation);
            }
        }

        Ok(allocations)
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dp = self.currency.decimal_places();
        write!(
            f,
            "{} {:.dp$}",
            self.currency.symbol(),
            self.amount,
            dp = dp as usize
        )
    }
}

impl Add for Money {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        self.checked_add(&other)
            .expect("Currency mismatch in Money::add")
    }
}

impl Sub for Money {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        self.checked_sub(&other)
            .expect("Currency mismatch in Money::sub")
    }
}

impl Neg for Money {
    type Output = Self;

    fn neg(self) -> Self {
        Self::new(-self.amount, self.currency)
    }
}

impl Mul<Decimal> for Money {
    type Output = Self;

    fn mul(self, factor: Decimal) -> Self {
        self.multiply(factor)
    }
}

impl Div<Decimal> for Money {
    type Output = Self;

    fn div(self, divisor: Decimal) -> Self {
        self.divide(divisor).expect("Division by zero in Money::div")
    }
}

/// Represents a percentage rate (e.g., interest rate, charge rate)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rate {
    /// The rate as a decimal (e.g., 0.05 for 5%)
    value: Decimal,
}

impl Rate {
    /// Creates a rate from a decimal value (e.g., 0.05 for 5%)
    pub fn new(value: Decimal) -> Self {
        Self { value }
    }

    /// Creates a rate from a percentage (e.g., 5.0 for 5%)
    pub fn from_percentage(percentage: Decimal) -> Self {
        Self {
            value: percentage / dec!(100),
        }
    }

    /// Returns the rate as a decimal
    pub fn as_decimal(&self) -> Decimal {
        self.value
    }

    /// Returns the rate as a percentage
    pub fn as_percentage(&self) -> Decimal {
        self.value * dec!(100)
    }

    /// Applies this rate to a money amount
    pub fn apply(&self, money: &Money) -> Money {
        money.multiply(self.value)
    }
}

impl fmt::Display for Rate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}%", self.as_percentage().round_dp(4))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_money_creation() {
        let m = Money::new(dec!(100.50), Currency::USD);
        assert_eq!(m.amount(), dec!(100.50));
        assert_eq!(m.currency(), Currency::USD);
    }

    #[test]
    fn test_money_from_minor() {
        let m = Money::from_minor(10050, Currency::USD);
        assert_eq!(m.amount(), dec!(100.50));
    }

    #[test]
    fn test_money_arithmetic() {
        let a = Money::new(dec!(100.00), Currency::USD);
        let b = Money::new(dec!(50.00), Currency::USD);

        assert_eq!((a + b).amount(), dec!(150.00));
        assert_eq!((a - b).amount(), dec!(50.00));
    }

    #[test]
    fn test_currency_mismatch() {
        let usd = Money::new(dec!(100.00), Currency::USD);
        let eur = Money::new(dec!(100.00), Currency::EUR);

        let result = usd.checked_add(&eur);
        assert!(matches!(result, Err(MoneyError::CurrencyMismatch(_, _))));
    }

    #[test]
    fn test_money_allocation() {
        let m = Money::new(dec!(100.00), Currency::USD);
        let parts = m.allocate(3).unwrap();

        assert_eq!(parts.len(), 3);
        let total: Money = parts.into_iter().fold(Money::zero(Currency::USD), |acc, p| acc + p);
        assert_eq!(total, m);
    }

    #[test]
    fn test_rate_application() {
        let rate = Rate::from_percentage(dec!(5.0));
        let amount = Money::new(dec!(1000.00), Currency::USD);

        let charge = rate.apply(&amount);
        assert_eq!(charge.amount(), dec!(50.00));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn money_allocation_sum_equals_original(
            amount in 1i64..1_000_000_000i64,
            parts in 1u32..100u32
        ) {
            let money = Money::from_minor(amount, Currency::USD);
            let allocations = money.allocate(parts).unwrap();

            let total: Decimal = allocations.iter().map(|m| m.amount()).sum();
            prop_assert_eq!(total, money.amount());
        }

        #[test]
        fn money_arithmetic_is_associative(
            a in -1_000_000i64..1_000_000i64,
            b in -1_000_000i64..1_000_000i64,
            c in -1_000_000i64..1_000_000i64
        ) {
            let ma = Money::from_minor(a, Currency::USD);
            let mb = Money::from_minor(b, Currency::USD);
            let mc = Money::from_minor(c, Currency::USD);

            prop_assert_eq!((ma + mb) + mc, ma + (mb + mc));
        }
    }
}
