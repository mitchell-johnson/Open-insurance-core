//! Claims repository implementation
//!
//! This module provides database access for claims management,
//! including FNOL, adjudication, reserving, and payments.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::DatabaseError;

/// Repository for managing claims data
///
/// The ClaimsRepository handles all database operations for the claims
/// lifecycle, from First Notice of Loss (FNOL) through settlement.
#[derive(Debug, Clone)]
pub struct ClaimsRepository {
    pool: PgPool,
}

impl ClaimsRepository {
    /// Creates a new ClaimsRepository with the given connection pool
    ///
    /// # Arguments
    ///
    /// * `pool` - The PostgreSQL connection pool
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Retrieves a claim by its identifier
    ///
    /// # Arguments
    ///
    /// * `claim_id` - The claim identifier
    ///
    /// # Returns
    ///
    /// The claim record or NotFound error
    pub async fn get_by_id(&self, claim_id: Uuid) -> Result<ClaimRow, DatabaseError> {
        let claim = sqlx::query_as!(
            ClaimRow,
            r#"
            SELECT
                claim_id,
                claim_number,
                policy_id,
                claimant_id,
                status as "status: ClaimStatus",
                loss_date,
                notification_date,
                loss_type as "loss_type: LossType",
                loss_description,
                loss_location,
                claimed_amount,
                approved_amount,
                paid_amount,
                currency,
                assigned_to,
                created_at,
                updated_at
            FROM claims
            WHERE claim_id = $1
            "#,
            claim_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| DatabaseError::not_found("Claim", claim_id))?;

        Ok(claim)
    }

    /// Retrieves all claims for a policy
    ///
    /// # Arguments
    ///
    /// * `policy_id` - The policy identifier
    pub async fn find_by_policy(&self, policy_id: Uuid) -> Result<Vec<ClaimRow>, DatabaseError> {
        let claims = sqlx::query_as!(
            ClaimRow,
            r#"
            SELECT
                claim_id,
                claim_number,
                policy_id,
                claimant_id,
                status as "status: ClaimStatus",
                loss_date,
                notification_date,
                loss_type as "loss_type: LossType",
                loss_description,
                loss_location,
                claimed_amount,
                approved_amount,
                paid_amount,
                currency,
                assigned_to,
                created_at,
                updated_at
            FROM claims
            WHERE policy_id = $1
            ORDER BY notification_date DESC
            "#,
            policy_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(claims)
    }

    /// Creates a new claim (FNOL - First Notice of Loss)
    ///
    /// # Arguments
    ///
    /// * `claim` - The claim data to insert
    ///
    /// # Returns
    ///
    /// The created claim with generated identifiers
    pub async fn create_fnol(&self, claim: NewClaim) -> Result<ClaimRow, DatabaseError> {
        let claim_id = Uuid::new_v4();
        let now = Utc::now();

        // Generate claim number (format: CLM-YYYY-NNNNNNN)
        let claim_number = format!("CLM-{}-{:07}", now.format("%Y"), claim_id.as_fields().0 % 10_000_000);

        let row = sqlx::query_as!(
            ClaimRow,
            r#"
            INSERT INTO claims (
                claim_id, claim_number, policy_id, claimant_id,
                status, loss_date, notification_date, loss_type,
                loss_description, loss_location, claimed_amount,
                currency, assigned_to, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $14
            )
            RETURNING
                claim_id, claim_number, policy_id, claimant_id,
                status as "status: ClaimStatus", loss_date, notification_date,
                loss_type as "loss_type: LossType", loss_description, loss_location,
                claimed_amount, approved_amount, paid_amount, currency,
                assigned_to, created_at, updated_at
            "#,
            claim_id,
            claim_number,
            claim.policy_id,
            claim.claimant_id,
            ClaimStatus::Fnol as ClaimStatus,
            claim.loss_date,
            claim.notification_date,
            claim.loss_type as LossType,
            claim.loss_description,
            claim.loss_location,
            claim.claimed_amount,
            claim.currency,
            claim.assigned_to,
            now
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    /// Updates a claim's status
    ///
    /// # Arguments
    ///
    /// * `claim_id` - The claim identifier
    /// * `status` - The new status
    /// * `reason` - Optional reason for the status change
    pub async fn update_status(
        &self,
        claim_id: Uuid,
        status: ClaimStatus,
        reason: Option<&str>,
    ) -> Result<ClaimRow, DatabaseError> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now();

        // Record status history
        sqlx::query!(
            r#"
            INSERT INTO claim_status_history (
                history_id, claim_id, status, reason, changed_at
            ) VALUES ($1, $2, $3, $4, $5)
            "#,
            Uuid::new_v4(),
            claim_id,
            status as ClaimStatus,
            reason,
            now
        )
        .execute(&mut *tx)
        .await?;

        // Update claim
        let claim = sqlx::query_as!(
            ClaimRow,
            r#"
            UPDATE claims
            SET status = $2, updated_at = $3
            WHERE claim_id = $1
            RETURNING
                claim_id, claim_number, policy_id, claimant_id,
                status as "status: ClaimStatus", loss_date, notification_date,
                loss_type as "loss_type: LossType", loss_description, loss_location,
                claimed_amount, approved_amount, paid_amount, currency,
                assigned_to, created_at, updated_at
            "#,
            claim_id,
            status as ClaimStatus,
            now
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(claim)
    }

    /// Records a reserve against a claim
    ///
    /// # Arguments
    ///
    /// * `claim_id` - The claim identifier
    /// * `reserve` - The reserve data
    pub async fn add_reserve(&self, claim_id: Uuid, reserve: NewReserve) -> Result<ReserveRow, DatabaseError> {
        let reserve_id = Uuid::new_v4();
        let now = Utc::now();

        let row = sqlx::query_as!(
            ReserveRow,
            r#"
            INSERT INTO claim_reserves (
                reserve_id, claim_id, reserve_type, amount, currency,
                reason, created_by, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING
                reserve_id, claim_id, reserve_type as "reserve_type: ReserveType",
                amount, currency, reason, created_by, created_at
            "#,
            reserve_id,
            claim_id,
            reserve.reserve_type as ReserveType,
            reserve.amount,
            reserve.currency,
            reserve.reason,
            reserve.created_by,
            now
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    /// Records a payment for a claim
    ///
    /// # Arguments
    ///
    /// * `claim_id` - The claim identifier
    /// * `payment` - The payment data
    pub async fn record_payment(
        &self,
        claim_id: Uuid,
        payment: NewPayment,
    ) -> Result<PaymentRow, DatabaseError> {
        let mut tx = self.pool.begin().await?;
        let payment_id = Uuid::new_v4();
        let now = Utc::now();

        // Create payment record
        let row = sqlx::query_as!(
            PaymentRow,
            r#"
            INSERT INTO claim_payments (
                payment_id, claim_id, payee_id, amount, currency,
                payment_type, payment_method, reference, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING
                payment_id, claim_id, payee_id, amount, currency,
                payment_type as "payment_type: PaymentType",
                payment_method as "payment_method: PaymentMethod",
                reference, created_at
            "#,
            payment_id,
            claim_id,
            payment.payee_id,
            payment.amount,
            payment.currency,
            payment.payment_type as PaymentType,
            payment.payment_method as PaymentMethod,
            payment.reference,
            now
        )
        .fetch_one(&mut *tx)
        .await?;

        // Update claim paid amount
        sqlx::query!(
            r#"
            UPDATE claims
            SET paid_amount = COALESCE(paid_amount, 0) + $2, updated_at = $3
            WHERE claim_id = $1
            "#,
            claim_id,
            payment.amount,
            now
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(row)
    }
}

/// Claim status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "claim_status", rename_all = "snake_case")]
pub enum ClaimStatus {
    /// First Notice of Loss received
    Fnol,
    /// Under investigation
    UnderInvestigation,
    /// Pending documentation
    PendingDocumentation,
    /// Under review/adjudication
    UnderReview,
    /// Approved for payment
    Approved,
    /// Partially approved
    PartiallyApproved,
    /// Denied
    Denied,
    /// Paid and closed
    Closed,
    /// Withdrawn by claimant
    Withdrawn,
    /// Reopened
    Reopened,
}

/// Loss type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "loss_type", rename_all = "snake_case")]
pub enum LossType {
    Death,
    Disability,
    CriticalIllness,
    Hospitalization,
    Accident,
    Property,
    Liability,
    Other,
}

/// Reserve type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "reserve_type", rename_all = "snake_case")]
pub enum ReserveType {
    /// Initial case reserve
    CaseReserve,
    /// Incurred But Not Reported
    Ibnr,
    /// Legal expenses
    LegalExpense,
    /// Additional expenses
    Expense,
}

/// Payment type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "payment_type", rename_all = "snake_case")]
pub enum PaymentType {
    /// Indemnity payment
    Indemnity,
    /// Expense payment
    Expense,
    /// Partial payment
    Partial,
    /// Final settlement
    FinalSettlement,
}

/// Payment method enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "payment_method", rename_all = "snake_case")]
pub enum PaymentMethod {
    BankTransfer,
    Check,
    DirectDeposit,
    Wire,
}

/// Database row for claim
#[derive(Debug, Clone)]
pub struct ClaimRow {
    pub claim_id: Uuid,
    pub claim_number: String,
    pub policy_id: Uuid,
    pub claimant_id: Uuid,
    pub status: ClaimStatus,
    pub loss_date: NaiveDate,
    pub notification_date: DateTime<Utc>,
    pub loss_type: LossType,
    pub loss_description: Option<String>,
    pub loss_location: Option<String>,
    pub claimed_amount: Option<Decimal>,
    pub approved_amount: Option<Decimal>,
    pub paid_amount: Option<Decimal>,
    pub currency: String,
    pub assigned_to: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Data for creating a new claim
#[derive(Debug, Clone)]
pub struct NewClaim {
    pub policy_id: Uuid,
    pub claimant_id: Uuid,
    pub loss_date: NaiveDate,
    pub notification_date: DateTime<Utc>,
    pub loss_type: LossType,
    pub loss_description: Option<String>,
    pub loss_location: Option<String>,
    pub claimed_amount: Option<Decimal>,
    pub currency: String,
    pub assigned_to: Option<String>,
}

/// Database row for reserve
#[derive(Debug, Clone)]
pub struct ReserveRow {
    pub reserve_id: Uuid,
    pub claim_id: Uuid,
    pub reserve_type: ReserveType,
    pub amount: Decimal,
    pub currency: String,
    pub reason: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Data for creating a new reserve
#[derive(Debug, Clone)]
pub struct NewReserve {
    pub reserve_type: ReserveType,
    pub amount: Decimal,
    pub currency: String,
    pub reason: Option<String>,
    pub created_by: Option<String>,
}

/// Database row for payment
#[derive(Debug, Clone)]
pub struct PaymentRow {
    pub payment_id: Uuid,
    pub claim_id: Uuid,
    pub payee_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub payment_type: PaymentType,
    pub payment_method: PaymentMethod,
    pub reference: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Data for creating a new payment
#[derive(Debug, Clone)]
pub struct NewPayment {
    pub payee_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub payment_type: PaymentType,
    pub payment_method: PaymentMethod,
    pub reference: Option<String>,
}
