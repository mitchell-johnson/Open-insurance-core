//! HTTP API Layer
//!
//! This crate provides the REST API for the insurance core system using Axum.
//!
//! # Architecture
//!
//! - **Handlers**: Request handlers for each domain
//! - **Middleware**: Authentication, authorization, tracing, audit logging
//! - **DTOs**: Request/Response data transfer objects
//! - **Error Handling**: Consistent error responses
//!
//! # Example
//!
//! ```rust,ignore
//! use interface_api::create_router;
//!
//! let app = create_router(pool, config);
//! axum::serve(listener, app).await?;
//! ```

pub mod config;
pub mod error;
pub mod middleware;
pub mod handlers;
pub mod dto;
pub mod auth;

use axum::{
    Router,
    routing::{get, post, put, delete},
    middleware as axum_middleware,
};
use sqlx::PgPool;
use tower_http::trace::TraceLayer;
use tower_http::cors::{CorsLayer, Any};

use crate::config::ApiConfig;
use crate::middleware::{auth_middleware, audit_middleware};
use crate::handlers::{policy, claims, party, fund, health};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: ApiConfig,
}

/// Creates the main API router
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `config` - API configuration
///
/// # Returns
///
/// Configured Axum router with all routes and middleware
pub fn create_router(pool: PgPool, config: ApiConfig) -> Router {
    let state = AppState { pool, config };

    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/health", get(health::health_check))
        .route("/health/ready", get(health::readiness_check));

    // Policy routes
    let policy_routes = Router::new()
        .route("/", post(policy::create_quote))
        .route("/", get(policy::list_policies))
        .route("/:id", get(policy::get_policy))
        .route("/:id", put(policy::update_policy))
        .route("/:id/issue", post(policy::issue_policy))
        .route("/:id/endorsements", post(policy::create_endorsement));

    // Claims routes
    let claims_routes = Router::new()
        .route("/", post(claims::create_fnol))
        .route("/", get(claims::list_claims))
        .route("/:id", get(claims::get_claim))
        .route("/:id/status", put(claims::update_status))
        .route("/:id/reserves", post(claims::add_reserve))
        .route("/:id/payments", post(claims::add_payment));

    // Party routes
    let party_routes = Router::new()
        .route("/", post(party::create_party))
        .route("/", get(party::list_parties))
        .route("/:id", get(party::get_party))
        .route("/:id", put(party::update_party))
        .route("/:id/kyc", post(party::submit_kyc));

    // Fund routes
    let fund_routes = Router::new()
        .route("/", get(fund::list_funds))
        .route("/:id/nav", get(fund::get_nav))
        .route("/:id/nav", post(fund::record_nav));

    // Protected API routes
    let api_routes = Router::new()
        .nest("/policies", policy_routes)
        .nest("/claims", claims_routes)
        .nest("/parties", party_routes)
        .nest("/funds", fund_routes)
        .layer(axum_middleware::from_fn_with_state(state.clone(), audit_middleware))
        .layer(axum_middleware::from_fn_with_state(state.clone(), auth_middleware));

    // Combine all routes
    Router::new()
        .merge(public_routes)
        .nest("/api/v1", api_routes)
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state)
}
