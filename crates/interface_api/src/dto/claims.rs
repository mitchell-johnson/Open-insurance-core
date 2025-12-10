//! Claims DTOs

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateFnolRequest {
    pub policy_id: Uuid,
    pub claimant_id: Uuid,
    pub loss_date: NaiveDate,
    pub loss_type: String,
    pub description: Option<String>,
    pub claimed_amount: Option<Decimal>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: String,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddReserveRequest {
    pub reserve_type: String,
    pub amount: Decimal,
    pub currency: String,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddPaymentRequest {
    pub payee_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub payment_type: String,
    pub payment_method: String,
}

#[derive(Debug, Serialize)]
pub struct ClaimResponse {
    pub id: Uuid,
    pub claim_number: String,
    pub policy_id: Uuid,
    pub status: String,
    pub loss_date: NaiveDate,
    pub claimed_amount: Option<Decimal>,
    pub approved_amount: Option<Decimal>,
    pub paid_amount: Decimal,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ReserveResponse {
    pub id: Uuid,
    pub reserve_type: String,
    pub amount: Decimal,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct PaymentResponse {
    pub id: Uuid,
    pub amount: Decimal,
    pub payment_type: String,
    pub paid_at: DateTime<Utc>,
}
