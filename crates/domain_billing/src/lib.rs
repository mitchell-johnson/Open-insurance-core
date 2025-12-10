//! Billing Domain - Double-Entry Ledger System
//!
//! This crate implements a strict double-entry bookkeeping system for the
//! insurance core, ensuring financial integrity for all monetary transactions.
//!
//! # Double-Entry Accounting Principles
//!
//! Every financial transaction creates balanced debits and credits:
//! - Debits increase asset/expense accounts
//! - Credits increase liability/equity/revenue accounts
//! - The sum of all debits must equal the sum of all credits
//!
//! # Account Types
//!
//! - **Assets**: Cash, Receivables, Investments
//! - **Liabilities**: Reserves, Payables, Unearned Premium
//! - **Equity**: Capital, Retained Earnings
//! - **Revenue**: Premium Income, Investment Income
//! - **Expenses**: Claims Paid, Commissions, Operating Expenses
//!
//! # Example
//!
//! ```rust,ignore
//! use domain_billing::{Ledger, Transaction, Posting};
//!
//! let mut ledger = Ledger::new();
//!
//! // Record premium receipt
//! let transaction = Transaction::new("Premium payment received")
//!     .debit(cash_account, premium_amount)
//!     .credit(premium_revenue, premium_amount);
//!
//! ledger.post(transaction)?;
//! ```

pub mod ledger;
pub mod account;
pub mod transaction;
pub mod invoice;
pub mod payment;
pub mod error;

pub use ledger::Ledger;
pub use account::{Account, AccountType, AccountCategory};
pub use transaction::{Transaction, Posting, PostingType};
pub use invoice::{Invoice, InvoiceItem, InvoiceStatus};
pub use payment::{Payment, PaymentMethod, PaymentStatus};
pub use error::BillingError;
