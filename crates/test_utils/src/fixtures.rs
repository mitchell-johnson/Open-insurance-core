//! Pre-built Test Fixtures
//!
//! Provides ready-to-use test data for common entities across the insurance system.
//! These fixtures are designed to be consistent and predictable for unit tests.

use chrono::{DateTime, NaiveDate, Utc, TimeZone};
use core_kernel::{
    Money, Currency, ValidPeriod, SystemPeriod, BiTemporalRecord,
    PolicyId, ClaimId, PartyId, AccountId, FundId, UnitHoldingId,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use uuid::Uuid;

/// Fixture for Money test data
pub struct MoneyFixtures;

impl MoneyFixtures {
    /// Creates a standard USD amount for testing
    pub fn usd_100() -> Money {
        Money::new(dec!(100.00), Currency::USD)
    }

    /// Creates a large USD amount for testing premium calculations
    pub fn usd_premium() -> Money {
        Money::new(dec!(1500.00), Currency::USD)
    }

    /// Creates a sum assured amount
    pub fn usd_sum_assured() -> Money {
        Money::new(dec!(500000.00), Currency::USD)
    }

    /// Creates a zero amount
    pub fn usd_zero() -> Money {
        Money::zero(Currency::USD)
    }

    /// Creates a EUR amount for currency mismatch tests
    pub fn eur_100() -> Money {
        Money::new(dec!(100.00), Currency::EUR)
    }

    /// Creates a JPY amount (zero decimal places)
    pub fn jpy_10000() -> Money {
        Money::new(dec!(10000), Currency::JPY)
    }

    /// Creates a negative amount for refund scenarios
    pub fn usd_refund() -> Money {
        Money::new(dec!(-50.00), Currency::USD)
    }
}

/// Fixture for temporal test data
pub struct TemporalFixtures;

impl TemporalFixtures {
    /// Standard policy start date (Jan 1, 2024)
    pub fn policy_start() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap()
    }

    /// Standard policy end date (Dec 31, 2024)
    pub fn policy_end() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap()
    }

    /// Mid-year timestamp for containment tests
    pub fn mid_year() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap()
    }

    /// Pre-policy timestamp
    pub fn before_policy() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2023, 12, 1, 0, 0, 0).unwrap()
    }

    /// Post-policy timestamp
    pub fn after_policy() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2025, 2, 1, 0, 0, 0).unwrap()
    }

    /// Creates a valid period for a standard 1-year policy
    pub fn one_year_policy_period() -> ValidPeriod {
        ValidPeriod::bounded(Self::policy_start(), Self::policy_end()).unwrap()
    }

    /// Creates an unbounded valid period starting now
    pub fn unbounded_from_now() -> ValidPeriod {
        ValidPeriod::from(Utc::now())
    }

    /// Standard loss date for claims
    pub fn loss_date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2024, 3, 15).unwrap()
    }

    /// Standard date of birth for test party (age 35)
    pub fn date_of_birth_35() -> NaiveDate {
        NaiveDate::from_ymd_opt(1989, 5, 15).unwrap()
    }
}

/// Fixture for identifier test data
pub struct IdFixtures;

impl IdFixtures {
    /// Creates a deterministic policy ID for testing
    pub fn policy_id() -> PolicyId {
        PolicyId::from_uuid(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap())
    }

    /// Creates a deterministic claim ID for testing
    pub fn claim_id() -> ClaimId {
        ClaimId::from_uuid(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440002").unwrap())
    }

    /// Creates a deterministic party ID for testing
    pub fn party_id() -> PartyId {
        PartyId::from_uuid(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440003").unwrap())
    }

    /// Creates a deterministic account ID for testing
    pub fn account_id() -> AccountId {
        AccountId::from_uuid(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440004").unwrap())
    }

    /// Creates a deterministic fund ID for testing
    pub fn fund_id() -> FundId {
        FundId::from_uuid(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440005").unwrap())
    }

    /// Creates a deterministic unit holding ID for testing
    pub fn unit_holding_id() -> UnitHoldingId {
        UnitHoldingId::from_uuid(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440006").unwrap())
    }
}

/// Fixture for decimal test data
pub struct DecimalFixtures;

impl DecimalFixtures {
    /// Standard NAV value
    pub fn nav_value() -> Decimal {
        dec!(15.4532)
    }

    /// Standard interest rate (5%)
    pub fn interest_rate() -> Decimal {
        dec!(0.05)
    }

    /// Standard mortality rate
    pub fn mortality_rate() -> Decimal {
        dec!(0.001)
    }

    /// Standard allocation percentage (100%)
    pub fn full_allocation() -> Decimal {
        dec!(1.00)
    }

    /// Partial allocation (60%)
    pub fn partial_allocation() -> Decimal {
        dec!(0.60)
    }

    /// Zero for comparison tests
    pub fn zero() -> Decimal {
        Decimal::ZERO
    }

    /// Small epsilon for floating point comparisons
    pub fn epsilon() -> Decimal {
        dec!(0.000001)
    }
}

/// Fixture for string test data
pub struct StringFixtures;

impl StringFixtures {
    /// Standard product code
    pub fn product_code() -> &'static str {
        "TERM_LIFE_20"
    }

    /// Standard policy number
    pub fn policy_number() -> &'static str {
        "POL-2024-000001"
    }

    /// Standard claim number
    pub fn claim_number() -> &'static str {
        "CLM-2024-000001"
    }

    /// Standard account code
    pub fn account_code() -> &'static str {
        "1000-CASH"
    }

    /// Standard fund code
    pub fn fund_code() -> &'static str {
        "EQ-GROWTH-01"
    }

    /// Test email address
    pub fn email() -> &'static str {
        "john.doe@example.com"
    }

    /// Test phone number
    pub fn phone() -> &'static str {
        "+1-555-123-4567"
    }

    /// Test first name
    pub fn first_name() -> &'static str {
        "John"
    }

    /// Test last name
    pub fn last_name() -> &'static str {
        "Doe"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_money_fixtures_currencies_match() {
        let usd = MoneyFixtures::usd_100();
        assert_eq!(usd.currency(), Currency::USD);

        let eur = MoneyFixtures::eur_100();
        assert_eq!(eur.currency(), Currency::EUR);
    }

    #[test]
    fn test_temporal_fixtures_ordering() {
        let start = TemporalFixtures::policy_start();
        let mid = TemporalFixtures::mid_year();
        let end = TemporalFixtures::policy_end();

        assert!(start < mid);
        assert!(mid < end);
    }

    #[test]
    fn test_id_fixtures_are_deterministic() {
        let id1 = IdFixtures::policy_id();
        let id2 = IdFixtures::policy_id();
        assert_eq!(id1, id2);
    }
}
