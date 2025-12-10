//! Fund DTOs

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct RecordNavRequest {
    pub nav_date: NaiveDate,
    pub value: Decimal,
    pub currency: String,
}

#[derive(Debug, Serialize)]
pub struct FundResponse {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub fund_type: String,
    pub risk_level: String,
    pub management_fee: Decimal,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
pub struct NavResponse {
    pub id: Uuid,
    pub fund_id: Uuid,
    pub nav_date: NaiveDate,
    pub value: Decimal,
    pub currency: String,
    pub created_at: DateTime<Utc>,
}
