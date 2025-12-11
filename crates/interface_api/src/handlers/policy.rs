//! Policy handlers

use axum::{extract::{Path, State}, Json};
use uuid::Uuid;

use crate::{AppState, error::ApiError};
use crate::dto::policy::*;

/// Creates a new policy quote
pub async fn create_quote(
    State(_state): State<AppState>,
    Json(_request): Json<CreateQuoteRequest>,
) -> Result<Json<PolicyResponse>, ApiError> {
    // TODO: Implement policy creation
    Err(ApiError::Internal("Not implemented".to_string()))
}

/// Lists policies
pub async fn list_policies(
    State(_state): State<AppState>,
) -> Result<Json<Vec<PolicyResponse>>, ApiError> {
    Ok(Json(vec![]))
}

/// Gets a policy by ID
pub async fn get_policy(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> Result<Json<PolicyResponse>, ApiError> {
    Err(ApiError::NotFound("Policy not found".to_string()))
}

/// Updates a policy
pub async fn update_policy(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
    Json(_request): Json<UpdatePolicyRequest>,
) -> Result<Json<PolicyResponse>, ApiError> {
    Err(ApiError::Internal("Not implemented".to_string()))
}

/// Issues a policy
pub async fn issue_policy(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
    Json(_request): Json<IssuePolicyRequest>,
) -> Result<Json<PolicyResponse>, ApiError> {
    Err(ApiError::Internal("Not implemented".to_string()))
}

/// Creates an endorsement
pub async fn create_endorsement(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
    Json(_request): Json<CreateEndorsementRequest>,
) -> Result<Json<EndorsementResponse>, ApiError> {
    Err(ApiError::Internal("Not implemented".to_string()))
}
