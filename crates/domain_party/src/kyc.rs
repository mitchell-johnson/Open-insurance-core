//! KYC (Know Your Customer) management

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use core_kernel::PartyId;

/// KYC status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KycStatus {
    Pending,
    InProgress,
    Verified,
    Failed,
    Expired,
}

/// Document type for KYC
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentType {
    Passport,
    DriversLicense,
    NationalId,
    ProofOfAddress,
    TaxReturn,
    BankStatement,
    Other(String),
}

/// A KYC document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycDocument {
    pub id: Uuid,
    pub party_id: PartyId,
    pub document_type: DocumentType,
    pub document_number: Option<String>,
    pub issue_date: Option<NaiveDate>,
    pub expiry_date: Option<NaiveDate>,
    pub issuing_country: Option<String>,
    pub verified: bool,
    pub verified_at: Option<DateTime<Utc>>,
    pub verified_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl KycDocument {
    /// Creates a new KYC document
    pub fn new(party_id: PartyId, document_type: DocumentType) -> Self {
        Self {
            id: Uuid::new_v4(),
            party_id,
            document_type,
            document_number: None,
            issue_date: None,
            expiry_date: None,
            issuing_country: None,
            verified: false,
            verified_at: None,
            verified_by: None,
            created_at: Utc::now(),
        }
    }

    /// Marks document as verified
    pub fn verify(&mut self, verifier: &str) {
        self.verified = true;
        self.verified_at = Some(Utc::now());
        self.verified_by = Some(verifier.to_string());
    }

    /// Checks if document is expired
    pub fn is_expired(&self) -> bool {
        self.expiry_date.map_or(false, |exp| exp < Utc::now().date_naive())
    }
}
