//! Claims handlers

use axum::{extract::{Path, State}, Json};
use uuid::Uuid;

use crate::{AppState, error::ApiError};
use crate::dto::claims::*;

/// Creates a new FNOL
pub async fn create_fnol(
    State(_state): State<AppState>,
    Json(_request): Json<CreateFnolRequest>,
) -> Result<Json<ClaimResponse>, ApiError> {
    Err(ApiError::Internal("Not implemented".to_string()))
}

/// Lists claims
pub async fn list_claims(
    State(_state): State<AppState>,
) -> Result<Json<Vec<ClaimResponse>>, ApiError> {
    Ok(Json(vec![]))
}

/// Gets a claim by ID
pub async fn get_claim(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> Result<Json<ClaimResponse>, ApiError> {
    Err(ApiError::NotFound("Claim not found".to_string()))
}

/// Updates claim status
pub async fn update_status(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
    Json(_request): Json<UpdateStatusRequest>,
) -> Result<Json<ClaimResponse>, ApiError> {
    Err(ApiError::Internal("Not implemented".to_string()))
}

/// Adds a reserve
pub async fn add_reserve(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
    Json(_request): Json<AddReserveRequest>,
) -> Result<Json<ReserveResponse>, ApiError> {
    Err(ApiError::Internal("Not implemented".to_string()))
}

/// Adds a payment
pub async fn add_payment(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
    Json(_request): Json<AddPaymentRequest>,
) -> Result<Json<PaymentResponse>, ApiError> {
    Err(ApiError::Internal("Not implemented".to_string()))
}
