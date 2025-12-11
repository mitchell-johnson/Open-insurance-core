//! Policy repository implementation
//!
//! This module provides database access for policy aggregates,
//! implementing bi-temporal versioning for policy changes.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::bitemporal::{BiTemporalQuery, BiTemporalRepository, TimestampRange};
use crate::error::DatabaseError;

/// Repository for managing policy data with bi-temporal support
///
/// The PolicyRepository handles all database operations for policies,
/// including creation, updates, and historical queries. All changes
/// are tracked bi-temporally to support audit and regulatory requirements.
///
/// # Example
///
/// ```rust,ignore
/// use infra_db::repositories::PolicyRepository;
///
/// let repo = PolicyRepository::new(pool);
/// let policy = repo.get_current(&policy_id).await?;
/// ```
#[derive(Debug, Clone)]
pub struct PolicyRepository {
    pool: PgPool,
}

impl PolicyRepository {
    /// Creates a new PolicyRepository with the given connection pool
    ///
    /// # Arguments
    ///
    /// * `pool` - The PostgreSQL connection pool
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Retrieves all policies for a given party (policyholder)
    ///
    /// # Arguments
    ///
    /// * `party_id` - The party identifier
    ///
    /// # Returns
    ///
    /// A vector of policy records for the party
    pub async fn find_by_party(&self, party_id: Uuid) -> Result<Vec<PolicyRow>, DatabaseError> {
        let policies = sqlx::query_as!(
            PolicyRow,
            r#"
            SELECT
                version_id,
                policy_id,
                policy_number,
                product_code,
                policyholder_id,
                status as "status: PolicyStatus",
                effective_date,
                expiry_date,
                premium,
                sum_assured,
                currency,
                created_at,
                updated_at
            FROM policy_versions
            WHERE policyholder_id = $1
              AND upper(sys_period) IS NULL
            ORDER BY effective_date DESC
            "#,
            party_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(policies)
    }

    /// Retrieves policies expiring within a date range
    ///
    /// Useful for renewal processing and notifications.
    ///
    /// # Arguments
    ///
    /// * `from` - Start of the expiry range
    /// * `to` - End of the expiry range
    pub async fn find_expiring_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<PolicyRow>, DatabaseError> {
        let policies = sqlx::query_as!(
            PolicyRow,
            r#"
            SELECT
                version_id,
                policy_id,
                policy_number,
                product_code,
                policyholder_id,
                status as "status: PolicyStatus",
                effective_date,
                expiry_date,
                premium,
                sum_assured,
                currency,
                created_at,
                updated_at
            FROM policy_versions
            WHERE expiry_date BETWEEN $1 AND $2
              AND upper(sys_period) IS NULL
              AND status = 'in_force'
            ORDER BY expiry_date ASC
            "#,
            from,
            to
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(policies)
    }

    /// Creates a new policy with initial version
    ///
    /// # Arguments
    ///
    /// * `policy` - The policy data to insert
    ///
    /// # Returns
    ///
    /// The created policy row with generated identifiers
    pub async fn insert(&self, policy: NewPolicy) -> Result<PolicyRow, DatabaseError> {
        let version_id = Uuid::new_v4();
        let policy_id = Uuid::new_v4();
        let now = Utc::now();

        let row = sqlx::query_as!(
            PolicyRow,
            r#"
            INSERT INTO policy_versions (
                version_id,
                policy_id,
                policy_number,
                product_code,
                policyholder_id,
                status,
                effective_date,
                expiry_date,
                premium,
                sum_assured,
                currency,
                valid_period,
                sys_period,
                created_at,
                updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
                tstzrange($7, $8),
                tstzrange($12, NULL),
                $12, $12
            )
            RETURNING
                version_id,
                policy_id,
                policy_number,
                product_code,
                policyholder_id,
                status as "status: PolicyStatus",
                effective_date,
                expiry_date,
                premium,
                sum_assured,
                currency,
                created_at,
                updated_at
            "#,
            version_id,
            policy_id,
            policy.policy_number,
            policy.product_code,
            policy.policyholder_id,
            policy.status as PolicyStatus,
            policy.effective_date,
            policy.expiry_date,
            policy.premium,
            policy.sum_assured,
            policy.currency,
            now
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    /// Updates a policy by creating a new version
    ///
    /// This supersedes the current version and creates a new one,
    /// preserving the full audit trail.
    ///
    /// # Arguments
    ///
    /// * `policy_id` - The policy to update
    /// * `update` - The update data
    /// * `effective_from` - When the update becomes effective
    pub async fn update(
        &self,
        policy_id: Uuid,
        update: PolicyUpdate,
        effective_from: DateTime<Utc>,
    ) -> Result<PolicyRow, DatabaseError> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now();

        // Close the current version's sys_period
        sqlx::query!(
            r#"
            UPDATE policy_versions
            SET sys_period = tstzrange(lower(sys_period), $1)
            WHERE policy_id = $2 AND upper(sys_period) IS NULL
            "#,
            now,
            policy_id
        )
        .execute(&mut *tx)
        .await?;

        // Get the current version data for fields not being updated
        let current = sqlx::query_as!(
            PolicyRow,
            r#"
            SELECT
                version_id,
                policy_id,
                policy_number,
                product_code,
                policyholder_id,
                status as "status: PolicyStatus",
                effective_date,
                expiry_date,
                premium,
                sum_assured,
                currency,
                created_at,
                updated_at
            FROM policy_versions
            WHERE policy_id = $1
            ORDER BY lower(sys_period) DESC
            LIMIT 1
            "#,
            policy_id
        )
        .fetch_one(&mut *tx)
        .await?;

        // Create the new version
        let new_version_id = Uuid::new_v4();
        let new_row = sqlx::query_as!(
            PolicyRow,
            r#"
            INSERT INTO policy_versions (
                version_id,
                policy_id,
                policy_number,
                product_code,
                policyholder_id,
                status,
                effective_date,
                expiry_date,
                premium,
                sum_assured,
                currency,
                valid_period,
                sys_period,
                created_at,
                updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
                tstzrange($7, $8),
                tstzrange($12, NULL),
                $13, $12
            )
            RETURNING
                version_id,
                policy_id,
                policy_number,
                product_code,
                policyholder_id,
                status as "status: PolicyStatus",
                effective_date,
                expiry_date,
                premium,
                sum_assured,
                currency,
                created_at,
                updated_at
            "#,
            new_version_id,
            policy_id,
            current.policy_number,
            current.product_code,
            current.policyholder_id,
            update.status.unwrap_or(current.status) as PolicyStatus,
            effective_from,
            update.expiry_date.unwrap_or(current.expiry_date),
            update.premium.unwrap_or(current.premium),
            update.sum_assured.unwrap_or(current.sum_assured),
            current.currency,
            now,
            current.created_at
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(new_row)
    }
}

/// Policy status enumeration
///
/// Represents the lifecycle states of an insurance policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "policy_status", rename_all = "snake_case")]
pub enum PolicyStatus {
    /// Policy has been quoted but not yet bound
    Quoted,
    /// Policy is active and in force
    InForce,
    /// Policy has lapsed due to non-payment
    Lapsed,
    /// Policy has been terminated
    Terminated,
    /// Policy has been cancelled
    Cancelled,
    /// Policy has expired naturally
    Expired,
    /// Policy is pending underwriting
    PendingUnderwriting,
}

/// Database row representation of a policy version
#[derive(Debug, Clone)]
pub struct PolicyRow {
    pub version_id: Uuid,
    pub policy_id: Uuid,
    pub policy_number: String,
    pub product_code: String,
    pub policyholder_id: Uuid,
    pub status: PolicyStatus,
    pub effective_date: DateTime<Utc>,
    pub expiry_date: DateTime<Utc>,
    pub premium: rust_decimal::Decimal,
    pub sum_assured: rust_decimal::Decimal,
    pub currency: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Data for creating a new policy
#[derive(Debug, Clone)]
pub struct NewPolicy {
    pub policy_number: String,
    pub product_code: String,
    pub policyholder_id: Uuid,
    pub status: PolicyStatus,
    pub effective_date: DateTime<Utc>,
    pub expiry_date: DateTime<Utc>,
    pub premium: rust_decimal::Decimal,
    pub sum_assured: rust_decimal::Decimal,
    pub currency: String,
}

/// Data for updating an existing policy
#[derive(Debug, Clone, Default)]
pub struct PolicyUpdate {
    pub status: Option<PolicyStatus>,
    pub expiry_date: Option<DateTime<Utc>>,
    pub premium: Option<rust_decimal::Decimal>,
    pub sum_assured: Option<rust_decimal::Decimal>,
}
