//! Party DTOs

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreatePartyRequest {
    pub party_type: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub company_name: Option<String>,
    pub date_of_birth: Option<NaiveDate>,
    pub email: Option<String>,
    pub phone: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePartyRequest {
    pub email: Option<String>,
    pub phone: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitKycRequest {
    pub document_type: String,
    pub document_number: Option<String>,
    pub expiry_date: Option<NaiveDate>,
}

#[derive(Debug, Serialize)]
pub struct PartyResponse {
    pub id: Uuid,
    pub party_type: String,
    pub display_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub kyc_status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct KycResponse {
    pub id: Uuid,
    pub document_type: String,
    pub verified: bool,
    pub created_at: DateTime<Utc>,
}
