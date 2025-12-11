//! Account types for the chart of accounts
//!
//! This module defines the account structure for double-entry bookkeeping.

use serde::{Deserialize, Serialize};

use core_kernel::AccountId;

/// Types of accounts in the chart of accounts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountType {
    /// Asset accounts (debit normal balance)
    Asset,
    /// Liability accounts (credit normal balance)
    Liability,
    /// Equity accounts (credit normal balance)
    Equity,
    /// Revenue accounts (credit normal balance)
    Revenue,
    /// Expense accounts (debit normal balance)
    Expense,
}

impl AccountType {
    /// Returns true if this account type has a debit normal balance
    pub fn is_debit_normal(&self) -> bool {
        matches!(self, AccountType::Asset | AccountType::Expense)
    }
}

/// Category of account for financial reporting
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountCategory {
    /// Cash and bank accounts
    Cash,
    /// Accounts receivable
    Receivables,
    /// Investment assets
    Investments,
    /// Fixed assets
    FixedAssets,
    /// Accounts payable
    Payables,
    /// Insurance reserves
    Reserves,
    /// Unearned premium
    UnearnedPremium,
    /// Premium income
    PremiumIncome,
    /// Investment income
    InvestmentIncome,
    /// Fee income
    FeeIncome,
    /// Claims expense
    ClaimsExpense,
    /// Commission expense
    CommissionExpense,
    /// Operating expense
    OperatingExpense,
    /// Other
    Other,
}

/// An account in the chart of accounts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// Unique identifier
    pub id: AccountId,
    /// Account code (e.g., "1000")
    pub code: String,
    /// Account name
    pub name: String,
    /// Account type
    pub account_type: AccountType,
    /// Account category
    pub category: Option<AccountCategory>,
    /// Parent account ID (for hierarchical charts)
    pub parent_id: Option<AccountId>,
    /// Description
    pub description: Option<String>,
    /// Whether account is active
    pub is_active: bool,
}

impl Account {
    /// Creates a new account
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier
    /// * `code` - Account code
    /// * `name` - Account name
    /// * `account_type` - Type of account
    pub fn new(id: AccountId, code: impl Into<String>, name: impl Into<String>, account_type: AccountType) -> Self {
        Self {
            id,
            code: code.into(),
            name: name.into(),
            account_type,
            category: None,
            parent_id: None,
            description: None,
            is_active: true,
        }
    }

    /// Sets the account category
    pub fn with_category(mut self, category: AccountCategory) -> Self {
        self.category = Some(category);
        self
    }

    /// Sets the parent account
    pub fn with_parent(mut self, parent_id: AccountId) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Sets the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Standard chart of accounts for insurance
pub struct InsuranceChartOfAccounts;

impl InsuranceChartOfAccounts {
    /// Creates standard insurance accounts
    pub fn create_standard_accounts() -> Vec<Account> {
        vec![
            // Assets
            Account::new(AccountId::new(), "1000", "Cash", AccountType::Asset)
                .with_category(AccountCategory::Cash),
            Account::new(AccountId::new(), "1100", "Premium Receivable", AccountType::Asset)
                .with_category(AccountCategory::Receivables),
            Account::new(AccountId::new(), "1200", "Reinsurance Receivable", AccountType::Asset)
                .with_category(AccountCategory::Receivables),
            Account::new(AccountId::new(), "1300", "Investments", AccountType::Asset)
                .with_category(AccountCategory::Investments),

            // Liabilities
            Account::new(AccountId::new(), "2000", "Unearned Premium Reserve", AccountType::Liability)
                .with_category(AccountCategory::UnearnedPremium),
            Account::new(AccountId::new(), "2100", "Loss Reserves", AccountType::Liability)
                .with_category(AccountCategory::Reserves),
            Account::new(AccountId::new(), "2200", "IBNR Reserve", AccountType::Liability)
                .with_category(AccountCategory::Reserves),
            Account::new(AccountId::new(), "2300", "Commission Payable", AccountType::Liability)
                .with_category(AccountCategory::Payables),

            // Equity
            Account::new(AccountId::new(), "3000", "Retained Earnings", AccountType::Equity),

            // Revenue
            Account::new(AccountId::new(), "4000", "Written Premium", AccountType::Revenue)
                .with_category(AccountCategory::PremiumIncome),
            Account::new(AccountId::new(), "4100", "Earned Premium", AccountType::Revenue)
                .with_category(AccountCategory::PremiumIncome),
            Account::new(AccountId::new(), "4200", "Investment Income", AccountType::Revenue)
                .with_category(AccountCategory::InvestmentIncome),
            Account::new(AccountId::new(), "4300", "Policy Fees", AccountType::Revenue)
                .with_category(AccountCategory::FeeIncome),

            // Expenses
            Account::new(AccountId::new(), "5000", "Incurred Losses", AccountType::Expense)
                .with_category(AccountCategory::ClaimsExpense),
            Account::new(AccountId::new(), "5100", "Loss Adjustment Expense", AccountType::Expense)
                .with_category(AccountCategory::ClaimsExpense),
            Account::new(AccountId::new(), "5200", "Commission Expense", AccountType::Expense)
                .with_category(AccountCategory::CommissionExpense),
            Account::new(AccountId::new(), "5300", "Operating Expense", AccountType::Expense)
                .with_category(AccountCategory::OperatingExpense),
        ]
    }
}
