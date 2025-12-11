//! Comprehensive tests for domain_billing

use chrono::{NaiveDate, Utc, Days};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use uuid::Uuid;

use core_kernel::{AccountId, InvoiceId, PolicyId, PartyId, PaymentId, Money, Currency};

use domain_billing::account::{Account, AccountType, AccountCategory, InsuranceChartOfAccounts};
use domain_billing::invoice::{Invoice, InvoiceItem, InvoiceItemType, InvoiceStatus};
use domain_billing::payment::{Payment, PaymentMethod, PaymentStatus, PaymentAllocation};
use domain_billing::transaction::{Transaction, Posting, PostingType, InsuranceTransactions};
use domain_billing::ledger::Ledger;

// ============================================================================
// Account Tests
// ============================================================================

mod account_tests {
    use super::*;

    #[test]
    fn test_account_type_is_debit_normal() {
        assert!(AccountType::Asset.is_debit_normal());
        assert!(AccountType::Expense.is_debit_normal());
        assert!(!AccountType::Liability.is_debit_normal());
        assert!(!AccountType::Equity.is_debit_normal());
        assert!(!AccountType::Revenue.is_debit_normal());
    }

    #[test]
    fn test_account_new() {
        let id = AccountId::new();
        let account = Account::new(id, "1000", "Cash", AccountType::Asset);

        assert_eq!(account.id, id);
        assert_eq!(account.code, "1000");
        assert_eq!(account.name, "Cash");
        assert_eq!(account.account_type, AccountType::Asset);
        assert!(account.is_active);
        assert!(account.category.is_none());
        assert!(account.parent_id.is_none());
        assert!(account.description.is_none());
    }

    #[test]
    fn test_account_with_category() {
        let account = Account::new(AccountId::new(), "1000", "Cash", AccountType::Asset)
            .with_category(AccountCategory::Cash);

        assert_eq!(account.category, Some(AccountCategory::Cash));
    }

    #[test]
    fn test_account_with_parent() {
        let parent_id = AccountId::new();
        let account = Account::new(AccountId::new(), "1010", "Petty Cash", AccountType::Asset)
            .with_parent(parent_id);

        assert_eq!(account.parent_id, Some(parent_id));
    }

    #[test]
    fn test_account_with_description() {
        let account = Account::new(AccountId::new(), "1000", "Cash", AccountType::Asset)
            .with_description("Main operating cash account");

        assert_eq!(account.description, Some("Main operating cash account".to_string()));
    }

    #[test]
    fn test_insurance_chart_of_accounts() {
        let accounts = InsuranceChartOfAccounts::create_standard_accounts();

        assert!(!accounts.is_empty());

        // Check we have different account types
        let asset_count = accounts.iter().filter(|a| a.account_type == AccountType::Asset).count();
        let liability_count = accounts.iter().filter(|a| a.account_type == AccountType::Liability).count();
        let revenue_count = accounts.iter().filter(|a| a.account_type == AccountType::Revenue).count();
        let expense_count = accounts.iter().filter(|a| a.account_type == AccountType::Expense).count();

        assert!(asset_count > 0);
        assert!(liability_count > 0);
        assert!(revenue_count > 0);
        assert!(expense_count > 0);
    }

    #[test]
    fn test_all_account_categories() {
        // Test that all categories can be serialized
        let categories = vec![
            AccountCategory::Cash,
            AccountCategory::Receivables,
            AccountCategory::Investments,
            AccountCategory::FixedAssets,
            AccountCategory::Payables,
            AccountCategory::Reserves,
            AccountCategory::UnearnedPremium,
            AccountCategory::PremiumIncome,
            AccountCategory::InvestmentIncome,
            AccountCategory::FeeIncome,
            AccountCategory::ClaimsExpense,
            AccountCategory::CommissionExpense,
            AccountCategory::OperatingExpense,
            AccountCategory::Other,
        ];

        for category in categories {
            let json = serde_json::to_string(&category).unwrap();
            assert!(!json.is_empty());
        }
    }
}

// ============================================================================
// Invoice Tests
// ============================================================================

mod invoice_tests {
    use super::*;

    fn create_test_invoice() -> Invoice {
        let policy_id = PolicyId::new_v7();
        let customer_id = PartyId::new_v7();
        let due_date = Utc::now().date_naive() + Days::new(30);
        Invoice::new(policy_id, customer_id, due_date, Currency::USD)
    }

    #[test]
    fn test_invoice_new() {
        let invoice = create_test_invoice();

        assert_eq!(invoice.status, InvoiceStatus::Draft);
        assert_eq!(invoice.currency, Currency::USD);
        assert!(invoice.invoice_number.starts_with("INV-"));
        assert!(invoice.items.is_empty());
        assert_eq!(invoice.subtotal, Money::zero(Currency::USD));
        assert_eq!(invoice.total, Money::zero(Currency::USD));
        assert_eq!(invoice.amount_paid, Money::zero(Currency::USD));
    }

    #[test]
    fn test_invoice_add_item() {
        let mut invoice = create_test_invoice();
        let item = InvoiceItem::new("Annual Premium", InvoiceItemType::Premium, Money::new(dec!(1000), Currency::USD));

        invoice.add_item(item);

        assert_eq!(invoice.items.len(), 1);
        assert_eq!(invoice.subtotal.amount(), dec!(1000));
        assert_eq!(invoice.total.amount(), dec!(1000));
    }

    #[test]
    fn test_invoice_with_tax() {
        let invoice = create_test_invoice();
        let invoice = invoice.with_tax(Money::new(dec!(100), Currency::USD));

        assert!(invoice.tax.is_some());
        assert_eq!(invoice.tax.unwrap().amount(), dec!(100));
    }

    #[test]
    fn test_invoice_issue() {
        let mut invoice = create_test_invoice();
        invoice.issue();

        assert_eq!(invoice.status, InvoiceStatus::Issued);
    }

    #[test]
    fn test_invoice_record_payment_partial() {
        let mut invoice = create_test_invoice();
        let item = InvoiceItem::new("Premium", InvoiceItemType::Premium, Money::new(dec!(1000), Currency::USD));
        invoice.add_item(item);

        invoice.record_payment(Money::new(dec!(500), Currency::USD));

        assert_eq!(invoice.status, InvoiceStatus::PartiallyPaid);
        assert_eq!(invoice.amount_paid.amount(), dec!(500));
    }

    #[test]
    fn test_invoice_record_payment_full() {
        let mut invoice = create_test_invoice();
        let item = InvoiceItem::new("Premium", InvoiceItemType::Premium, Money::new(dec!(1000), Currency::USD));
        invoice.add_item(item);

        invoice.record_payment(Money::new(dec!(1000), Currency::USD));

        assert_eq!(invoice.status, InvoiceStatus::Paid);
    }

    #[test]
    fn test_invoice_is_overdue() {
        let policy_id = PolicyId::new_v7();
        let customer_id = PartyId::new_v7();
        let past_due_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let mut invoice = Invoice::new(policy_id, customer_id, past_due_date, Currency::USD);
        invoice.status = InvoiceStatus::Issued;

        assert!(invoice.is_overdue());
    }

    #[test]
    fn test_invoice_balance_due() {
        let mut invoice = create_test_invoice();
        let item = InvoiceItem::new("Premium", InvoiceItemType::Premium, Money::new(dec!(1000), Currency::USD));
        invoice.add_item(item);
        invoice.record_payment(Money::new(dec!(300), Currency::USD));

        let balance = invoice.balance_due();
        assert_eq!(balance.amount(), dec!(700));
    }

    #[test]
    fn test_invoice_item_new() {
        let item = InvoiceItem::new("Premium", InvoiceItemType::Premium, Money::new(dec!(100), Currency::USD));

        assert_eq!(item.description, "Premium");
        assert_eq!(item.item_type, InvoiceItemType::Premium);
        assert_eq!(item.quantity, Decimal::ONE);
        assert!(item.discount.is_none());
    }

    #[test]
    fn test_invoice_item_with_quantity() {
        let item = InvoiceItem::new("Installment", InvoiceItemType::Premium, Money::new(dec!(100), Currency::USD))
            .with_quantity(dec!(12));

        assert_eq!(item.quantity, dec!(12));
        assert_eq!(item.total().amount(), dec!(1200));
    }

    #[test]
    fn test_invoice_item_with_discount() {
        let item = InvoiceItem::new("Premium", InvoiceItemType::Premium, Money::new(dec!(1000), Currency::USD))
            .with_discount(Money::new(dec!(100), Currency::USD));

        assert!(item.discount.is_some());
        assert_eq!(item.total().amount(), dec!(900));
    }

    #[test]
    fn test_all_invoice_item_types() {
        let types = vec![
            InvoiceItemType::Premium,
            InvoiceItemType::PolicyFee,
            InvoiceItemType::EndorsementFee,
            InvoiceItemType::ReinstatementFee,
            InvoiceItemType::LateFee,
            InvoiceItemType::Tax,
            InvoiceItemType::Other,
        ];

        for item_type in types {
            let json = serde_json::to_string(&item_type).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_all_invoice_statuses() {
        let statuses = vec![
            InvoiceStatus::Draft,
            InvoiceStatus::Issued,
            InvoiceStatus::Sent,
            InvoiceStatus::PartiallyPaid,
            InvoiceStatus::Paid,
            InvoiceStatus::Overdue,
            InvoiceStatus::Cancelled,
            InvoiceStatus::WrittenOff,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            assert!(!json.is_empty());
        }
    }
}

// ============================================================================
// Payment Tests
// ============================================================================

mod payment_tests {
    use super::*;

    fn create_test_payment() -> Payment {
        let invoice_id = InvoiceId::new_v7();
        let payer_id = PartyId::new_v7();
        let amount = Money::new(dec!(500), Currency::USD);
        Payment::new(invoice_id, payer_id, amount, PaymentMethod::BankTransfer)
    }

    #[test]
    fn test_payment_new() {
        let payment = create_test_payment();

        assert_eq!(payment.status, PaymentStatus::Pending);
        assert_eq!(payment.amount.amount(), dec!(500));
        assert_eq!(payment.method, PaymentMethod::BankTransfer);
        assert!(payment.external_reference.is_none());
        assert!(payment.completed_at.is_none());
    }

    #[test]
    fn test_payment_with_reference() {
        let payment = create_test_payment()
            .with_reference("TXN-123456");

        assert_eq!(payment.external_reference, Some("TXN-123456".to_string()));
    }

    #[test]
    fn test_payment_complete() {
        let mut payment = create_test_payment();
        payment.complete();

        assert_eq!(payment.status, PaymentStatus::Completed);
        assert!(payment.completed_at.is_some());
    }

    #[test]
    fn test_payment_fail() {
        let mut payment = create_test_payment();
        payment.fail("Insufficient funds");

        assert_eq!(payment.status, PaymentStatus::Failed);
        assert_eq!(payment.notes, Some("Insufficient funds".to_string()));
    }

    #[test]
    fn test_payment_reverse() {
        let mut payment = create_test_payment();
        payment.complete();
        payment.reverse("Customer requested refund");

        assert_eq!(payment.status, PaymentStatus::Reversed);
        assert!(payment.notes.as_ref().unwrap().contains("Reversed"));
    }

    #[test]
    fn test_all_payment_methods() {
        let methods = vec![
            PaymentMethod::BankTransfer,
            PaymentMethod::CreditCard,
            PaymentMethod::DebitCard,
            PaymentMethod::DirectDebit,
            PaymentMethod::Check,
            PaymentMethod::Cash,
            PaymentMethod::DigitalWallet,
            PaymentMethod::SalaryDeduction,
        ];

        for method in methods {
            let json = serde_json::to_string(&method).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_all_payment_statuses() {
        let statuses = vec![
            PaymentStatus::Pending,
            PaymentStatus::Completed,
            PaymentStatus::Failed,
            PaymentStatus::Reversed,
            PaymentStatus::OnHold,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_payment_allocation() {
        let allocation = PaymentAllocation {
            payment_id: PaymentId::new_v7(),
            invoice_id: InvoiceId::new_v7(),
            amount: Money::new(dec!(100), Currency::USD),
            allocated_at: Utc::now(),
        };

        let json = serde_json::to_string(&allocation).unwrap();
        let deserialized: PaymentAllocation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.amount.amount(), dec!(100));
    }
}

// ============================================================================
// Transaction Tests
// ============================================================================

mod transaction_tests {
    use super::*;

    #[test]
    fn test_posting_debit() {
        let account_id = AccountId::new();
        let posting = Posting::debit(account_id, Money::new(dec!(100), Currency::USD));

        assert_eq!(posting.account_id, account_id);
        assert_eq!(posting.amount.amount(), dec!(100));
        assert_eq!(posting.posting_type, PostingType::Debit);
    }

    #[test]
    fn test_posting_credit() {
        let account_id = AccountId::new();
        let posting = Posting::credit(account_id, Money::new(dec!(100), Currency::USD));

        assert_eq!(posting.posting_type, PostingType::Credit);
    }

    #[test]
    fn test_posting_with_description() {
        let posting = Posting::debit(AccountId::new(), Money::new(dec!(100), Currency::USD))
            .with_description("Premium payment");

        assert_eq!(posting.description, Some("Premium payment".to_string()));
    }

    #[test]
    fn test_transaction_new() {
        let txn = Transaction::new("Test transaction");

        assert_eq!(txn.description, "Test transaction");
        assert!(txn.postings.is_empty());
        assert!(txn.transaction_date.is_none());
        assert!(txn.reference_type.is_none());
    }

    #[test]
    fn test_transaction_dated() {
        let date = Utc::now();
        let txn = Transaction::new("Test").dated(date);

        assert!(txn.transaction_date.is_some());
    }

    #[test]
    fn test_transaction_with_reference() {
        let ref_id = Uuid::new_v4();
        let txn = Transaction::new("Test")
            .with_reference("policy", ref_id);

        assert_eq!(txn.reference_type, Some("policy".to_string()));
        assert_eq!(txn.reference_id, Some(ref_id));
    }

    #[test]
    fn test_transaction_debit_credit() {
        let cash = AccountId::new();
        let revenue = AccountId::new();
        let amount = Money::new(dec!(1000), Currency::USD);

        let txn = Transaction::new("Premium payment")
            .debit(cash, amount)
            .credit(revenue, amount);

        assert_eq!(txn.postings.len(), 2);
        assert!(txn.is_balanced());
    }

    #[test]
    fn test_transaction_posting() {
        let posting = Posting::debit(AccountId::new(), Money::new(dec!(100), Currency::USD));
        let txn = Transaction::new("Test").posting(posting);

        assert_eq!(txn.postings.len(), 1);
    }

    #[test]
    fn test_transaction_is_balanced_true() {
        let cash = AccountId::new();
        let revenue = AccountId::new();
        let amount = Money::new(dec!(1000), Currency::USD);

        let txn = Transaction::new("Balanced")
            .debit(cash, amount)
            .credit(revenue, amount);

        assert!(txn.is_balanced());
    }

    #[test]
    fn test_transaction_is_balanced_false() {
        let cash = AccountId::new();
        let revenue = AccountId::new();

        let txn = Transaction::new("Unbalanced")
            .debit(cash, Money::new(dec!(1000), Currency::USD))
            .credit(revenue, Money::new(dec!(500), Currency::USD));

        assert!(!txn.is_balanced());
    }

    #[test]
    fn test_insurance_transactions_premium_receipt() {
        let cash = AccountId::new();
        let premium = AccountId::new();
        let amount = Money::new(dec!(1000), Currency::USD);
        let policy_id = Uuid::new_v4();

        let txn = InsuranceTransactions::premium_receipt(cash, premium, amount, policy_id);

        assert!(txn.is_balanced());
        assert_eq!(txn.postings.len(), 2);
        assert_eq!(txn.reference_type, Some("policy".to_string()));
    }

    #[test]
    fn test_insurance_transactions_claim_payment() {
        let loss = AccountId::new();
        let cash = AccountId::new();
        let amount = Money::new(dec!(5000), Currency::USD);
        let claim_id = Uuid::new_v4();

        let txn = InsuranceTransactions::claim_payment(loss, cash, amount, claim_id);

        assert!(txn.is_balanced());
        assert_eq!(txn.reference_type, Some("claim".to_string()));
    }

    #[test]
    fn test_insurance_transactions_establish_reserve() {
        let loss = AccountId::new();
        let reserve = AccountId::new();
        let amount = Money::new(dec!(10000), Currency::USD);
        let claim_id = Uuid::new_v4();

        let txn = InsuranceTransactions::establish_reserve(loss, reserve, amount, claim_id);

        assert!(txn.is_balanced());
    }

    #[test]
    fn test_insurance_transactions_commission_accrual() {
        let expense = AccountId::new();
        let payable = AccountId::new();
        let amount = Money::new(dec!(100), Currency::USD);
        let policy_id = Uuid::new_v4();

        let txn = InsuranceTransactions::commission_accrual(expense, payable, amount, policy_id);

        assert!(txn.is_balanced());
    }
}

// ============================================================================
// Ledger Tests
// ============================================================================

mod ledger_tests {
    use super::*;

    fn setup_ledger_with_accounts() -> (Ledger, AccountId, AccountId) {
        let mut ledger = Ledger::new(Currency::USD);

        let cash_id = AccountId::new();
        let revenue_id = AccountId::new();

        ledger.add_account(Account::new(cash_id, "1000", "Cash", AccountType::Asset)).unwrap();
        ledger.add_account(Account::new(revenue_id, "4000", "Revenue", AccountType::Revenue)).unwrap();

        (ledger, cash_id, revenue_id)
    }

    #[test]
    fn test_ledger_add_account() {
        let mut ledger = Ledger::new(Currency::USD);
        let account = Account::new(AccountId::new(), "1000", "Cash", AccountType::Asset);
        let id = account.id;

        let result = ledger.add_account(account);
        assert!(result.is_ok());

        assert!(ledger.get_account(&id).is_some());
    }

    #[test]
    fn test_ledger_add_duplicate_account() {
        let mut ledger = Ledger::new(Currency::USD);
        let id = AccountId::new();
        let account1 = Account::new(id, "1000", "Cash", AccountType::Asset);
        let account2 = Account::new(id, "1001", "Cash 2", AccountType::Asset);

        ledger.add_account(account1).unwrap();
        let result = ledger.add_account(account2);

        assert!(result.is_err());
    }

    #[test]
    fn test_ledger_get_balance() {
        let (ledger, cash_id, _) = setup_ledger_with_accounts();

        let balance = ledger.get_balance(&cash_id);
        assert!(balance.is_some());
        assert_eq!(balance.unwrap().amount(), Decimal::ZERO);
    }

    #[test]
    fn test_ledger_get_balance_nonexistent() {
        let ledger = Ledger::new(Currency::USD);
        let balance = ledger.get_balance(&AccountId::new());
        assert!(balance.is_none());
    }

    #[test]
    fn test_ledger_post_balanced_transaction() {
        let (mut ledger, cash_id, revenue_id) = setup_ledger_with_accounts();
        let amount = Money::new(dec!(1000), Currency::USD);

        let txn = Transaction::new("Premium")
            .debit(cash_id, amount)
            .credit(revenue_id, amount);

        let result = ledger.post(txn);
        assert!(result.is_ok());

        // Check balances updated
        let cash_balance = ledger.get_balance(&cash_id).unwrap();
        assert_eq!(cash_balance.amount(), dec!(1000));
    }

    #[test]
    fn test_ledger_trial_balance() {
        let (mut ledger, cash_id, revenue_id) = setup_ledger_with_accounts();
        let amount = Money::new(dec!(1000), Currency::USD);

        let txn = Transaction::new("Premium")
            .debit(cash_id, amount)
            .credit(revenue_id, amount);

        ledger.post(txn).unwrap();

        let trial = ledger.trial_balance();
        assert!(trial.is_balanced);
    }

    #[test]
    fn test_ledger_reverse_entry() {
        let (mut ledger, cash_id, revenue_id) = setup_ledger_with_accounts();
        let amount = Money::new(dec!(1000), Currency::USD);

        let txn = Transaction::new("Premium")
            .debit(cash_id, amount)
            .credit(revenue_id, amount);

        let entry_id = ledger.post(txn).unwrap();

        // Reverse
        let reversal_result = ledger.reverse(&entry_id, "Customer refund");
        assert!(reversal_result.is_ok());

        // Check balance is back to zero
        let cash_balance = ledger.get_balance(&cash_id).unwrap();
        assert_eq!(cash_balance.amount(), Decimal::ZERO);
    }
}
