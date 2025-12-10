//! Fund allocation logic

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use core_kernel::FundId;
use crate::error::FundError;

/// Fund allocation percentage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Allocation {
    /// Fund ID
    pub fund_id: FundId,
    /// Allocation percentage (0-100)
    pub percentage: Decimal,
}

/// Strategy for allocating premium to funds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationStrategy {
    /// Individual fund allocations
    pub allocations: Vec<Allocation>,
}

impl AllocationStrategy {
    /// Creates a new allocation strategy
    pub fn new(allocations: Vec<Allocation>) -> Result<Self, FundError> {
        let strategy = Self { allocations };
        strategy.validate()?;
        Ok(strategy)
    }

    /// Validates that allocations sum to 100%
    pub fn validate(&self) -> Result<(), FundError> {
        let total: Decimal = self.allocations.iter().map(|a| a.percentage).sum();

        if total != dec!(100) {
            return Err(FundError::InvalidAllocation(format!(
                "Allocations must sum to 100%, got {}%",
                total
            )));
        }

        for allocation in &self.allocations {
            if allocation.percentage < Decimal::ZERO || allocation.percentage > dec!(100) {
                return Err(FundError::InvalidAllocation(format!(
                    "Invalid allocation percentage: {}",
                    allocation.percentage
                )));
            }
        }

        Ok(())
    }

    /// Calculates amount for each fund from a total
    pub fn calculate_amounts(&self, total: Decimal) -> Vec<(FundId, Decimal)> {
        self.allocations
            .iter()
            .map(|a| (a.fund_id, (total * a.percentage / dec!(100)).round_dp(2)))
            .collect()
    }
}
