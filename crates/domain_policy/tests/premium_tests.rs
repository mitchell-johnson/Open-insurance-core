//! Premium Calculation and Schedule Tests
//!
//! This module contains comprehensive tests for premium-related functionality:
//! - Premium frequency calculations
//! - Premium builder methods (fees, taxes, riders, discounts)
//! - Premium schedule generation
//! - Discount calculations
//!
//! # Test Coverage
//!
//! ## Premium Frequency
//! - Payments per year for all frequencies
//! - Modal factors for each frequency
//! - Next due date calculations including edge cases
//!
//! ## Premium Components
//! - Base premium
//! - Policy fees
//! - Rider premiums
//! - Taxes
//! - Various discount types
//!
//! ## Premium Schedule
//! - Schedule generation for different periods
//! - Entry properties and payment status
//!
//! # Test Organization
//!
//! - `frequency_tests` - PremiumFrequency method tests
//! - `premium_builder_tests` - Premium builder method tests
//! - `discount_tests` - Discount calculation tests
//! - `schedule_tests` - Premium schedule generation tests

use chrono::{Datelike, NaiveDate};
use core_kernel::{Currency, Money};
use domain_policy::premium::{
    Discount, DiscountType, PaymentStatus, Premium, PremiumFrequency,
    PremiumSchedule, PremiumScheduleEntry, RiderPremium,
};
use rust_decimal_macros::dec;

// ============================================================================
// PREMIUM FREQUENCY TESTS
// ============================================================================

mod frequency_tests {
    use super::*;

    /// Verifies payments_per_year returns correct values for all frequencies
    #[test]
    fn test_payments_per_year() {
        assert_eq!(
            PremiumFrequency::Single.payments_per_year(),
            1,
            "Single premium has 1 payment"
        );
        assert_eq!(
            PremiumFrequency::Annual.payments_per_year(),
            1,
            "Annual has 1 payment per year"
        );
        assert_eq!(
            PremiumFrequency::SemiAnnual.payments_per_year(),
            2,
            "Semi-annual has 2 payments per year"
        );
        assert_eq!(
            PremiumFrequency::Quarterly.payments_per_year(),
            4,
            "Quarterly has 4 payments per year"
        );
        assert_eq!(
            PremiumFrequency::Monthly.payments_per_year(),
            12,
            "Monthly has 12 payments per year"
        );
    }

    /// Verifies modal factors are correct for all frequencies
    #[test]
    fn test_modal_factors() {
        assert_eq!(
            PremiumFrequency::Single.modal_factor(),
            dec!(1.0),
            "Single premium modal factor should be 1.0"
        );
        assert_eq!(
            PremiumFrequency::Annual.modal_factor(),
            dec!(1.0),
            "Annual modal factor should be 1.0"
        );
        assert_eq!(
            PremiumFrequency::SemiAnnual.modal_factor(),
            dec!(0.5125),
            "Semi-annual modal factor should be 0.5125 (~2.5% loading)"
        );
        assert_eq!(
            PremiumFrequency::Quarterly.modal_factor(),
            dec!(0.2625),
            "Quarterly modal factor should be 0.2625 (~5% loading)"
        );
        assert_eq!(
            PremiumFrequency::Monthly.modal_factor(),
            dec!(0.0875),
            "Monthly modal factor should be 0.0875 (~5% loading)"
        );
    }

    /// Verifies next_due_date for Single frequency returns same date
    #[test]
    fn test_next_due_date_single() {
        let today = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let next = PremiumFrequency::Single.next_due_date(today);

        assert_eq!(next, today, "Single premium should return same date");
    }

    /// Verifies next_due_date for Annual frequency adds one year
    #[test]
    fn test_next_due_date_annual() {
        let today = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let next = PremiumFrequency::Annual.next_due_date(today);

        assert_eq!(
            next,
            NaiveDate::from_ymd_opt(2025, 6, 15).unwrap(),
            "Annual should add one year"
        );
    }

    /// Verifies next_due_date for SemiAnnual frequency adds 182 days
    #[test]
    fn test_next_due_date_semi_annual() {
        let today = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let next = PremiumFrequency::SemiAnnual.next_due_date(today);

        assert_eq!(
            next,
            today + chrono::Duration::days(182),
            "Semi-annual should add 182 days"
        );
    }

    /// Verifies next_due_date for Quarterly frequency adds 91 days
    #[test]
    fn test_next_due_date_quarterly() {
        let today = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let next = PremiumFrequency::Quarterly.next_due_date(today);

        assert_eq!(
            next,
            today + chrono::Duration::days(91),
            "Quarterly should add 91 days"
        );
    }

    /// Verifies next_due_date for Monthly frequency adds one month
    #[test]
    fn test_next_due_date_monthly() {
        let today = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let next = PremiumFrequency::Monthly.next_due_date(today);

        assert_eq!(
            next,
            NaiveDate::from_ymd_opt(2024, 7, 15).unwrap(),
            "Monthly should add one month"
        );
    }

    /// Verifies monthly due date handles December to January rollover
    #[test]
    fn test_next_due_date_monthly_december_rollover() {
        let december = NaiveDate::from_ymd_opt(2024, 12, 15).unwrap();
        let next = PremiumFrequency::Monthly.next_due_date(december);

        assert_eq!(
            next,
            NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
            "December monthly should roll to January next year"
        );
    }

    /// Verifies annual due date handles leap year edge case
    #[test]
    fn test_next_due_date_annual_leap_year() {
        let feb_29 = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        let next = PremiumFrequency::Annual.next_due_date(feb_29);

        // 2025 is not a leap year, so Feb 29 doesn't exist
        // Should fall back to adding 365 days
        assert!(next.year() == 2025, "Should be in 2025");
    }

    /// Verifies monthly due date handles month-end edge cases
    #[test]
    fn test_next_due_date_monthly_month_end() {
        // Jan 31 -> Feb doesn't have 31 days
        let jan_31 = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        let next = PremiumFrequency::Monthly.next_due_date(jan_31);

        // Falls back to adding 30 days
        assert!(
            next.month() == 2 || next.month() == 3,
            "Should handle month-end gracefully"
        );
    }
}

// ============================================================================
// PREMIUM BUILDER TESTS
// ============================================================================

mod premium_builder_tests {
    use super::*;

    /// Verifies basic premium creation
    #[test]
    fn test_basic_premium_creation() {
        let premium = Premium::new(
            Money::new(dec!(1000), Currency::USD),
            PremiumFrequency::Annual,
        );

        assert_eq!(
            premium.total_per_payment().amount(),
            dec!(1000),
            "Base premium should equal 1000"
        );
        assert_eq!(
            premium.annualized().amount(),
            dec!(1000),
            "Annualized should equal 1000 for annual frequency"
        );
    }

    /// Verifies premium with policy fee
    #[test]
    fn test_premium_with_policy_fee() {
        let premium = Premium::new(
            Money::new(dec!(1000), Currency::USD),
            PremiumFrequency::Annual,
        )
        .with_policy_fee(Money::new(dec!(50), Currency::USD));

        assert_eq!(
            premium.total_per_payment().amount(),
            dec!(1050),
            "Premium should include policy fee"
        );
    }

    /// Verifies premium with tax
    #[test]
    fn test_premium_with_tax() {
        let premium = Premium::new(
            Money::new(dec!(1000), Currency::USD),
            PremiumFrequency::Annual,
        )
        .with_tax(Money::new(dec!(100), Currency::USD));

        assert_eq!(
            premium.total_per_payment().amount(),
            dec!(1100),
            "Premium should include tax"
        );
    }

    /// Verifies adding rider premium
    #[test]
    fn test_add_rider_premium() {
        let mut premium = Premium::new(
            Money::new(dec!(1000), Currency::USD),
            PremiumFrequency::Annual,
        );

        premium.add_rider_premium(RiderPremium {
            rider_code: "WOP".to_string(),
            rider_name: "Waiver of Premium".to_string(),
            amount: Money::new(dec!(100), Currency::USD),
        });

        assert_eq!(
            premium.total_per_payment().amount(),
            dec!(1100),
            "Premium should include rider premium"
        );
    }

    /// Verifies premium with all components combined
    #[test]
    fn test_premium_with_all_components() {
        let mut premium = Premium::new(
            Money::new(dec!(1000), Currency::USD),
            PremiumFrequency::Annual,
        )
        .with_policy_fee(Money::new(dec!(50), Currency::USD))
        .with_tax(Money::new(dec!(100), Currency::USD))
        .with_discount(Discount {
            discount_type: DiscountType::Percentage,
            value: dec!(10), // 10% discount
        });

        premium.add_rider_premium(RiderPremium {
            rider_code: "CI".to_string(),
            rider_name: "Critical Illness".to_string(),
            amount: Money::new(dec!(200), Currency::USD),
        });

        // Calculation: Base 1000 + Rider 200 + Fee 50 + Tax 100 = 1350
        // Discount: 10% of 1350 = 135
        // Total: 1350 - 135 = 1215
        assert_eq!(
            premium.total_per_payment().amount(),
            dec!(1215),
            "Premium calculation with all components failed"
        );
    }

    /// Verifies annualized calculation for monthly frequency
    #[test]
    fn test_annualized_monthly() {
        let premium = Premium::new(
            Money::new(dec!(100), Currency::USD),
            PremiumFrequency::Monthly,
        );

        assert_eq!(
            premium.annualized().amount(),
            dec!(1200),
            "Monthly $100 x 12 = $1200 annualized"
        );
    }

    /// Verifies annualized calculation for quarterly frequency
    #[test]
    fn test_annualized_quarterly() {
        let premium = Premium::new(
            Money::new(dec!(250), Currency::USD),
            PremiumFrequency::Quarterly,
        );

        assert_eq!(
            premium.annualized().amount(),
            dec!(1000),
            "Quarterly $250 x 4 = $1000 annualized"
        );
    }

    /// Verifies annualized calculation for semi-annual frequency
    #[test]
    fn test_annualized_semi_annual() {
        let premium = Premium::new(
            Money::new(dec!(500), Currency::USD),
            PremiumFrequency::SemiAnnual,
        );

        assert_eq!(
            premium.annualized().amount(),
            dec!(1000),
            "Semi-annual $500 x 2 = $1000 annualized"
        );
    }
}

// ============================================================================
// DISCOUNT TESTS
// ============================================================================

mod discount_tests {
    use super::*;

    /// Verifies percentage discount calculation
    #[test]
    fn test_percentage_discount() {
        let discount = Discount {
            discount_type: DiscountType::Percentage,
            value: dec!(10), // 10%
        };

        let base = Money::new(dec!(1000), Currency::USD);
        let discount_amount = discount.calculate(&base);

        assert_eq!(
            discount_amount.amount(),
            dec!(100),
            "10% of 1000 should be 100"
        );
    }

    /// Verifies fixed amount discount calculation
    #[test]
    fn test_fixed_amount_discount() {
        let discount = Discount {
            discount_type: DiscountType::FixedAmount,
            value: dec!(150), // Fixed $150
        };

        let base = Money::new(dec!(1000), Currency::USD);
        let discount_amount = discount.calculate(&base);

        assert_eq!(
            discount_amount.amount(),
            dec!(150),
            "Fixed amount should be exactly 150"
        );
    }

    /// Verifies multi-policy discount calculation
    #[test]
    fn test_multi_policy_discount() {
        let discount = Discount {
            discount_type: DiscountType::MultiPolicy,
            value: dec!(5), // 5%
        };

        let base = Money::new(dec!(1000), Currency::USD);
        let discount_amount = discount.calculate(&base);

        assert_eq!(
            discount_amount.amount(),
            dec!(50),
            "5% multi-policy discount of 1000 should be 50"
        );
    }

    /// Verifies annual payment discount calculation
    #[test]
    fn test_annual_payment_discount() {
        let discount = Discount {
            discount_type: DiscountType::AnnualPayment,
            value: dec!(3), // 3%
        };

        let base = Money::new(dec!(1000), Currency::USD);
        let discount_amount = discount.calculate(&base);

        assert_eq!(
            discount_amount.amount(),
            dec!(30),
            "3% annual payment discount of 1000 should be 30"
        );
    }

    /// Verifies loyalty discount calculation
    #[test]
    fn test_loyalty_discount() {
        let discount = Discount {
            discount_type: DiscountType::Loyalty,
            value: dec!(10), // 10%
        };

        let base = Money::new(dec!(1000), Currency::USD);
        let discount_amount = discount.calculate(&base);

        assert_eq!(
            discount_amount.amount(),
            dec!(100),
            "10% loyalty discount of 1000 should be 100"
        );
    }
}

// ============================================================================
// SCHEDULE TESTS
// ============================================================================

mod schedule_tests {
    use super::*;

    /// Verifies schedule generation for monthly payments over 1 year
    #[test]
    fn test_schedule_generation_monthly() {
        let premium = Premium::new(
            Money::new(dec!(100), Currency::USD),
            PremiumFrequency::Monthly,
        );

        let schedule = premium.generate_schedule(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            1, // 1 year
        );

        assert_eq!(schedule.len(), 12, "Should have 12 monthly payments");

        // Verify first entry
        assert_eq!(schedule[0].sequence_number, 1);
        assert_eq!(
            schedule[0].due_date,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
        );

        // Verify last entry
        assert_eq!(schedule[11].sequence_number, 12);
    }

    /// Verifies schedule generation for quarterly payments over 2 years
    #[test]
    fn test_schedule_generation_quarterly() {
        let premium = Premium::new(
            Money::new(dec!(250), Currency::USD),
            PremiumFrequency::Quarterly,
        );

        let schedule = premium.generate_schedule(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            2, // 2 years
        );

        assert_eq!(
            schedule.len(),
            8,
            "Should have 8 quarterly payments over 2 years"
        );
        assert_eq!(schedule[0].sequence_number, 1);
        assert_eq!(schedule[7].sequence_number, 8);
    }

    /// Verifies schedule generation for annual payments
    #[test]
    fn test_schedule_generation_annual() {
        let premium = Premium::new(
            Money::new(dec!(1000), Currency::USD),
            PremiumFrequency::Annual,
        );

        let schedule = premium.generate_schedule(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            5, // 5 years
        );

        assert_eq!(schedule.len(), 5, "Should have 5 annual payments");
    }

    /// Verifies schedule entry properties
    #[test]
    fn test_schedule_entry_properties() {
        let premium = Premium::new(
            Money::new(dec!(1000), Currency::USD),
            PremiumFrequency::Annual,
        );

        let schedule = premium.generate_schedule(
            NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            1,
        );

        assert_eq!(schedule.len(), 1);
        assert_eq!(schedule[0].sequence_number, 1);
        assert_eq!(
            schedule[0].due_date,
            NaiveDate::from_ymd_opt(2024, 6, 15).unwrap()
        );
        assert_eq!(schedule[0].amount.amount(), dec!(1000));
        assert!(matches!(schedule[0].status, PaymentStatus::Pending));
    }

    /// Verifies all PaymentStatus variants can be created
    #[test]
    fn test_payment_status_variants() {
        assert!(matches!(PaymentStatus::Pending, PaymentStatus::Pending));
        assert!(matches!(PaymentStatus::Paid, PaymentStatus::Paid));
        assert!(matches!(PaymentStatus::Overdue, PaymentStatus::Overdue));
        assert!(matches!(PaymentStatus::Waived, PaymentStatus::Waived));
        assert!(matches!(PaymentStatus::GracePeriod, PaymentStatus::GracePeriod));
    }

    /// Verifies PremiumSchedule struct can hold entries
    #[test]
    fn test_premium_schedule_struct() {
        let schedule = PremiumSchedule { entries: vec![] };
        assert!(schedule.entries.is_empty());
    }
}
