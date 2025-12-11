//! Transaction and posting types
//!
//! This module defines the structure of financial transactions
//! in the double-entry ledger system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use core_kernel::{AccountId, Money};

/// Type of posting (debit or credit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PostingType {
    /// Debit posting
    Debit,
    /// Credit posting
    Credit,
}

/// A single posting (line item) in a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Posting {
    /// Unique posting identifier
    pub id: Uuid,
    /// Account to post to
    pub account_id: AccountId,
    /// Amount (always positive)
    pub amount: Money,
    /// Debit or credit
    pub posting_type: PostingType,
    /// Optional description for this line
    pub description: Option<String>,
}

impl Posting {
    /// Creates a new debit posting
    ///
    /// # Arguments
    ///
    /// * `account_id` - Account to debit
    /// * `amount` - Amount to debit
    pub fn debit(account_id: AccountId, amount: Money) -> Self {
        Self {
            id: Uuid::new_v4(),
            account_id,
            amount,
            posting_type: PostingType::Debit,
            description: None,
        }
    }

    /// Creates a new credit posting
    ///
    /// # Arguments
    ///
    /// * `account_id` - Account to credit
    /// * `amount` - Amount to credit
    pub fn credit(account_id: AccountId, amount: Money) -> Self {
        Self {
            id: Uuid::new_v4(),
            account_id,
            amount,
            posting_type: PostingType::Credit,
            description: None,
        }
    }

    /// Adds a description to the posting
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// A financial transaction consisting of multiple postings
///
/// Transactions must always balance: total debits = total credits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction description
    pub description: String,
    /// Transaction date
    pub transaction_date: Option<DateTime<Utc>>,
    /// Reference type (e.g., "policy", "claim")
    pub reference_type: Option<String>,
    /// Reference ID
    pub reference_id: Option<Uuid>,
    /// List of postings
    pub postings: Vec<Posting>,
}

impl Transaction {
    /// Creates a new transaction
    ///
    /// # Arguments
    ///
    /// * `description` - Description of the transaction
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            transaction_date: None,
            reference_type: None,
            reference_id: None,
            postings: Vec::new(),
        }
    }

    /// Sets the transaction date
    pub fn dated(mut self, date: DateTime<Utc>) -> Self {
        self.transaction_date = Some(date);
        self
    }

    /// Sets the reference
    pub fn with_reference(mut self, ref_type: impl Into<String>, ref_id: Uuid) -> Self {
        self.reference_type = Some(ref_type.into());
        self.reference_id = Some(ref_id);
        self
    }

    /// Adds a debit posting
    pub fn debit(mut self, account_id: AccountId, amount: Money) -> Self {
        self.postings.push(Posting::debit(account_id, amount));
        self
    }

    /// Adds a credit posting
    pub fn credit(mut self, account_id: AccountId, amount: Money) -> Self {
        self.postings.push(Posting::credit(account_id, amount));
        self
    }

    /// Adds a custom posting
    pub fn posting(mut self, posting: Posting) -> Self {
        self.postings.push(posting);
        self
    }

    /// Checks if the transaction is balanced
    pub fn is_balanced(&self) -> bool {
        let mut debits = rust_decimal::Decimal::ZERO;
        let mut credits = rust_decimal::Decimal::ZERO;

        for posting in &self.postings {
            match posting.posting_type {
                PostingType::Debit => debits += posting.amount.amount(),
                PostingType::Credit => credits += posting.amount.amount(),
            }
        }

        debits == credits
    }
}

/// Builder for common insurance transactions
pub struct InsuranceTransactions;

impl InsuranceTransactions {
    /// Creates a premium receipt transaction
    ///
    /// # Arguments
    ///
    /// * `cash_account` - Cash account ID
    /// * `premium_account` - Premium revenue account ID
    /// * `amount` - Premium amount
    /// * `policy_id` - Policy reference
    pub fn premium_receipt(
        cash_account: AccountId,
        premium_account: AccountId,
        amount: Money,
        policy_id: Uuid,
    ) -> Transaction {
        Transaction::new("Premium payment received")
            .with_reference("policy", policy_id)
            .debit(cash_account, amount)
            .credit(premium_account, amount)
    }

    /// Creates a claim payment transaction
    ///
    /// # Arguments
    ///
    /// * `loss_account` - Incurred losses expense account
    /// * `cash_account` - Cash account ID
    /// * `amount` - Claim amount
    /// * `claim_id` - Claim reference
    pub fn claim_payment(
        loss_account: AccountId,
        cash_account: AccountId,
        amount: Money,
        claim_id: Uuid,
    ) -> Transaction {
        Transaction::new("Claim payment")
            .with_reference("claim", claim_id)
            .debit(loss_account, amount)
            .credit(cash_account, amount)
    }

    /// Creates a reserve establishment transaction
    ///
    /// # Arguments
    ///
    /// * `loss_account` - Incurred losses expense account
    /// * `reserve_account` - Loss reserve liability account
    /// * `amount` - Reserve amount
    /// * `claim_id` - Claim reference
    pub fn establish_reserve(
        loss_account: AccountId,
        reserve_account: AccountId,
        amount: Money,
        claim_id: Uuid,
    ) -> Transaction {
        Transaction::new("Establish loss reserve")
            .with_reference("claim", claim_id)
            .debit(loss_account, amount)
            .credit(reserve_account, amount)
    }

    /// Creates a commission expense transaction
    ///
    /// # Arguments
    ///
    /// * `commission_expense` - Commission expense account
    /// * `commission_payable` - Commission payable liability account
    /// * `amount` - Commission amount
    /// * `policy_id` - Policy reference
    pub fn commission_accrual(
        commission_expense: AccountId,
        commission_payable: AccountId,
        amount: Money,
        policy_id: Uuid,
    ) -> Transaction {
        Transaction::new("Commission accrual")
            .with_reference("policy", policy_id)
            .debit(commission_expense, amount)
            .credit(commission_payable, amount)
    }
}
