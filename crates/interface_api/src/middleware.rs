//! API middleware

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use chrono::Utc;
use tracing::{info, warn};

use crate::AppState;
use crate::auth::Claims;

/// Authentication middleware
///
/// Validates JWT tokens and extracts user claims
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract token from Authorization header
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => &header[7..],
        _ => {
            warn!("Missing or invalid Authorization header");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // Validate token
    match crate::auth::validate_token(token, &state.config.jwt_secret) {
        Ok(claims) => {
            // Add claims to request extensions
            request.extensions_mut().insert(claims);
            Ok(next.run(request).await)
        }
        Err(e) => {
            warn!("Token validation failed: {:?}", e);
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// Audit logging middleware
///
/// Logs all API requests for compliance and debugging
pub async fn audit_middleware(
    State(_state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let user_id = request
        .extensions()
        .get::<Claims>()
        .map(|c| c.sub.clone())
        .unwrap_or_else(|| "anonymous".to_string());

    let start = Utc::now();

    let response = next.run(request).await;

    let duration = Utc::now() - start;
    let status = response.status();

    info!(
        method = %method,
        uri = %uri,
        user = %user_id,
        status = %status.as_u16(),
        duration_ms = duration.num_milliseconds(),
        "API request"
    );

    response
}
