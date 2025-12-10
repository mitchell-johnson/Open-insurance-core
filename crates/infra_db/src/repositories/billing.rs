//! Billing repository implementation
//!
//! This module provides database access for the double-entry ledger system,
//! including journal entries, postings, and account management.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::DatabaseError;

/// Repository for managing the double-entry ledger
///
/// The BillingRepository handles all database operations for the
/// financial ledger, ensuring ACID compliance for all transactions.
#[derive(Debug, Clone)]
pub struct BillingRepository {
    pool: PgPool,
}

impl BillingRepository {
    /// Creates a new BillingRepository with the given connection pool
    ///
    /// # Arguments
    ///
    /// * `pool` - The PostgreSQL connection pool
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Creates a journal entry with its postings in a single transaction
    ///
    /// This method ensures atomicity: either all postings are created
    /// together with the journal entry, or none are.
    ///
    /// # Arguments
    ///
    /// * `entry` - The journal entry data
    /// * `postings` - The list of postings (must balance to zero)
    ///
    /// # Returns
    ///
    /// The created journal entry ID
    ///
    /// # Errors
    ///
    /// Returns error if postings don't balance to zero
    pub async fn create_journal_entry(
        &self,
        entry: NewJournalEntry,
        postings: Vec<NewPosting>,
    ) -> Result<Uuid, DatabaseError> {
        // Verify postings balance
        let balance: Decimal = postings.iter().map(|p| p.amount).sum();
        if !balance.is_zero() {
            return Err(DatabaseError::ConstraintViolation(format!(
                "Postings do not balance. Sum: {}",
                balance
            )));
        }

        let mut tx = self.pool.begin().await?;
        let entry_id = Uuid::new_v4();
        let now = Utc::now();

        // Create journal entry
        sqlx::query!(
            r#"
            INSERT INTO journal_entries (
                entry_id, entry_date, description, reference_type,
                reference_id, created_by, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            entry_id,
            entry.entry_date,
            entry.description,
            entry.reference_type,
            entry.reference_id,
            entry.created_by,
            now
        )
        .execute(&mut *tx)
        .await?;

        // Create postings
        for posting in postings {
            let posting_id = Uuid::new_v4();
            sqlx::query!(
                r#"
                INSERT INTO postings (
                    posting_id, entry_id, account_id, amount, currency,
                    posting_type, description, created_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                "#,
                posting_id,
                entry_id,
                posting.account_id,
                posting.amount,
                posting.currency,
                posting.posting_type as PostingType,
                posting.description,
                now
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(entry_id)
    }

    /// Retrieves the balance of an account
    ///
    /// # Arguments
    ///
    /// * `account_id` - The account identifier
    ///
    /// # Returns
    ///
    /// The current balance of the account
    pub async fn get_account_balance(&self, account_id: Uuid) -> Result<Decimal, DatabaseError> {
        let result = sqlx::query!(
            r#"
            SELECT COALESCE(SUM(amount), 0) as "balance!"
            FROM postings
            WHERE account_id = $1
            "#,
            account_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.balance)
    }

    /// Retrieves journal entries for a reference (e.g., policy, claim)
    ///
    /// # Arguments
    ///
    /// * `reference_type` - The type of reference (e.g., "policy", "claim")
    /// * `reference_id` - The reference identifier
    pub async fn find_entries_by_reference(
        &self,
        reference_type: &str,
        reference_id: Uuid,
    ) -> Result<Vec<JournalEntryRow>, DatabaseError> {
        let entries = sqlx::query_as!(
            JournalEntryRow,
            r#"
            SELECT
                entry_id,
                entry_date,
                description,
                reference_type,
                reference_id,
                created_by,
                created_at
            FROM journal_entries
            WHERE reference_type = $1 AND reference_id = $2
            ORDER BY entry_date DESC, created_at DESC
            "#,
            reference_type,
            reference_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(entries)
    }

    /// Retrieves all postings for a journal entry
    ///
    /// # Arguments
    ///
    /// * `entry_id` - The journal entry identifier
    pub async fn get_postings(&self, entry_id: Uuid) -> Result<Vec<PostingRow>, DatabaseError> {
        let postings = sqlx::query_as!(
            PostingRow,
            r#"
            SELECT
                posting_id,
                entry_id,
                account_id,
                amount,
                currency,
                posting_type as "posting_type: PostingType",
                description,
                created_at
            FROM postings
            WHERE entry_id = $1
            ORDER BY created_at
            "#,
            entry_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(postings)
    }
}

/// Posting type (debit or credit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "posting_type", rename_all = "snake_case")]
pub enum PostingType {
    /// Debit posting (positive amount)
    Debit,
    /// Credit posting (negative amount)
    Credit,
}

/// Account type for chart of accounts
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "account_type", rename_all = "snake_case")]
pub enum AccountType {
    /// Asset account
    Asset,
    /// Liability account
    Liability,
    /// Equity account
    Equity,
    /// Revenue account
    Revenue,
    /// Expense account
    Expense,
}

/// Database row for journal entry
#[derive(Debug, Clone)]
pub struct JournalEntryRow {
    pub entry_id: Uuid,
    pub entry_date: DateTime<Utc>,
    pub description: String,
    pub reference_type: Option<String>,
    pub reference_id: Option<Uuid>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Database row for posting
#[derive(Debug, Clone)]
pub struct PostingRow {
    pub posting_id: Uuid,
    pub entry_id: Uuid,
    pub account_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub posting_type: PostingType,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Data for creating a new journal entry
#[derive(Debug, Clone)]
pub struct NewJournalEntry {
    pub entry_date: DateTime<Utc>,
    pub description: String,
    pub reference_type: Option<String>,
    pub reference_id: Option<Uuid>,
    pub created_by: Option<String>,
}

/// Data for creating a new posting
#[derive(Debug, Clone)]
pub struct NewPosting {
    pub account_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub posting_type: PostingType,
    pub description: Option<String>,
}
