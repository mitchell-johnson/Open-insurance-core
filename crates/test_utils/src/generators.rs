//! Property-Based Test Generators
//!
//! Provides proptest strategies for generating random test data
//! that maintains domain invariants.

use core_kernel::{Money, Currency, ValidPeriod, PolicyId, ClaimId, PartyId};
use chrono::{DateTime, Duration, TimeZone, Utc};
use proptest::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Strategy for generating valid Currency values
pub fn currency_strategy() -> impl Strategy<Value = Currency> {
    prop_oneof![
        Just(Currency::USD),
        Just(Currency::EUR),
        Just(Currency::GBP),
        Just(Currency::JPY),
        Just(Currency::CHF),
        Just(Currency::INR),
        Just(Currency::AUD),
        Just(Currency::CAD),
        Just(Currency::SGD),
        Just(Currency::HKD),
    ]
}

/// Strategy for generating valid positive amounts in minor units
pub fn positive_amount_minor_strategy() -> impl Strategy<Value = i64> {
    1i64..1_000_000_000i64
}

/// Strategy for generating valid amount ranges
pub fn amount_minor_strategy() -> impl Strategy<Value = i64> {
    -1_000_000_000i64..1_000_000_000i64
}

/// Strategy for generating valid Money values with positive amounts
pub fn positive_money_strategy() -> impl Strategy<Value = Money> {
    (positive_amount_minor_strategy(), currency_strategy())
        .prop_map(|(amount, currency)| Money::from_minor(amount, currency))
}

/// Strategy for generating valid Money values (can be negative)
pub fn money_strategy() -> impl Strategy<Value = Money> {
    (amount_minor_strategy(), currency_strategy())
        .prop_map(|(amount, currency)| Money::from_minor(amount, currency))
}

/// Strategy for generating valid USD Money values
pub fn usd_money_strategy() -> impl Strategy<Value = Money> {
    positive_amount_minor_strategy()
        .prop_map(|amount| Money::from_minor(amount, Currency::USD))
}

/// Strategy for generating valid Decimal values for rates (0.0 to 1.0)
pub fn rate_decimal_strategy() -> impl Strategy<Value = Decimal> {
    (0u32..10000u32).prop_map(|n| Decimal::new(n as i64, 4))
}

/// Strategy for generating valid Decimal values for percentages (0% to 100%)
pub fn percentage_strategy() -> impl Strategy<Value = Decimal> {
    (0u32..10000u32).prop_map(|n| Decimal::new(n as i64, 2))
}

/// Strategy for generating positive Decimal values
pub fn positive_decimal_strategy() -> impl Strategy<Value = Decimal> {
    (1i64..1_000_000_000i64, 0u32..4u32)
        .prop_map(|(m, s)| Decimal::new(m, s))
}

/// Strategy for generating valid NAV values (typically 1.0 to 1000.0)
pub fn nav_strategy() -> impl Strategy<Value = Decimal> {
    (100i64..100000i64).prop_map(|n| Decimal::new(n, 2))
}

/// Strategy for generating allocation parts (1 to 100)
pub fn allocation_parts_strategy() -> impl Strategy<Value = u32> {
    1u32..100u32
}

/// Strategy for generating valid allocation ratios that sum to 1.0
pub fn allocation_ratios_strategy(count: usize) -> impl Strategy<Value = Vec<Decimal>> {
    proptest::collection::vec(1u32..1000u32, count..=count)
        .prop_map(|weights| {
            let total: u32 = weights.iter().sum();
            weights
                .into_iter()
                .map(|w| Decimal::new(w as i64, 0) / Decimal::new(total as i64, 0))
                .collect()
        })
}

/// Strategy for generating valid timestamps within a year
pub fn timestamp_2024_strategy() -> impl Strategy<Value = DateTime<Utc>> {
    (0i64..365i64).prop_map(|days| {
        Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap() + Duration::days(days)
    })
}

/// Strategy for generating valid time ranges (start before end)
pub fn time_range_strategy() -> impl Strategy<Value = (DateTime<Utc>, DateTime<Utc>)> {
    (0i64..365i64, 1i64..365i64).prop_map(|(start_days, duration_days)| {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap() + Duration::days(start_days);
        let end = start + Duration::days(duration_days);
        (start, end)
    })
}

/// Strategy for generating valid ValidPeriod instances
pub fn valid_period_strategy() -> impl Strategy<Value = ValidPeriod> {
    time_range_strategy().prop_map(|(start, end)| {
        ValidPeriod::bounded(start, end).expect("Generated invalid period")
    })
}

/// Strategy for generating PolicyId
pub fn policy_id_strategy() -> impl Strategy<Value = PolicyId> {
    any::<[u8; 16]>().prop_map(|bytes| {
        PolicyId::from_uuid(uuid::Uuid::from_bytes(bytes))
    })
}

/// Strategy for generating ClaimId
pub fn claim_id_strategy() -> impl Strategy<Value = ClaimId> {
    any::<[u8; 16]>().prop_map(|bytes| {
        ClaimId::from_uuid(uuid::Uuid::from_bytes(bytes))
    })
}

/// Strategy for generating PartyId
pub fn party_id_strategy() -> impl Strategy<Value = PartyId> {
    any::<[u8; 16]>().prop_map(|bytes| {
        PartyId::from_uuid(uuid::Uuid::from_bytes(bytes))
    })
}

/// Strategy for generating valid ages (18 to 99)
pub fn age_strategy() -> impl Strategy<Value = u32> {
    18u32..100u32
}

/// Strategy for generating policy terms in years (1 to 40)
pub fn term_years_strategy() -> impl Strategy<Value = u32> {
    1u32..41u32
}

/// Strategy for generating product codes
pub fn product_code_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("TERM_LIFE_10".to_string()),
        Just("TERM_LIFE_20".to_string()),
        Just("TERM_LIFE_30".to_string()),
        Just("WHOLE_LIFE".to_string()),
        Just("ULIP_GROWTH".to_string()),
        Just("ULIP_BALANCED".to_string()),
        Just("ENDOWMENT_15".to_string()),
    ]
}

/// Strategy for generating fund types
pub fn fund_type_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("equity".to_string()),
        Just("bond".to_string()),
        Just("balanced".to_string()),
        Just("money_market".to_string()),
        Just("index".to_string()),
    ]
}

/// Strategy for generating valid email addresses
pub fn email_strategy() -> impl Strategy<Value = String> {
    ("[a-z]{5,10}", "[a-z]{3,8}")
        .prop_map(|(local, domain)| format!("{}@{}.com", local, domain))
}

/// Strategy for generating valid phone numbers
pub fn phone_strategy() -> impl Strategy<Value = String> {
    (100u32..999u32, 100u32..999u32, 1000u32..9999u32)
        .prop_map(|(area, prefix, line)| format!("+1-{}-{}-{}", area, prefix, line))
}

/// Strategy for generating names
pub fn name_strategy() -> impl Strategy<Value = String> {
    "[A-Z][a-z]{2,10}".prop_map(|s| s)
}

#[cfg(test)]
mod tests {
    use super::*;

    proptest! {
        #[test]
        fn positive_money_is_always_positive(money in positive_money_strategy()) {
            prop_assert!(money.amount() > Decimal::ZERO);
        }

        #[test]
        fn rate_is_valid(rate in rate_decimal_strategy()) {
            prop_assert!(rate >= Decimal::ZERO);
            prop_assert!(rate <= Decimal::ONE);
        }

        #[test]
        fn valid_period_end_after_start(period in valid_period_strategy()) {
            if let Some(end) = period.end {
                prop_assert!(end > period.start);
            }
        }

        #[test]
        fn allocation_ratios_sum_to_one(ratios in allocation_ratios_strategy(5)) {
            let sum: Decimal = ratios.iter().sum();
            // Allow small rounding error
            prop_assert!((sum - Decimal::ONE).abs() < dec!(0.0001));
        }

        #[test]
        fn age_is_valid_for_insurance(age in age_strategy()) {
            prop_assert!(age >= 18);
            prop_assert!(age < 100);
        }
    }
}
