//! Unit holdings for policyholders

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use core_kernel::{FundId, PolicyId, UnitHoldingId};
use crate::{round_units, calculate_value};

/// A policyholder's unit holding in a fund
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitHolding {
    /// Unique identifier
    pub id: UnitHoldingId,
    /// Policy ID
    pub policy_id: PolicyId,
    /// Fund ID
    pub fund_id: FundId,
    /// Number of units held
    pub units: Decimal,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl UnitHolding {
    /// Creates a new unit holding
    pub fn new(policy_id: PolicyId, fund_id: FundId) -> Self {
        let now = Utc::now();
        Self {
            id: UnitHoldingId::new_v7(),
            policy_id,
            fund_id,
            units: Decimal::ZERO,
            created_at: now,
            updated_at: now,
        }
    }

    /// Adds units to the holding
    pub fn add_units(&mut self, units: Decimal) {
        self.units = round_units(self.units + units);
        self.updated_at = Utc::now();
    }

    /// Removes units from the holding
    pub fn remove_units(&mut self, units: Decimal) -> Result<(), &'static str> {
        if units > self.units {
            return Err("Insufficient units");
        }
        self.units = round_units(self.units - units);
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Calculates current value at given NAV
    pub fn value_at_nav(&self, nav: Decimal) -> Decimal {
        calculate_value(self.units, nav)
    }
}
