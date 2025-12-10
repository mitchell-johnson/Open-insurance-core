//! Party handlers

use axum::{extract::{Path, State}, Json};
use uuid::Uuid;

use crate::{AppState, error::ApiError};
use crate::dto::party::*;

/// Creates a new party
pub async fn create_party(
    State(_state): State<AppState>,
    Json(_request): Json<CreatePartyRequest>,
) -> Result<Json<PartyResponse>, ApiError> {
    Err(ApiError::Internal("Not implemented".to_string()))
}

/// Lists parties
pub async fn list_parties(
    State(_state): State<AppState>,
) -> Result<Json<Vec<PartyResponse>>, ApiError> {
    Ok(Json(vec![]))
}

/// Gets a party by ID
pub async fn get_party(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> Result<Json<PartyResponse>, ApiError> {
    Err(ApiError::NotFound("Party not found".to_string()))
}

/// Updates a party
pub async fn update_party(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
    Json(_request): Json<UpdatePartyRequest>,
) -> Result<Json<PartyResponse>, ApiError> {
    Err(ApiError::Internal("Not implemented".to_string()))
}

/// Submits KYC documents
pub async fn submit_kyc(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
    Json(_request): Json<SubmitKycRequest>,
) -> Result<Json<KycResponse>, ApiError> {
    Err(ApiError::Internal("Not implemented".to_string()))
}
