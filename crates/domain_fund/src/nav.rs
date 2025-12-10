//! Net Asset Value (NAV) management
//!
//! This module handles NAV pricing and history for funds.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, NaiveDate, Utc};

use core_kernel::{FundId, NavId};

/// A single NAV price point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nav {
    /// Unique identifier
    pub id: NavId,
    /// Fund ID
    pub fund_id: FundId,
    /// Date of the NAV
    pub nav_date: NaiveDate,
    /// NAV value per unit
    pub value: Decimal,
    /// Currency
    pub currency: String,
    /// Total fund assets under management
    pub aum: Option<Decimal>,
    /// Bid price (for dual pricing)
    pub bid_price: Option<Decimal>,
    /// Offer price (for dual pricing)
    pub offer_price: Option<Decimal>,
    /// Source of NAV data
    pub source: Option<String>,
    /// When this NAV was recorded
    pub created_at: DateTime<Utc>,
}

impl Nav {
    /// Creates a new NAV record
    ///
    /// # Arguments
    ///
    /// * `fund_id` - The fund this NAV is for
    /// * `nav_date` - The valuation date
    /// * `value` - The NAV value per unit
    /// * `currency` - The currency
    pub fn new(fund_id: FundId, nav_date: NaiveDate, value: Decimal, currency: impl Into<String>) -> Self {
        Self {
            id: NavId::new_v7(),
            fund_id,
            nav_date,
            value,
            currency: currency.into(),
            aum: None,
            bid_price: None,
            offer_price: None,
            source: None,
            created_at: Utc::now(),
        }
    }

    /// Sets dual pricing (bid/offer)
    pub fn with_dual_pricing(mut self, bid: Decimal, offer: Decimal) -> Self {
        self.bid_price = Some(bid);
        self.offer_price = Some(offer);
        self
    }

    /// Sets assets under management
    pub fn with_aum(mut self, aum: Decimal) -> Self {
        self.aum = Some(aum);
        self
    }
}

/// NAV history for performance calculations
#[derive(Debug)]
pub struct NavHistory {
    pub fund_id: FundId,
    pub navs: Vec<Nav>,
}

impl NavHistory {
    /// Creates a new NAV history
    pub fn new(fund_id: FundId) -> Self {
        Self {
            fund_id,
            navs: Vec::new(),
        }
    }

    /// Adds a NAV record
    pub fn add(&mut self, nav: Nav) {
        self.navs.push(nav);
        self.navs.sort_by(|a, b| a.nav_date.cmp(&b.nav_date));
    }

    /// Gets the latest NAV
    pub fn latest(&self) -> Option<&Nav> {
        self.navs.last()
    }

    /// Gets NAV for a specific date
    pub fn at_date(&self, date: NaiveDate) -> Option<&Nav> {
        self.navs.iter().find(|n| n.nav_date == date)
    }

    /// Calculates return between two dates
    pub fn calculate_return(&self, from: NaiveDate, to: NaiveDate) -> Option<Decimal> {
        let start_nav = self.at_date(from)?;
        let end_nav = self.at_date(to)?;

        if start_nav.value.is_zero() {
            return None;
        }

        Some((end_nav.value - start_nav.value) / start_nav.value)
    }
}
