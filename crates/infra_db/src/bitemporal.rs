//! Bi-temporal data access patterns
//!
//! This module provides abstractions for working with bi-temporal data in PostgreSQL.
//! Bi-temporal data tracks two time dimensions:
//!
//! - **Valid Time** (`valid_period`): When a fact is true in the real world
//! - **System Time** (`sys_period`): When the fact was recorded in the database
//!
//! # PostgreSQL Implementation
//!
//! The bi-temporal pattern is implemented using PostgreSQL's native `tstzrange` type
//! with exclusion constraints to prevent overlapping valid periods.
//!
//! ```sql
//! CREATE TABLE policy_versions (
//!     version_id UUID PRIMARY KEY,
//!     policy_id UUID NOT NULL,
//!     -- Business data fields...
//!     valid_period tstzrange NOT NULL,
//!     sys_period tstzrange NOT NULL DEFAULT tstzrange(CURRENT_TIMESTAMP, NULL),
//!     EXCLUDE USING gist (policy_id WITH =, valid_period WITH &&)
//!         WHERE (upper(sys_period) IS NULL)
//! );
//! ```
//!
//! # Query Patterns
//!
//! - **Current view**: `WHERE upper(sys_period) IS NULL`
//! - **As-of valid time**: `WHERE valid_period @> $timestamp`
//! - **As-of system time**: `WHERE sys_period @> $timestamp`
//! - **Bi-temporal**: Both conditions combined

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::DatabaseError;

/// Represents a PostgreSQL timestamp range for bi-temporal queries
///
/// This struct maps to PostgreSQL's `tstzrange` type and supports
/// both bounded and unbounded ranges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimestampRange {
    /// Lower bound of the range (inclusive)
    pub lower: DateTime<Utc>,
    /// Upper bound of the range (exclusive), None for unbounded
    pub upper: Option<DateTime<Utc>>,
}

impl TimestampRange {
    /// Creates a new timestamp range
    ///
    /// # Arguments
    ///
    /// * `lower` - Start of the range (inclusive)
    /// * `upper` - End of the range (exclusive), None for unbounded
    pub fn new(lower: DateTime<Utc>, upper: Option<DateTime<Utc>>) -> Self {
        Self { lower, upper }
    }

    /// Creates an unbounded range starting from the given timestamp
    ///
    /// # Arguments
    ///
    /// * `start` - Start of the range
    pub fn from(start: DateTime<Utc>) -> Self {
        Self {
            lower: start,
            upper: None,
        }
    }

    /// Creates a current (open-ended) range starting now
    pub fn current() -> Self {
        Self::from(Utc::now())
    }

    /// Checks if this range contains a specific timestamp
    ///
    /// # Arguments
    ///
    /// * `timestamp` - The timestamp to check
    pub fn contains(&self, timestamp: DateTime<Utc>) -> bool {
        timestamp >= self.lower && self.upper.map_or(true, |u| timestamp < u)
    }

    /// Checks if this range is currently open (no upper bound)
    pub fn is_current(&self) -> bool {
        self.upper.is_none()
    }

    /// Closes this range at the specified timestamp
    ///
    /// # Arguments
    ///
    /// * `end` - The timestamp to close the range at
    pub fn close(&mut self, end: DateTime<Utc>) {
        self.upper = Some(end);
    }
}

/// Query parameters for bi-temporal data retrieval
///
/// This struct encapsulates the different ways to query bi-temporal data,
/// supporting current views, historical views, and point-in-time queries.
#[derive(Debug, Clone, Default)]
pub struct BiTemporalQuery {
    /// Valid time point for "as of" queries
    pub valid_at: Option<DateTime<Utc>>,
    /// System time point for "as known at" queries
    pub system_at: Option<DateTime<Utc>>,
    /// Whether to include superseded (historical) versions
    pub include_history: bool,
}

impl BiTemporalQuery {
    /// Creates a query for current data only
    ///
    /// This returns the most recent version of data that is currently valid.
    pub fn current() -> Self {
        Self {
            valid_at: Some(Utc::now()),
            system_at: None,
            include_history: false,
        }
    }

    /// Creates a query for data valid at a specific point in time
    ///
    /// # Arguments
    ///
    /// * `timestamp` - The valid time point to query
    ///
    /// # Example
    ///
    /// ```rust
    /// use infra_db::BiTemporalQuery;
    /// use chrono::{Utc, TimeZone};
    ///
    /// // What coverage was in effect on June 15th?
    /// let query = BiTemporalQuery::valid_at(
    ///     Utc.with_ymd_and_hms(2024, 6, 15, 0, 0, 0).unwrap()
    /// );
    /// ```
    pub fn valid_at(timestamp: DateTime<Utc>) -> Self {
        Self {
            valid_at: Some(timestamp),
            system_at: None,
            include_history: false,
        }
    }

    /// Creates a query for data as known at a specific system time
    ///
    /// This enables "time travel" to see what data was recorded at a
    /// specific point in time, useful for auditing and debugging.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - The system time point to query
    ///
    /// # Example
    ///
    /// ```rust
    /// use infra_db::BiTemporalQuery;
    /// use chrono::{Utc, TimeZone};
    ///
    /// // What did we know about this policy last month?
    /// let query = BiTemporalQuery::system_at(
    ///     Utc.with_ymd_and_hms(2024, 5, 1, 0, 0, 0).unwrap()
    /// );
    /// ```
    pub fn system_at(timestamp: DateTime<Utc>) -> Self {
        Self {
            valid_at: None,
            system_at: Some(timestamp),
            include_history: false,
        }
    }

    /// Creates a full bi-temporal query
    ///
    /// This returns data that was valid at `valid_time` and known at `system_time`.
    /// Useful for regulatory reporting and historical reconstruction.
    ///
    /// # Arguments
    ///
    /// * `valid_time` - The business-effective time point
    /// * `system_time` - The system-recorded time point
    ///
    /// # Example
    ///
    /// ```rust
    /// use infra_db::BiTemporalQuery;
    /// use chrono::{Utc, TimeZone};
    ///
    /// // What did we think the coverage was on Feb 1st, when viewed on Feb 5th?
    /// let query = BiTemporalQuery::bitemporal(
    ///     Utc.with_ymd_and_hms(2024, 2, 1, 0, 0, 0).unwrap(),
    ///     Utc.with_ymd_and_hms(2024, 2, 5, 0, 0, 0).unwrap(),
    /// );
    /// ```
    pub fn bitemporal(valid_time: DateTime<Utc>, system_time: DateTime<Utc>) -> Self {
        Self {
            valid_at: Some(valid_time),
            system_at: Some(system_time),
            include_history: false,
        }
    }

    /// Creates a query that includes all historical versions
    ///
    /// This returns all versions of the data, including superseded ones,
    /// useful for audit trails and compliance reporting.
    pub fn with_history() -> Self {
        Self {
            valid_at: None,
            system_at: None,
            include_history: true,
        }
    }

    /// Modifies the query to include historical versions
    pub fn include_history(mut self) -> Self {
        self.include_history = true;
        self
    }

    /// Generates the WHERE clause conditions for this query
    ///
    /// Returns a tuple of (SQL conditions, bind parameters)
    pub fn to_where_clause(&self) -> (String, Vec<DateTime<Utc>>) {
        let mut conditions = Vec::new();
        let mut params = Vec::new();

        if !self.include_history {
            conditions.push("upper(sys_period) IS NULL".to_string());
        }

        if let Some(valid_at) = self.valid_at {
            conditions.push(format!("valid_period @> ${}", params.len() + 1));
            params.push(valid_at);
        }

        if let Some(system_at) = self.system_at {
            conditions.push(format!("sys_period @> ${}", params.len() + 1));
            params.push(system_at);
        }

        let clause = if conditions.is_empty() {
            "1=1".to_string()
        } else {
            conditions.join(" AND ")
        };

        (clause, params)
    }
}

/// Trait for bi-temporal repository operations
///
/// This trait defines the standard operations for managing bi-temporal entities.
/// Implementations handle the complexity of temporal versioning, ensuring that
/// historical data is preserved while new versions are created.
///
/// # Type Parameters
///
/// * `Entity` - The domain entity type
/// * `Id` - The entity identifier type
#[async_trait]
pub trait BiTemporalRepository<Entity, Id>
where
    Entity: Send + Sync,
    Id: Send + Sync,
{
    /// Retrieves the current version of an entity
    ///
    /// # Arguments
    ///
    /// * `id` - The entity identifier
    ///
    /// # Returns
    ///
    /// The current version of the entity, or `NotFound` error
    async fn get_current(&self, id: &Id) -> Result<Entity, DatabaseError>;

    /// Retrieves an entity using bi-temporal query parameters
    ///
    /// # Arguments
    ///
    /// * `id` - The entity identifier
    /// * `query` - The bi-temporal query parameters
    ///
    /// # Returns
    ///
    /// The entity matching the query criteria, or `NotFound` error
    async fn get_at(&self, id: &Id, query: &BiTemporalQuery) -> Result<Entity, DatabaseError>;

    /// Retrieves the full history of an entity
    ///
    /// # Arguments
    ///
    /// * `id` - The entity identifier
    ///
    /// # Returns
    ///
    /// A vector of all historical versions, ordered by system time
    async fn get_history(&self, id: &Id) -> Result<Vec<Entity>, DatabaseError>;

    /// Creates a new entity with an initial version
    ///
    /// # Arguments
    ///
    /// * `entity` - The entity to create
    /// * `valid_from` - When the entity becomes valid
    ///
    /// # Returns
    ///
    /// The created entity with assigned identifiers
    async fn create(&self, entity: Entity, valid_from: DateTime<Utc>)
        -> Result<Entity, DatabaseError>;

    /// Updates an entity by creating a new version
    ///
    /// This supersedes the current version and creates a new one,
    /// preserving the historical record.
    ///
    /// # Arguments
    ///
    /// * `id` - The entity identifier
    /// * `entity` - The updated entity data
    /// * `valid_from` - When the update becomes valid
    ///
    /// # Returns
    ///
    /// The new version of the entity
    async fn update(
        &self,
        id: &Id,
        entity: Entity,
        valid_from: DateTime<Utc>,
    ) -> Result<Entity, DatabaseError>;

    /// Performs a retroactive correction
    ///
    /// This creates a correction that changes historical data. The old
    /// version is superseded (sys_period closed) and a new version is
    /// created with the original valid_period but new data.
    ///
    /// # Arguments
    ///
    /// * `id` - The entity identifier
    /// * `entity` - The corrected entity data
    /// * `valid_period` - The period to which this correction applies
    /// * `reason` - The reason for the correction (for audit)
    async fn correct(
        &self,
        id: &Id,
        entity: Entity,
        valid_period: TimestampRange,
        reason: &str,
    ) -> Result<Entity, DatabaseError>;
}

/// Metadata for bi-temporal records
///
/// This struct captures the temporal metadata associated with each
/// record version, used for audit trails and temporal queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiTemporalMetadata {
    /// Unique identifier for this version
    pub version_id: Uuid,
    /// When this version was recorded
    pub recorded_at: DateTime<Utc>,
    /// When this version was superseded (None if current)
    pub superseded_at: Option<DateTime<Utc>>,
    /// Start of valid period
    pub valid_from: DateTime<Utc>,
    /// End of valid period (None if unbounded)
    pub valid_to: Option<DateTime<Utc>>,
    /// User who made this change
    pub changed_by: Option<String>,
    /// Reason for the change (for corrections)
    pub change_reason: Option<String>,
}

impl BiTemporalMetadata {
    /// Creates metadata for a new record
    ///
    /// # Arguments
    ///
    /// * `valid_from` - When the record becomes valid
    /// * `valid_to` - When the record expires (None for indefinite)
    /// * `changed_by` - User making the change
    pub fn new(
        valid_from: DateTime<Utc>,
        valid_to: Option<DateTime<Utc>>,
        changed_by: Option<String>,
    ) -> Self {
        Self {
            version_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            superseded_at: None,
            valid_from,
            valid_to,
            changed_by,
            change_reason: None,
        }
    }

    /// Checks if this record is the current (non-superseded) version
    pub fn is_current(&self) -> bool {
        self.superseded_at.is_none()
    }

    /// Checks if this record is valid at the given timestamp
    pub fn is_valid_at(&self, timestamp: DateTime<Utc>) -> bool {
        timestamp >= self.valid_from && self.valid_to.map_or(true, |t| timestamp < t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_timestamp_range_contains() {
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2024, 12, 31, 23, 59, 59).unwrap();
        let range = TimestampRange::new(start, Some(end));

        let mid = Utc.with_ymd_and_hms(2024, 6, 15, 12, 0, 0).unwrap();
        assert!(range.contains(mid));

        let after = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        assert!(!range.contains(after));
    }

    #[test]
    fn test_bitemporal_query_where_clause() {
        let query = BiTemporalQuery::current();
        let (clause, _) = query.to_where_clause();
        assert!(clause.contains("upper(sys_period) IS NULL"));

        let ts = Utc.with_ymd_and_hms(2024, 6, 15, 0, 0, 0).unwrap();
        let query = BiTemporalQuery::valid_at(ts);
        let (clause, params) = query.to_where_clause();
        assert!(clause.contains("valid_period @>"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_metadata_is_current() {
        let metadata = BiTemporalMetadata::new(Utc::now(), None, None);
        assert!(metadata.is_current());
    }
}
