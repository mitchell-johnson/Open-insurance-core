//! Fund handlers

use axum::{extract::{Path, State}, Json};
use uuid::Uuid;

use crate::{AppState, error::ApiError};
use crate::dto::fund::*;

/// Lists funds
pub async fn list_funds(
    State(_state): State<AppState>,
) -> Result<Json<Vec<FundResponse>>, ApiError> {
    Ok(Json(vec![]))
}

/// Gets NAV for a fund
pub async fn get_nav(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
) -> Result<Json<NavResponse>, ApiError> {
    Err(ApiError::NotFound("NAV not found".to_string()))
}

/// Records NAV for a fund
pub async fn record_nav(
    State(_state): State<AppState>,
    Path(_id): Path<Uuid>,
    Json(_request): Json<RecordNavRequest>,
) -> Result<Json<NavResponse>, ApiError> {
    Err(ApiError::Internal("Not implemented".to_string()))
}
