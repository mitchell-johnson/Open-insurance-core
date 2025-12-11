//! Premium calculations and schedules
//!
//! This module handles premium-related value objects and calculations.

use chrono::{NaiveDate, Datelike};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use core_kernel::Money;

/// Premium payment frequency
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PremiumFrequency {
    /// Single premium payment
    Single,
    /// Annual premium
    Annual,
    /// Semi-annual premium (twice per year)
    SemiAnnual,
    /// Quarterly premium
    Quarterly,
    /// Monthly premium
    Monthly,
}

impl PremiumFrequency {
    /// Returns the number of payments per year
    pub fn payments_per_year(&self) -> u32 {
        match self {
            PremiumFrequency::Single => 1,
            PremiumFrequency::Annual => 1,
            PremiumFrequency::SemiAnnual => 2,
            PremiumFrequency::Quarterly => 4,
            PremiumFrequency::Monthly => 12,
        }
    }

    /// Returns the modal factor for this frequency
    ///
    /// Modal factor adjusts annual premium to account for
    /// more frequent payment options (typically slightly higher)
    pub fn modal_factor(&self) -> Decimal {
        match self {
            PremiumFrequency::Single => dec!(1.0),
            PremiumFrequency::Annual => dec!(1.0),
            PremiumFrequency::SemiAnnual => dec!(0.5125), // ~2.5% loading
            PremiumFrequency::Quarterly => dec!(0.2625),  // ~5% loading
            PremiumFrequency::Monthly => dec!(0.0875),    // ~5% loading
        }
    }

    /// Calculates the next due date from a given date
    ///
    /// # Arguments
    ///
    /// * `from_date` - The reference date
    ///
    /// # Returns
    ///
    /// The next premium due date
    pub fn next_due_date(&self, from_date: NaiveDate) -> NaiveDate {
        match self {
            PremiumFrequency::Single => from_date,
            PremiumFrequency::Annual => {
                NaiveDate::from_ymd_opt(from_date.year() + 1, from_date.month(), from_date.day())
                    .unwrap_or(from_date + chrono::Duration::days(365))
            }
            PremiumFrequency::SemiAnnual => from_date + chrono::Duration::days(182),
            PremiumFrequency::Quarterly => from_date + chrono::Duration::days(91),
            PremiumFrequency::Monthly => {
                let next_month = if from_date.month() == 12 {
                    NaiveDate::from_ymd_opt(from_date.year() + 1, 1, from_date.day())
                } else {
                    NaiveDate::from_ymd_opt(from_date.year(), from_date.month() + 1, from_date.day())
                };
                next_month.unwrap_or(from_date + chrono::Duration::days(30))
            }
        }
    }
}

/// Premium information for a policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Premium {
    /// Base premium amount (per payment)
    pub base_amount: Money,
    /// Payment frequency
    pub frequency: PremiumFrequency,
    /// Policy fee (per payment)
    pub policy_fee: Option<Money>,
    /// Rider premiums
    pub rider_premiums: Vec<RiderPremium>,
    /// Tax amount
    pub tax: Option<Money>,
    /// Discount applied
    pub discount: Option<Discount>,
}

impl Premium {
    /// Creates a new premium with basic settings
    ///
    /// # Arguments
    ///
    /// * `amount` - The premium amount per payment
    /// * `frequency` - Payment frequency
    pub fn new(amount: Money, frequency: PremiumFrequency) -> Self {
        Self {
            base_amount: amount,
            frequency,
            policy_fee: None,
            rider_premiums: Vec::new(),
            tax: None,
            discount: None,
        }
    }

    /// Adds a policy fee
    pub fn with_policy_fee(mut self, fee: Money) -> Self {
        self.policy_fee = Some(fee);
        self
    }

    /// Adds a rider premium
    pub fn add_rider_premium(&mut self, rider: RiderPremium) {
        self.rider_premiums.push(rider);
    }

    /// Adds tax
    pub fn with_tax(mut self, tax: Money) -> Self {
        self.tax = Some(tax);
        self
    }

    /// Applies a discount
    pub fn with_discount(mut self, discount: Discount) -> Self {
        self.discount = Some(discount);
        self
    }

    /// Calculates total premium per payment
    ///
    /// # Returns
    ///
    /// Total premium including base, riders, fees, tax, and discount
    pub fn total_per_payment(&self) -> Money {
        let mut total = self.base_amount;

        // Add rider premiums
        for rider in &self.rider_premiums {
            total = total + rider.amount;
        }

        // Add policy fee
        if let Some(fee) = &self.policy_fee {
            total = total + *fee;
        }

        // Add tax
        if let Some(tax) = &self.tax {
            total = total + *tax;
        }

        // Apply discount
        if let Some(discount) = &self.discount {
            let discount_amount = discount.calculate(&total);
            total = total - discount_amount;
        }

        total
    }

    /// Calculates annualized premium
    ///
    /// # Returns
    ///
    /// The total premium for one year
    pub fn annualized(&self) -> Money {
        let per_payment = self.total_per_payment();
        let factor = Decimal::from(self.frequency.payments_per_year());
        per_payment * factor
    }

    /// Generates a premium schedule for the given period
    ///
    /// # Arguments
    ///
    /// * `start_date` - Policy effective date
    /// * `years` - Number of years to generate
    ///
    /// # Returns
    ///
    /// A vector of scheduled premium payments
    pub fn generate_schedule(&self, start_date: NaiveDate, years: u32) -> Vec<PremiumScheduleEntry> {
        let mut schedule = Vec::new();
        let mut current_date = start_date;
        let total_payments = years * self.frequency.payments_per_year();

        for sequence in 1..=total_payments {
            schedule.push(PremiumScheduleEntry {
                sequence_number: sequence,
                due_date: current_date,
                amount: self.total_per_payment(),
                status: PaymentStatus::Pending,
            });

            current_date = self.frequency.next_due_date(current_date);
        }

        schedule
    }
}

/// Rider premium component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiderPremium {
    /// Rider code
    pub rider_code: String,
    /// Rider name
    pub rider_name: String,
    /// Premium amount
    pub amount: Money,
}

/// Discount applied to premium
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discount {
    /// Discount type
    pub discount_type: DiscountType,
    /// Discount value (percentage or fixed amount)
    pub value: Decimal,
}

/// Types of discounts
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscountType {
    /// Percentage discount
    Percentage,
    /// Fixed amount discount
    FixedAmount,
    /// Multi-policy discount
    MultiPolicy,
    /// Annual payment discount
    AnnualPayment,
    /// Loyalty discount
    Loyalty,
}

impl Discount {
    /// Calculates the discount amount
    ///
    /// # Arguments
    ///
    /// * `base` - The base amount to apply discount to
    ///
    /// # Returns
    ///
    /// The discount amount
    pub fn calculate(&self, base: &Money) -> Money {
        match self.discount_type {
            DiscountType::Percentage | DiscountType::MultiPolicy |
            DiscountType::AnnualPayment | DiscountType::Loyalty => {
                base.multiply(self.value / dec!(100))
            }
            DiscountType::FixedAmount => {
                Money::new(self.value, base.currency())
            }
        }
    }
}

/// A single entry in the premium schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PremiumScheduleEntry {
    /// Sequence number (1, 2, 3, ...)
    pub sequence_number: u32,
    /// Due date
    pub due_date: NaiveDate,
    /// Amount due
    pub amount: Money,
    /// Payment status
    pub status: PaymentStatus,
}

/// Premium schedule for a policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PremiumSchedule {
    /// Schedule entries
    pub entries: Vec<PremiumScheduleEntry>,
}

/// Status of a premium payment
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentStatus {
    /// Payment is due but not yet made
    Pending,
    /// Payment has been made
    Paid,
    /// Payment is overdue
    Overdue,
    /// Payment was waived
    Waived,
    /// Payment is in grace period
    GracePeriod,
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_kernel::Currency;
    use rust_decimal_macros::dec;

    #[test]
    fn test_premium_calculation() {
        let premium = Premium::new(
            Money::new(dec!(1000), Currency::USD),
            PremiumFrequency::Annual,
        );

        assert_eq!(premium.total_per_payment().amount(), dec!(1000));
        assert_eq!(premium.annualized().amount(), dec!(1000));
    }

    #[test]
    fn test_premium_with_discount() {
        let premium = Premium::new(
            Money::new(dec!(1000), Currency::USD),
            PremiumFrequency::Annual,
        ).with_discount(Discount {
            discount_type: DiscountType::Percentage,
            value: dec!(10), // 10% discount
        });

        assert_eq!(premium.total_per_payment().amount(), dec!(900));
    }

    #[test]
    fn test_premium_schedule() {
        let premium = Premium::new(
            Money::new(dec!(100), Currency::USD),
            PremiumFrequency::Monthly,
        );

        let schedule = premium.generate_schedule(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            1, // 1 year
        );

        assert_eq!(schedule.len(), 12);
    }

    #[test]
    fn test_frequency_next_due_date() {
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        let next_annual = PremiumFrequency::Annual.next_due_date(date);
        assert_eq!(next_annual, NaiveDate::from_ymd_opt(2025, 1, 15).unwrap());

        let next_monthly = PremiumFrequency::Monthly.next_due_date(date);
        assert_eq!(next_monthly, NaiveDate::from_ymd_opt(2024, 2, 15).unwrap());
    }
}
