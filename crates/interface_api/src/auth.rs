//! Authentication and authorization

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// JWT claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// User's roles
    pub roles: Vec<String>,
    /// Expiration timestamp
    pub exp: i64,
    /// Issued at timestamp
    pub iat: i64,
}

/// Auth errors
#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid token")]
    InvalidToken,
    #[error("Token expired")]
    TokenExpired,
    #[error("Missing permission: {0}")]
    MissingPermission(String),
}

/// Creates a new JWT token
///
/// # Arguments
///
/// * `user_id` - User identifier
/// * `roles` - User's roles
/// * `secret` - JWT secret key
/// * `expiration_secs` - Token validity in seconds
pub fn create_token(
    user_id: &str,
    roles: Vec<String>,
    secret: &str,
    expiration_secs: u64,
) -> Result<String, AuthError> {
    let now = Utc::now();
    let exp = now + Duration::seconds(expiration_secs as i64);

    let claims = Claims {
        sub: user_id.to_string(),
        roles,
        exp: exp.timestamp(),
        iat: now.timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| AuthError::InvalidToken)
}

/// Validates a JWT token
///
/// # Arguments
///
/// * `token` - The JWT token to validate
/// * `secret` - JWT secret key
pub fn validate_token(token: &str, secret: &str) -> Result<Claims, AuthError> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| {
        if e.to_string().contains("ExpiredSignature") {
            AuthError::TokenExpired
        } else {
            AuthError::InvalidToken
        }
    })?;

    Ok(token_data.claims)
}

/// Checks if user has required role
pub fn has_role(claims: &Claims, required_role: &str) -> bool {
    claims.roles.iter().any(|r| r == required_role || r == "admin")
}

/// Permission definitions
pub mod permissions {
    pub const POLICY_READ: &str = "policy:read";
    pub const POLICY_WRITE: &str = "policy:write";
    pub const POLICY_BIND: &str = "policy:bind";
    pub const CLAIM_READ: &str = "claim:read";
    pub const CLAIM_WRITE: &str = "claim:write";
    pub const CLAIM_APPROVE: &str = "claim:approve";
    pub const PARTY_READ: &str = "party:read";
    pub const PARTY_WRITE: &str = "party:write";
    pub const FUND_READ: &str = "fund:read";
    pub const FUND_WRITE: &str = "fund:write";
}
