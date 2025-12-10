//! Database error types
//!
//! This module defines the error types that can occur during database operations,
//! providing meaningful error messages and proper error chaining.

use thiserror::Error;

/// Errors that can occur during database operations
///
/// This enum captures all possible database-related errors, including
/// connection issues, query failures, and constraint violations.
#[derive(Debug, Error)]
pub enum DatabaseError {
    /// Failed to establish a database connection
    #[error("Failed to connect to database: {0}")]
    ConnectionFailed(String),

    /// Query execution failed
    #[error("Query failed: {0}")]
    QueryFailed(String),

    /// Entity not found in database
    #[error("Entity not found: {0}")]
    NotFound(String),

    /// Unique constraint violation
    #[error("Duplicate entry: {0}")]
    DuplicateEntry(String),

    /// Foreign key constraint violation
    #[error("Foreign key violation: {0}")]
    ForeignKeyViolation(String),

    /// Check constraint violation
    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),

    /// Exclusion constraint violation (used for temporal overlaps)
    #[error("Temporal overlap detected: {0}")]
    TemporalOverlap(String),

    /// Transaction error
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    /// Migration error
    #[error("Migration failed: {0}")]
    MigrationFailed(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Pool exhaustion - no available connections
    #[error("Connection pool exhausted")]
    PoolExhausted,

    /// Generic SQL error
    #[error("SQL error: {0}")]
    SqlError(#[from] sqlx::Error),
}

impl DatabaseError {
    /// Creates a not found error for a specific entity type and identifier
    ///
    /// # Arguments
    ///
    /// * `entity` - The type of entity (e.g., "Policy", "Claim")
    /// * `id` - The identifier that was not found
    ///
    /// # Example
    ///
    /// ```rust
    /// use infra_db::DatabaseError;
    ///
    /// let error = DatabaseError::not_found("Policy", "POL-123");
    /// assert!(error.to_string().contains("Policy"));
    /// ```
    pub fn not_found(entity: &str, id: impl std::fmt::Display) -> Self {
        DatabaseError::NotFound(format!("{} with id '{}' not found", entity, id))
    }

    /// Creates a duplicate entry error
    ///
    /// # Arguments
    ///
    /// * `entity` - The type of entity
    /// * `field` - The field that caused the duplicate
    /// * `value` - The duplicate value
    pub fn duplicate(entity: &str, field: &str, value: impl std::fmt::Display) -> Self {
        DatabaseError::DuplicateEntry(format!(
            "{} with {} '{}' already exists",
            entity, field, value
        ))
    }

    /// Checks if this error indicates a record was not found
    pub fn is_not_found(&self) -> bool {
        matches!(self, DatabaseError::NotFound(_))
    }

    /// Checks if this error is a constraint violation
    pub fn is_constraint_violation(&self) -> bool {
        matches!(
            self,
            DatabaseError::DuplicateEntry(_)
                | DatabaseError::ForeignKeyViolation(_)
                | DatabaseError::ConstraintViolation(_)
                | DatabaseError::TemporalOverlap(_)
        )
    }

    /// Checks if this error is a connection-related issue
    pub fn is_connection_error(&self) -> bool {
        matches!(
            self,
            DatabaseError::ConnectionFailed(_) | DatabaseError::PoolExhausted
        )
    }
}

/// Converts SQLx errors to more specific DatabaseError variants
///
/// This function analyzes the SQLx error and maps it to the appropriate
/// DatabaseError variant based on the PostgreSQL error code.
impl From<&sqlx::Error> for DatabaseError {
    fn from(error: &sqlx::Error) -> Self {
        match error {
            sqlx::Error::RowNotFound => {
                DatabaseError::NotFound("Record not found".to_string())
            }
            sqlx::Error::PoolTimedOut => DatabaseError::PoolExhausted,
            sqlx::Error::Database(db_err) => {
                // PostgreSQL error codes
                // https://www.postgresql.org/docs/current/errcodes-appendix.html
                if let Some(code) = db_err.code() {
                    match code.as_ref() {
                        "23505" => DatabaseError::DuplicateEntry(db_err.message().to_string()),
                        "23503" => {
                            DatabaseError::ForeignKeyViolation(db_err.message().to_string())
                        }
                        "23514" => {
                            DatabaseError::ConstraintViolation(db_err.message().to_string())
                        }
                        "23P01" => DatabaseError::TemporalOverlap(db_err.message().to_string()),
                        _ => DatabaseError::QueryFailed(db_err.message().to_string()),
                    }
                } else {
                    DatabaseError::QueryFailed(db_err.message().to_string())
                }
            }
            _ => DatabaseError::QueryFailed(error.to_string()),
        }
    }
}
