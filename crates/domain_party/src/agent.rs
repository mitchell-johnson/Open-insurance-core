//! Insurance agent management

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use core_kernel::{AgentId, PartyId};

/// Agent status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Active,
    Inactive,
    Suspended,
    Terminated,
}

/// An insurance agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub party_id: PartyId,
    pub agent_code: String,
    pub license_number: Option<String>,
    pub license_expiry: Option<NaiveDate>,
    pub appointed_date: NaiveDate,
    pub status: AgentStatus,
    pub default_commission_rate: Option<Decimal>,
    pub territory: Option<String>,
    pub manager_id: Option<AgentId>,
    pub created_at: DateTime<Utc>,
}

impl Agent {
    /// Creates a new agent
    pub fn new(party_id: PartyId, agent_code: impl Into<String>) -> Self {
        Self {
            id: AgentId::new_v7(),
            party_id,
            agent_code: agent_code.into(),
            license_number: None,
            license_expiry: None,
            appointed_date: Utc::now().date_naive(),
            status: AgentStatus::Active,
            default_commission_rate: None,
            territory: None,
            manager_id: None,
            created_at: Utc::now(),
        }
    }

    /// Checks if license is valid
    pub fn is_licensed(&self) -> bool {
        match self.license_expiry {
            Some(exp) => exp >= Utc::now().date_naive(),
            None => self.license_number.is_none(), // No license required
        }
    }
}
