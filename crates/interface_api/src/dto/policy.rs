//! Policy DTOs

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateQuoteRequest {
    pub product_code: String,
    pub policyholder_id: Uuid,
    pub coverages: Vec<CoverageRequest>,
    pub effective_date: NaiveDate,
    pub term_years: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct CoverageRequest {
    pub coverage_type: String,
    pub sum_assured: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePolicyRequest {
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct IssuePolicyRequest {
    pub effective_date: NaiveDate,
    pub underwriter: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateEndorsementRequest {
    pub endorsement_type: String,
    pub effective_date: NaiveDate,
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct PolicyResponse {
    pub id: Uuid,
    pub policy_number: String,
    pub product_code: String,
    pub status: String,
    pub effective_date: Option<NaiveDate>,
    pub expiry_date: Option<NaiveDate>,
    pub premium: Decimal,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct EndorsementResponse {
    pub id: Uuid,
    pub endorsement_number: String,
    pub status: String,
    pub effective_date: NaiveDate,
}
