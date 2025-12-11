//! Invoice management
//!
//! This module handles invoice creation and management for premium billing.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use core_kernel::{InvoiceId, PolicyId, PartyId, Money, Currency};

/// Invoice status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvoiceStatus {
    /// Invoice is being drafted
    Draft,
    /// Invoice has been issued
    Issued,
    /// Invoice has been sent to customer
    Sent,
    /// Partial payment received
    PartiallyPaid,
    /// Fully paid
    Paid,
    /// Past due date
    Overdue,
    /// Cancelled/voided
    Cancelled,
    /// Written off as bad debt
    WrittenOff,
}

/// An invoice for premium or fees
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    /// Unique identifier
    pub id: InvoiceId,
    /// Invoice number (human-readable)
    pub invoice_number: String,
    /// Related policy ID
    pub policy_id: PolicyId,
    /// Customer ID
    pub customer_id: PartyId,
    /// Invoice date
    pub invoice_date: NaiveDate,
    /// Due date
    pub due_date: NaiveDate,
    /// Currency
    pub currency: Currency,
    /// Invoice items
    pub items: Vec<InvoiceItem>,
    /// Subtotal
    pub subtotal: Money,
    /// Tax amount
    pub tax: Option<Money>,
    /// Total amount
    pub total: Money,
    /// Amount paid
    pub amount_paid: Money,
    /// Status
    pub status: InvoiceStatus,
    /// Notes
    pub notes: Option<String>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl Invoice {
    /// Creates a new invoice
    ///
    /// # Arguments
    ///
    /// * `policy_id` - Related policy
    /// * `customer_id` - Customer being billed
    /// * `due_date` - Payment due date
    /// * `currency` - Invoice currency
    pub fn new(
        policy_id: PolicyId,
        customer_id: PartyId,
        due_date: NaiveDate,
        currency: Currency,
    ) -> Self {
        let now = Utc::now();
        let id = InvoiceId::new_v7();

        Self {
            id,
            invoice_number: generate_invoice_number(),
            policy_id,
            customer_id,
            invoice_date: now.date_naive(),
            due_date,
            currency,
            items: Vec::new(),
            subtotal: Money::zero(currency),
            tax: None,
            total: Money::zero(currency),
            amount_paid: Money::zero(currency),
            status: InvoiceStatus::Draft,
            notes: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Adds an item to the invoice
    pub fn add_item(&mut self, item: InvoiceItem) {
        self.items.push(item);
        self.recalculate_totals();
    }

    /// Sets the tax amount
    pub fn with_tax(mut self, tax: Money) -> Self {
        self.tax = Some(tax);
        self.recalculate_totals();
        self
    }

    /// Issues the invoice
    pub fn issue(&mut self) {
        self.status = InvoiceStatus::Issued;
        self.updated_at = Utc::now();
    }

    /// Records a payment against the invoice
    pub fn record_payment(&mut self, amount: Money) {
        self.amount_paid = self.amount_paid + amount;
        self.updated_at = Utc::now();

        if self.amount_paid >= self.total {
            self.status = InvoiceStatus::Paid;
        } else if self.amount_paid > Money::zero(self.currency) {
            self.status = InvoiceStatus::PartiallyPaid;
        }
    }

    /// Checks if invoice is overdue
    pub fn is_overdue(&self) -> bool {
        let today = Utc::now().date_naive();
        today > self.due_date && !matches!(self.status, InvoiceStatus::Paid | InvoiceStatus::Cancelled)
    }

    /// Returns the balance due
    pub fn balance_due(&self) -> Money {
        self.total - self.amount_paid
    }

    /// Recalculates totals based on items
    fn recalculate_totals(&mut self) {
        self.subtotal = self.items.iter()
            .fold(Money::zero(self.currency), |acc, item| acc + item.total());

        self.total = match &self.tax {
            Some(tax) => self.subtotal + *tax,
            None => self.subtotal,
        };
    }
}

/// A line item on an invoice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceItem {
    /// Item ID
    pub id: Uuid,
    /// Description
    pub description: String,
    /// Item type
    pub item_type: InvoiceItemType,
    /// Quantity
    pub quantity: Decimal,
    /// Unit price
    pub unit_price: Money,
    /// Discount (if any)
    pub discount: Option<Money>,
}

impl InvoiceItem {
    /// Creates a new invoice item
    pub fn new(description: impl Into<String>, item_type: InvoiceItemType, unit_price: Money) -> Self {
        Self {
            id: Uuid::new_v4(),
            description: description.into(),
            item_type,
            quantity: Decimal::ONE,
            unit_price,
            discount: None,
        }
    }

    /// Sets the quantity
    pub fn with_quantity(mut self, quantity: Decimal) -> Self {
        self.quantity = quantity;
        self
    }

    /// Applies a discount
    pub fn with_discount(mut self, discount: Money) -> Self {
        self.discount = Some(discount);
        self
    }

    /// Calculates the total for this item
    pub fn total(&self) -> Money {
        let gross = self.unit_price * self.quantity;
        match &self.discount {
            Some(d) => gross - *d,
            None => gross,
        }
    }
}

/// Types of invoice items
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvoiceItemType {
    /// Premium charge
    Premium,
    /// Policy fee
    PolicyFee,
    /// Endorsement fee
    EndorsementFee,
    /// Reinstatement fee
    ReinstatementFee,
    /// Late payment fee
    LateFee,
    /// Tax
    Tax,
    /// Other charge
    Other,
}

/// Generates a unique invoice number
fn generate_invoice_number() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("INV-{}", duration.as_millis() % 10_000_000_000)
}
