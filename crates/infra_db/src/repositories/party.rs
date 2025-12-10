//! Party repository implementation
//!
//! This module provides database access for party (customer, agent) data,
//! implementing bi-temporal versioning for party changes.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::DatabaseError;

/// Repository for managing party data with bi-temporal support
///
/// The PartyRepository handles all database operations for parties,
/// including customers, agents, and other stakeholders.
#[derive(Debug, Clone)]
pub struct PartyRepository {
    pool: PgPool,
}

impl PartyRepository {
    /// Creates a new PartyRepository with the given connection pool
    ///
    /// # Arguments
    ///
    /// * `pool` - The PostgreSQL connection pool
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Retrieves a party by their identifier
    ///
    /// # Arguments
    ///
    /// * `party_id` - The party identifier
    ///
    /// # Returns
    ///
    /// The current party record or NotFound error
    pub async fn get_by_id(&self, party_id: Uuid) -> Result<PartyRow, DatabaseError> {
        let party = sqlx::query_as!(
            PartyRow,
            r#"
            SELECT
                version_id,
                party_id,
                party_type as "party_type: PartyType",
                first_name,
                last_name,
                company_name,
                email,
                phone,
                date_of_birth,
                tax_id,
                kyc_status as "kyc_status: KycStatus",
                created_at,
                updated_at
            FROM party_versions
            WHERE party_id = $1 AND upper(sys_period) IS NULL
            "#,
            party_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| DatabaseError::not_found("Party", party_id))?;

        Ok(party)
    }

    /// Searches for parties by email address
    ///
    /// # Arguments
    ///
    /// * `email` - The email address to search for
    ///
    /// # Returns
    ///
    /// A vector of matching party records
    pub async fn find_by_email(&self, email: &str) -> Result<Vec<PartyRow>, DatabaseError> {
        let parties = sqlx::query_as!(
            PartyRow,
            r#"
            SELECT
                version_id,
                party_id,
                party_type as "party_type: PartyType",
                first_name,
                last_name,
                company_name,
                email,
                phone,
                date_of_birth,
                tax_id,
                kyc_status as "kyc_status: KycStatus",
                created_at,
                updated_at
            FROM party_versions
            WHERE lower(email) = lower($1) AND upper(sys_period) IS NULL
            "#,
            email
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(parties)
    }

    /// Creates a new party
    ///
    /// # Arguments
    ///
    /// * `party` - The party data to insert
    ///
    /// # Returns
    ///
    /// The created party row with generated identifiers
    pub async fn insert(&self, party: NewParty) -> Result<PartyRow, DatabaseError> {
        let version_id = Uuid::new_v4();
        let party_id = Uuid::new_v4();
        let now = Utc::now();

        let row = sqlx::query_as!(
            PartyRow,
            r#"
            INSERT INTO party_versions (
                version_id,
                party_id,
                party_type,
                first_name,
                last_name,
                company_name,
                email,
                phone,
                date_of_birth,
                tax_id,
                kyc_status,
                valid_period,
                sys_period,
                created_at,
                updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
                tstzrange($12, NULL),
                tstzrange($12, NULL),
                $12, $12
            )
            RETURNING
                version_id,
                party_id,
                party_type as "party_type: PartyType",
                first_name,
                last_name,
                company_name,
                email,
                phone,
                date_of_birth,
                tax_id,
                kyc_status as "kyc_status: KycStatus",
                created_at,
                updated_at
            "#,
            version_id,
            party_id,
            party.party_type as PartyType,
            party.first_name,
            party.last_name,
            party.company_name,
            party.email,
            party.phone,
            party.date_of_birth,
            party.tax_id,
            party.kyc_status as KycStatus,
            now
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }
}

/// Party type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "party_type", rename_all = "snake_case")]
pub enum PartyType {
    /// Individual person
    Individual,
    /// Corporate entity
    Corporate,
    /// Insurance agent
    Agent,
    /// Broker
    Broker,
    /// Beneficiary
    Beneficiary,
}

/// KYC (Know Your Customer) status
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "kyc_status", rename_all = "snake_case")]
pub enum KycStatus {
    /// KYC not yet performed
    Pending,
    /// KYC in progress
    InProgress,
    /// KYC verified successfully
    Verified,
    /// KYC failed verification
    Failed,
    /// KYC expired, needs renewal
    Expired,
}

/// Database row representation of a party version
#[derive(Debug, Clone)]
pub struct PartyRow {
    pub version_id: Uuid,
    pub party_id: Uuid,
    pub party_type: PartyType,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub company_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub date_of_birth: Option<chrono::NaiveDate>,
    pub tax_id: Option<String>,
    pub kyc_status: KycStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Data for creating a new party
#[derive(Debug, Clone)]
pub struct NewParty {
    pub party_type: PartyType,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub company_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub date_of_birth: Option<chrono::NaiveDate>,
    pub tax_id: Option<String>,
    pub kyc_status: KycStatus,
}
