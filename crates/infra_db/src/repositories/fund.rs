//! Fund repository implementation
//!
//! This module provides database access for Unit-Linked Insurance Plan (ULIP)
//! fund management, including NAV tracking and unit allocations.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::DatabaseError;

/// Repository for managing investment fund data
///
/// The FundRepository handles all database operations for ULIP funds,
/// including NAV calculations, unit registries, and fund switches.
#[derive(Debug, Clone)]
pub struct FundRepository {
    pool: PgPool,
}

impl FundRepository {
    /// Creates a new FundRepository with the given connection pool
    ///
    /// # Arguments
    ///
    /// * `pool` - The PostgreSQL connection pool
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Retrieves the latest NAV for a fund
    ///
    /// # Arguments
    ///
    /// * `fund_id` - The fund identifier
    ///
    /// # Returns
    ///
    /// The most recent NAV record for the fund
    pub async fn get_latest_nav(&self, fund_id: Uuid) -> Result<NavRow, DatabaseError> {
        let nav = sqlx::query_as!(
            NavRow,
            r#"
            SELECT
                nav_id,
                fund_id,
                nav_date,
                nav_value,
                currency,
                created_at
            FROM fund_navs
            WHERE fund_id = $1
            ORDER BY nav_date DESC
            LIMIT 1
            "#,
            fund_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| DatabaseError::not_found("NAV for fund", fund_id))?;

        Ok(nav)
    }

    /// Retrieves NAV history for a fund within a date range
    ///
    /// # Arguments
    ///
    /// * `fund_id` - The fund identifier
    /// * `from` - Start date (inclusive)
    /// * `to` - End date (inclusive)
    pub async fn get_nav_history(
        &self,
        fund_id: Uuid,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<NavRow>, DatabaseError> {
        let navs = sqlx::query_as!(
            NavRow,
            r#"
            SELECT
                nav_id,
                fund_id,
                nav_date,
                nav_value,
                currency,
                created_at
            FROM fund_navs
            WHERE fund_id = $1 AND nav_date BETWEEN $2 AND $3
            ORDER BY nav_date ASC
            "#,
            fund_id,
            from,
            to
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(navs)
    }

    /// Records a new NAV for a fund
    ///
    /// # Arguments
    ///
    /// * `fund_id` - The fund identifier
    /// * `nav_date` - The date of the NAV
    /// * `nav_value` - The NAV value
    /// * `currency` - The currency code
    pub async fn insert_nav(
        &self,
        fund_id: Uuid,
        nav_date: NaiveDate,
        nav_value: Decimal,
        currency: &str,
    ) -> Result<NavRow, DatabaseError> {
        let nav_id = Uuid::new_v4();
        let now = Utc::now();

        let nav = sqlx::query_as!(
            NavRow,
            r#"
            INSERT INTO fund_navs (nav_id, fund_id, nav_date, nav_value, currency, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING nav_id, fund_id, nav_date, nav_value, currency, created_at
            "#,
            nav_id,
            fund_id,
            nav_date,
            nav_value,
            currency,
            now
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(nav)
    }

    /// Retrieves unit holdings for a policy
    ///
    /// # Arguments
    ///
    /// * `policy_id` - The policy identifier
    pub async fn get_unit_holdings(&self, policy_id: Uuid) -> Result<Vec<UnitHoldingRow>, DatabaseError> {
        let holdings = sqlx::query_as!(
            UnitHoldingRow,
            r#"
            SELECT
                holding_id,
                policy_id,
                fund_id,
                units,
                created_at,
                updated_at
            FROM unit_holdings
            WHERE policy_id = $1
            ORDER BY fund_id
            "#,
            policy_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(holdings)
    }

    /// Allocates units to a policy's fund holding
    ///
    /// # Arguments
    ///
    /// * `policy_id` - The policy identifier
    /// * `fund_id` - The fund identifier
    /// * `units` - The number of units to allocate (positive to add, negative to redeem)
    /// * `transaction_type` - The type of transaction
    ///
    /// # Returns
    ///
    /// The updated unit holding record
    pub async fn allocate_units(
        &self,
        policy_id: Uuid,
        fund_id: Uuid,
        units: Decimal,
        transaction_type: UnitTransactionType,
    ) -> Result<UnitHoldingRow, DatabaseError> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now();
        let transaction_id = Uuid::new_v4();

        // Record the transaction
        sqlx::query!(
            r#"
            INSERT INTO unit_transactions (
                transaction_id, policy_id, fund_id, units,
                transaction_type, transaction_date, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            transaction_id,
            policy_id,
            fund_id,
            units,
            transaction_type as UnitTransactionType,
            now,
            now
        )
        .execute(&mut *tx)
        .await?;

        // Upsert the holding
        let holding = sqlx::query_as!(
            UnitHoldingRow,
            r#"
            INSERT INTO unit_holdings (holding_id, policy_id, fund_id, units, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $5)
            ON CONFLICT (policy_id, fund_id) DO UPDATE
            SET units = unit_holdings.units + $4, updated_at = $5
            RETURNING holding_id, policy_id, fund_id, units, created_at, updated_at
            "#,
            Uuid::new_v4(),
            policy_id,
            fund_id,
            units,
            now
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(holding)
    }

    /// Retrieves all available funds
    pub async fn list_funds(&self) -> Result<Vec<FundRow>, DatabaseError> {
        let funds = sqlx::query_as!(
            FundRow,
            r#"
            SELECT
                fund_id,
                fund_code,
                fund_name,
                fund_type as "fund_type: FundType",
                currency,
                risk_level as "risk_level: RiskLevel",
                management_fee,
                is_active,
                created_at
            FROM funds
            WHERE is_active = true
            ORDER BY fund_name
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(funds)
    }
}

/// Fund type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "fund_type", rename_all = "snake_case")]
pub enum FundType {
    /// Equity/stock fund
    Equity,
    /// Fixed income/bond fund
    Bond,
    /// Balanced (mixed) fund
    Balanced,
    /// Money market fund
    MoneyMarket,
    /// Index fund
    Index,
    /// Sector-specific fund
    Sector,
}

/// Risk level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "risk_level", rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    VeryHigh,
}

/// Unit transaction type
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "unit_transaction_type", rename_all = "snake_case")]
pub enum UnitTransactionType {
    /// Premium allocation
    Allocation,
    /// Redemption/withdrawal
    Redemption,
    /// Fund switch (in)
    SwitchIn,
    /// Fund switch (out)
    SwitchOut,
    /// Mortality charge deduction
    MortalityCharge,
    /// Policy fee deduction
    PolicyFee,
}

/// Database row for fund
#[derive(Debug, Clone)]
pub struct FundRow {
    pub fund_id: Uuid,
    pub fund_code: String,
    pub fund_name: String,
    pub fund_type: FundType,
    pub currency: String,
    pub risk_level: RiskLevel,
    pub management_fee: Decimal,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Database row for NAV record
#[derive(Debug, Clone)]
pub struct NavRow {
    pub nav_id: Uuid,
    pub fund_id: Uuid,
    pub nav_date: NaiveDate,
    pub nav_value: Decimal,
    pub currency: String,
    pub created_at: DateTime<Utc>,
}

/// Database row for unit holding
#[derive(Debug, Clone)]
pub struct UnitHoldingRow {
    pub holding_id: Uuid,
    pub policy_id: Uuid,
    pub fund_id: Uuid,
    pub units: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
