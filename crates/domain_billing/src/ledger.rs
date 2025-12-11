//! Double-entry ledger implementation
//!
//! This module provides the core ledger functionality, ensuring that
//! all transactions are balanced and maintain financial integrity.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::collections::HashMap;
use uuid::Uuid;

use core_kernel::{AccountId, JournalEntryId, Money, Currency};
use crate::account::{Account, AccountType};
use crate::transaction::{Transaction, Posting, PostingType};
use crate::error::BillingError;

/// The main ledger for tracking financial transactions
///
/// The Ledger enforces double-entry accounting rules, ensuring that
/// every transaction is balanced (debits = credits).
///
/// # Invariants
///
/// - All transactions must balance to zero
/// - Account balances are always consistent with postings
/// - Historical entries cannot be modified (only reversed)
#[derive(Debug)]
pub struct Ledger {
    /// Chart of accounts
    accounts: HashMap<AccountId, Account>,
    /// Journal entries
    journal_entries: Vec<JournalEntry>,
    /// Running account balances
    balances: HashMap<AccountId, Money>,
    /// Default currency
    currency: Currency,
}

impl Ledger {
    /// Creates a new ledger with the specified currency
    ///
    /// # Arguments
    ///
    /// * `currency` - The default currency for the ledger
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let ledger = Ledger::new(Currency::USD);
    /// ```
    pub fn new(currency: Currency) -> Self {
        Self {
            accounts: HashMap::new(),
            journal_entries: Vec::new(),
            balances: HashMap::new(),
            currency,
        }
    }

    /// Adds an account to the chart of accounts
    ///
    /// # Arguments
    ///
    /// * `account` - The account to add
    ///
    /// # Errors
    ///
    /// Returns error if account already exists
    pub fn add_account(&mut self, account: Account) -> Result<(), BillingError> {
        if self.accounts.contains_key(&account.id) {
            return Err(BillingError::AccountAlreadyExists(account.id.to_string()));
        }

        let account_id = account.id;
        self.accounts.insert(account_id, account);
        self.balances.insert(account_id, Money::zero(self.currency));

        Ok(())
    }

    /// Gets an account by ID
    ///
    /// # Arguments
    ///
    /// * `id` - The account identifier
    pub fn get_account(&self, id: &AccountId) -> Option<&Account> {
        self.accounts.get(id)
    }

    /// Gets the current balance of an account
    ///
    /// # Arguments
    ///
    /// * `id` - The account identifier
    ///
    /// # Returns
    ///
    /// The current balance, or None if account doesn't exist
    pub fn get_balance(&self, id: &AccountId) -> Option<Money> {
        self.balances.get(id).copied()
    }

    /// Posts a transaction to the ledger
    ///
    /// This method validates that the transaction is balanced and
    /// updates all affected account balances.
    ///
    /// # Arguments
    ///
    /// * `transaction` - The transaction to post
    ///
    /// # Returns
    ///
    /// The journal entry ID on success
    ///
    /// # Errors
    ///
    /// - Returns error if transaction is not balanced
    /// - Returns error if any referenced account doesn't exist
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let transaction = Transaction::new("Premium payment")
    ///     .debit(cash_account, amount)
    ///     .credit(revenue_account, amount);
    ///
    /// let entry_id = ledger.post(transaction)?;
    /// ```
    pub fn post(&mut self, transaction: Transaction) -> Result<JournalEntryId, BillingError> {
        // Validate transaction balance
        self.validate_balance(&transaction)?;

        // Validate all accounts exist
        for posting in &transaction.postings {
            if !self.accounts.contains_key(&posting.account_id) {
                return Err(BillingError::AccountNotFound(posting.account_id.to_string()));
            }
        }

        // Create journal entry
        let entry_id = JournalEntryId::new_v7();
        let now = Utc::now();

        let entry = JournalEntry {
            id: entry_id,
            transaction_date: transaction.transaction_date.unwrap_or(now),
            description: transaction.description,
            reference_type: transaction.reference_type,
            reference_id: transaction.reference_id,
            postings: transaction.postings.clone(),
            created_at: now,
        };

        // Update account balances
        for posting in &transaction.postings {
            let balance = self.balances.get_mut(&posting.account_id).unwrap();
            let account = self.accounts.get(&posting.account_id).unwrap();

            // Calculate balance change based on account type and posting type
            let change = self.calculate_balance_change(
                account.account_type,
                &posting.posting_type,
                posting.amount,
            );

            *balance = balance.checked_add(&change)
                .map_err(|e| BillingError::CalculationError(e.to_string()))?;
        }

        self.journal_entries.push(entry);

        Ok(entry_id)
    }

    /// Creates a reversal entry for a previous transaction
    ///
    /// # Arguments
    ///
    /// * `entry_id` - The journal entry to reverse
    /// * `reason` - Reason for the reversal
    ///
    /// # Returns
    ///
    /// The new reversal entry ID
    pub fn reverse(
        &mut self,
        entry_id: &JournalEntryId,
        reason: &str,
    ) -> Result<JournalEntryId, BillingError> {
        let original = self.journal_entries
            .iter()
            .find(|e| &e.id == entry_id)
            .ok_or_else(|| BillingError::EntryNotFound(entry_id.to_string()))?
            .clone();

        // Create reversed postings (swap debits and credits)
        let reversed_postings: Vec<Posting> = original.postings
            .iter()
            .map(|p| Posting {
                id: Uuid::new_v4(),
                account_id: p.account_id,
                amount: p.amount,
                posting_type: match p.posting_type {
                    PostingType::Debit => PostingType::Credit,
                    PostingType::Credit => PostingType::Debit,
                },
                description: Some(format!("Reversal: {}", reason)),
            })
            .collect();

        let reversal = Transaction {
            description: format!("Reversal of {}: {}", entry_id, reason),
            transaction_date: Some(Utc::now()),
            reference_type: Some("reversal".to_string()),
            reference_id: Some(*entry_id.as_uuid()),
            postings: reversed_postings,
        };

        self.post(reversal)
    }

    /// Validates that a transaction is balanced (debits = credits)
    fn validate_balance(&self, transaction: &Transaction) -> Result<(), BillingError> {
        let mut total_debits = Money::zero(self.currency);
        let mut total_credits = Money::zero(self.currency);

        for posting in &transaction.postings {
            match posting.posting_type {
                PostingType::Debit => {
                    total_debits = total_debits.checked_add(&posting.amount)
                        .map_err(|e| BillingError::CalculationError(e.to_string()))?;
                }
                PostingType::Credit => {
                    total_credits = total_credits.checked_add(&posting.amount)
                        .map_err(|e| BillingError::CalculationError(e.to_string()))?;
                }
            }
        }

        if total_debits != total_credits {
            return Err(BillingError::UnbalancedTransaction {
                debits: total_debits.amount(),
                credits: total_credits.amount(),
            });
        }

        Ok(())
    }

    /// Calculates the balance change for a posting
    ///
    /// In double-entry accounting:
    /// - Asset & Expense accounts: Debits increase, Credits decrease
    /// - Liability, Equity & Revenue accounts: Credits increase, Debits decrease
    fn calculate_balance_change(
        &self,
        account_type: AccountType,
        posting_type: &PostingType,
        amount: Money,
    ) -> Money {
        let is_debit_normal = matches!(
            account_type,
            AccountType::Asset | AccountType::Expense
        );

        match (is_debit_normal, posting_type) {
            (true, PostingType::Debit) => amount,
            (true, PostingType::Credit) => -amount,
            (false, PostingType::Debit) => -amount,
            (false, PostingType::Credit) => amount,
        }
    }

    /// Generates a trial balance report
    ///
    /// # Returns
    ///
    /// A trial balance showing all account balances
    pub fn trial_balance(&self) -> TrialBalance {
        let mut entries = Vec::new();
        let mut total_debits = Money::zero(self.currency);
        let mut total_credits = Money::zero(self.currency);

        for (account_id, balance) in &self.balances {
            let account = self.accounts.get(account_id).unwrap();

            let (debit, credit) = if balance.is_negative() {
                (Money::zero(self.currency), balance.abs())
            } else {
                (*balance, Money::zero(self.currency))
            };

            // Adjust based on account normal balance
            let (debit, credit) = match account.account_type {
                AccountType::Asset | AccountType::Expense => (balance.abs(), Money::zero(self.currency)),
                _ => (Money::zero(self.currency), balance.abs()),
            };

            if !balance.is_zero() {
                entries.push(TrialBalanceEntry {
                    account_id: *account_id,
                    account_name: account.name.clone(),
                    debit,
                    credit,
                });

                total_debits = total_debits + debit;
                total_credits = total_credits + credit;
            }
        }

        TrialBalance {
            entries,
            total_debits,
            total_credits,
            is_balanced: total_debits == total_credits,
        }
    }
}

/// A journal entry in the ledger
#[derive(Debug, Clone)]
pub struct JournalEntry {
    /// Unique entry identifier
    pub id: JournalEntryId,
    /// Transaction date
    pub transaction_date: DateTime<Utc>,
    /// Description
    pub description: String,
    /// Reference type (e.g., "policy", "claim")
    pub reference_type: Option<String>,
    /// Reference ID
    pub reference_id: Option<Uuid>,
    /// Individual postings
    pub postings: Vec<Posting>,
    /// When entry was created
    pub created_at: DateTime<Utc>,
}

/// Trial balance report
#[derive(Debug)]
pub struct TrialBalance {
    /// Individual account entries
    pub entries: Vec<TrialBalanceEntry>,
    /// Total debits
    pub total_debits: Money,
    /// Total credits
    pub total_credits: Money,
    /// Whether the trial balance is balanced
    pub is_balanced: bool,
}

/// A single entry in the trial balance
#[derive(Debug)]
pub struct TrialBalanceEntry {
    /// Account ID
    pub account_id: AccountId,
    /// Account name
    pub account_name: String,
    /// Debit balance
    pub debit: Money,
    /// Credit balance
    pub credit: Money,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn setup_ledger() -> Ledger {
        let mut ledger = Ledger::new(Currency::USD);

        // Add accounts
        ledger.add_account(Account::new(
            AccountId::new(),
            "1000",
            "Cash",
            AccountType::Asset,
        )).unwrap();

        ledger.add_account(Account::new(
            AccountId::new(),
            "4000",
            "Premium Revenue",
            AccountType::Revenue,
        )).unwrap();

        ledger
    }

    #[test]
    fn test_balanced_transaction() {
        let mut ledger = setup_ledger();
        let accounts: Vec<_> = ledger.accounts.keys().copied().collect();

        let transaction = Transaction::new("Premium payment")
            .debit(accounts[0], Money::new(dec!(1000), Currency::USD))
            .credit(accounts[1], Money::new(dec!(1000), Currency::USD));

        let result = ledger.post(transaction);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unbalanced_transaction() {
        let mut ledger = setup_ledger();
        let accounts: Vec<_> = ledger.accounts.keys().copied().collect();

        let transaction = Transaction::new("Unbalanced")
            .debit(accounts[0], Money::new(dec!(1000), Currency::USD))
            .credit(accounts[1], Money::new(dec!(500), Currency::USD));

        let result = ledger.post(transaction);
        assert!(matches!(result, Err(BillingError::UnbalancedTransaction { .. })));
    }
}
