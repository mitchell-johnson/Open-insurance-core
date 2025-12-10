//! Party entity

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

use core_kernel::PartyId;
use crate::address::Address;
use crate::kyc::KycStatus;

/// Party type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartyType {
    Individual,
    Corporate,
    Agent,
    Broker,
}

/// Individual person details
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Individual {
    pub first_name: String,
    pub middle_name: Option<String>,
    pub last_name: String,
    pub date_of_birth: NaiveDate,
    pub gender: Option<Gender>,
    pub nationality: Option<String>,
    pub tax_id: Option<String>,
    pub occupation: Option<String>,
}

/// Gender
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
    Other,
}

/// Corporate entity details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Corporate {
    pub company_name: String,
    pub registration_number: Option<String>,
    pub tax_id: Option<String>,
    pub industry: Option<String>,
    pub incorporation_date: Option<NaiveDate>,
    pub incorporation_country: Option<String>,
}

/// A party (person or organization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    pub id: PartyId,
    pub party_type: PartyType,
    pub individual: Option<Individual>,
    pub corporate: Option<Corporate>,
    #[serde(default)]
    pub addresses: Vec<Address>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub preferred_language: Option<String>,
    pub kyc_status: KycStatus,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Party {
    /// Creates a new individual party
    pub fn new_individual(individual: Individual) -> Self {
        let now = Utc::now();
        Self {
            id: PartyId::new_v7(),
            party_type: PartyType::Individual,
            individual: Some(individual),
            corporate: None,
            addresses: Vec::new(),
            email: None,
            phone: None,
            preferred_language: None,
            kyc_status: KycStatus::Pending,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a new corporate party
    pub fn new_corporate(corporate: Corporate) -> Self {
        let now = Utc::now();
        Self {
            id: PartyId::new_v7(),
            party_type: PartyType::Corporate,
            individual: None,
            corporate: Some(corporate),
            addresses: Vec::new(),
            email: None,
            phone: None,
            preferred_language: None,
            kyc_status: KycStatus::Pending,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Returns the display name
    pub fn display_name(&self) -> String {
        match &self.party_type {
            PartyType::Individual => {
                if let Some(ind) = &self.individual {
                    format!("{} {}", ind.first_name, ind.last_name)
                } else {
                    "Unknown".to_string()
                }
            }
            PartyType::Corporate => {
                self.corporate.as_ref().map(|c| c.company_name.clone()).unwrap_or_default()
            }
            _ => "Unknown".to_string(),
        }
    }

    /// Adds an address
    pub fn add_address(&mut self, address: Address) {
        self.addresses.push(address);
        self.updated_at = Utc::now();
    }
}
