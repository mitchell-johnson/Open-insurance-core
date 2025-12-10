//! Fund definition and management
//!
//! This module defines the Fund entity and its properties.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use core_kernel::FundId;

/// Types of investment funds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FundType {
    /// Equity/stock fund
    Equity,
    /// Fixed income/bond fund
    Bond,
    /// Balanced (mixed equity and bond) fund
    Balanced,
    /// Money market fund
    MoneyMarket,
    /// Index tracking fund
    Index,
    /// Sector-specific fund
    Sector,
    /// International/global fund
    International,
    /// Guaranteed return fund
    Guaranteed,
}

/// Risk level classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Low risk - capital preservation
    Low = 1,
    /// Medium-low risk
    MediumLow = 2,
    /// Medium risk
    Medium = 3,
    /// Medium-high risk
    MediumHigh = 4,
    /// High risk - aggressive growth
    High = 5,
}

/// An investment fund available for ULIP allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fund {
    /// Unique identifier
    pub id: FundId,
    /// Fund code (short identifier)
    pub code: String,
    /// Fund name
    pub name: String,
    /// Fund description
    pub description: Option<String>,
    /// Fund type
    pub fund_type: FundType,
    /// Currency
    pub currency: String,
    /// Risk level
    pub risk_level: RiskLevel,
    /// Annual management fee (as decimal, e.g., 0.015 = 1.5%)
    pub management_fee: Decimal,
    /// Minimum allocation percentage
    pub min_allocation_percent: Option<Decimal>,
    /// Maximum allocation percentage
    pub max_allocation_percent: Option<Decimal>,
    /// Whether fund accepts new investments
    pub is_open: bool,
    /// Whether fund is active
    pub is_active: bool,
    /// Fund launch date
    pub launch_date: Option<DateTime<Utc>>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

impl Fund {
    /// Creates a new fund
    ///
    /// # Arguments
    ///
    /// * `code` - Short fund identifier
    /// * `name` - Fund name
    /// * `fund_type` - Type of fund
    /// * `risk_level` - Risk classification
    pub fn new(
        code: impl Into<String>,
        name: impl Into<String>,
        fund_type: FundType,
        risk_level: RiskLevel,
    ) -> Self {
        Self {
            id: FundId::new_v7(),
            code: code.into(),
            name: name.into(),
            description: None,
            fund_type,
            currency: "USD".to_string(),
            risk_level,
            management_fee: Decimal::ZERO,
            min_allocation_percent: None,
            max_allocation_percent: None,
            is_open: true,
            is_active: true,
            launch_date: None,
            created_at: Utc::now(),
        }
    }

    /// Sets the management fee
    pub fn with_management_fee(mut self, fee: Decimal) -> Self {
        self.management_fee = fee;
        self
    }

    /// Sets the currency
    pub fn with_currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = currency.into();
        self
    }

    /// Sets allocation constraints
    pub fn with_allocation_limits(mut self, min: Decimal, max: Decimal) -> Self {
        self.min_allocation_percent = Some(min);
        self.max_allocation_percent = Some(max);
        self
    }

    /// Checks if an allocation percentage is valid for this fund
    pub fn validate_allocation(&self, percent: Decimal) -> bool {
        let min_ok = self.min_allocation_percent.map_or(true, |min| percent >= min);
        let max_ok = self.max_allocation_percent.map_or(true, |max| percent <= max);
        min_ok && max_ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_fund_creation() {
        let fund = Fund::new("EQ001", "Global Equity Fund", FundType::Equity, RiskLevel::High)
            .with_management_fee(dec!(0.015));

        assert_eq!(fund.code, "EQ001");
        assert_eq!(fund.fund_type, FundType::Equity);
        assert!(fund.is_active);
    }

    #[test]
    fn test_allocation_validation() {
        let fund = Fund::new("BAL001", "Balanced Fund", FundType::Balanced, RiskLevel::Medium)
            .with_allocation_limits(dec!(10), dec!(50));

        assert!(fund.validate_allocation(dec!(30)));
        assert!(!fund.validate_allocation(dec!(5)));
        assert!(!fund.validate_allocation(dec!(60)));
    }
}
