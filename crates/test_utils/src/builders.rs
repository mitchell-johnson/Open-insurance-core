//! Test Data Builders
//!
//! Provides builder patterns for constructing test data with sensible defaults.
//! These builders allow tests to specify only the relevant fields while using
//! defaults for everything else.

use chrono::{DateTime, NaiveDate, Utc, TimeZone};
use core_kernel::{
    Money, Currency, ValidPeriod,
    PolicyId, ClaimId, PartyId, AccountId, FundId,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::fixtures::{MoneyFixtures, TemporalFixtures, StringFixtures, IdFixtures};

/// Builder for constructing test policy data
pub struct TestPolicyDataBuilder {
    policy_id: PolicyId,
    policy_number: String,
    product_code: String,
    policyholder_id: PartyId,
    effective_date: DateTime<Utc>,
    expiry_date: DateTime<Utc>,
    premium: Money,
    sum_assured: Money,
}

impl Default for TestPolicyDataBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TestPolicyDataBuilder {
    /// Creates a new builder with default values
    pub fn new() -> Self {
        Self {
            policy_id: PolicyId::new(),
            policy_number: StringFixtures::policy_number().to_string(),
            product_code: StringFixtures::product_code().to_string(),
            policyholder_id: IdFixtures::party_id(),
            effective_date: TemporalFixtures::policy_start(),
            expiry_date: TemporalFixtures::policy_end(),
            premium: MoneyFixtures::usd_premium(),
            sum_assured: MoneyFixtures::usd_sum_assured(),
        }
    }

    /// Sets the policy ID
    pub fn with_policy_id(mut self, id: PolicyId) -> Self {
        self.policy_id = id;
        self
    }

    /// Sets the policy number
    pub fn with_policy_number(mut self, number: impl Into<String>) -> Self {
        self.policy_number = number.into();
        self
    }

    /// Sets the product code
    pub fn with_product_code(mut self, code: impl Into<String>) -> Self {
        self.product_code = code.into();
        self
    }

    /// Sets the policyholder ID
    pub fn with_policyholder_id(mut self, id: PartyId) -> Self {
        self.policyholder_id = id;
        self
    }

    /// Sets the effective date
    pub fn with_effective_date(mut self, date: DateTime<Utc>) -> Self {
        self.effective_date = date;
        self
    }

    /// Sets the expiry date
    pub fn with_expiry_date(mut self, date: DateTime<Utc>) -> Self {
        self.expiry_date = date;
        self
    }

    /// Sets the premium
    pub fn with_premium(mut self, premium: Money) -> Self {
        self.premium = premium;
        self
    }

    /// Sets the sum assured
    pub fn with_sum_assured(mut self, sum_assured: Money) -> Self {
        self.sum_assured = sum_assured;
        self
    }

    /// Sets the term in years from effective date
    pub fn with_term_years(mut self, years: i32) -> Self {
        self.expiry_date = self.effective_date + chrono::Duration::days(years as i64 * 365);
        self
    }

    /// Builds the test policy data
    pub fn build(self) -> TestPolicyData {
        TestPolicyData {
            policy_id: self.policy_id,
            policy_number: self.policy_number,
            product_code: self.product_code,
            policyholder_id: self.policyholder_id,
            effective_date: self.effective_date,
            expiry_date: self.expiry_date,
            premium: self.premium,
            sum_assured: self.sum_assured,
        }
    }
}

/// Test policy data structure
#[derive(Debug, Clone)]
pub struct TestPolicyData {
    pub policy_id: PolicyId,
    pub policy_number: String,
    pub product_code: String,
    pub policyholder_id: PartyId,
    pub effective_date: DateTime<Utc>,
    pub expiry_date: DateTime<Utc>,
    pub premium: Money,
    pub sum_assured: Money,
}

/// Builder for constructing test claim data
pub struct TestClaimDataBuilder {
    claim_id: ClaimId,
    claim_number: String,
    policy_id: PolicyId,
    claimant_id: PartyId,
    loss_date: NaiveDate,
    claimed_amount: Money,
    loss_description: String,
}

impl Default for TestClaimDataBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TestClaimDataBuilder {
    /// Creates a new builder with default values
    pub fn new() -> Self {
        Self {
            claim_id: ClaimId::new(),
            claim_number: StringFixtures::claim_number().to_string(),
            policy_id: IdFixtures::policy_id(),
            claimant_id: IdFixtures::party_id(),
            loss_date: TemporalFixtures::loss_date(),
            claimed_amount: Money::new(dec!(50000.00), Currency::USD),
            loss_description: "Test claim for unit testing".to_string(),
        }
    }

    /// Sets the claim ID
    pub fn with_claim_id(mut self, id: ClaimId) -> Self {
        self.claim_id = id;
        self
    }

    /// Sets the claim number
    pub fn with_claim_number(mut self, number: impl Into<String>) -> Self {
        self.claim_number = number.into();
        self
    }

    /// Sets the policy ID
    pub fn with_policy_id(mut self, id: PolicyId) -> Self {
        self.policy_id = id;
        self
    }

    /// Sets the claimant ID
    pub fn with_claimant_id(mut self, id: PartyId) -> Self {
        self.claimant_id = id;
        self
    }

    /// Sets the loss date
    pub fn with_loss_date(mut self, date: NaiveDate) -> Self {
        self.loss_date = date;
        self
    }

    /// Sets the claimed amount
    pub fn with_claimed_amount(mut self, amount: Money) -> Self {
        self.claimed_amount = amount;
        self
    }

    /// Sets the loss description
    pub fn with_loss_description(mut self, desc: impl Into<String>) -> Self {
        self.loss_description = desc.into();
        self
    }

    /// Builds the test claim data
    pub fn build(self) -> TestClaimData {
        TestClaimData {
            claim_id: self.claim_id,
            claim_number: self.claim_number,
            policy_id: self.policy_id,
            claimant_id: self.claimant_id,
            loss_date: self.loss_date,
            claimed_amount: self.claimed_amount,
            loss_description: self.loss_description,
        }
    }
}

/// Test claim data structure
#[derive(Debug, Clone)]
pub struct TestClaimData {
    pub claim_id: ClaimId,
    pub claim_number: String,
    pub policy_id: PolicyId,
    pub claimant_id: PartyId,
    pub loss_date: NaiveDate,
    pub claimed_amount: Money,
    pub loss_description: String,
}

/// Builder for constructing test party data
pub struct TestPartyDataBuilder {
    party_id: PartyId,
    first_name: String,
    last_name: String,
    email: String,
    phone: String,
    date_of_birth: NaiveDate,
}

impl Default for TestPartyDataBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TestPartyDataBuilder {
    /// Creates a new builder with default values
    pub fn new() -> Self {
        Self {
            party_id: PartyId::new(),
            first_name: StringFixtures::first_name().to_string(),
            last_name: StringFixtures::last_name().to_string(),
            email: StringFixtures::email().to_string(),
            phone: StringFixtures::phone().to_string(),
            date_of_birth: TemporalFixtures::date_of_birth_35(),
        }
    }

    /// Sets the party ID
    pub fn with_party_id(mut self, id: PartyId) -> Self {
        self.party_id = id;
        self
    }

    /// Sets the first name
    pub fn with_first_name(mut self, name: impl Into<String>) -> Self {
        self.first_name = name.into();
        self
    }

    /// Sets the last name
    pub fn with_last_name(mut self, name: impl Into<String>) -> Self {
        self.last_name = name.into();
        self
    }

    /// Sets the email
    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = email.into();
        self
    }

    /// Sets the phone
    pub fn with_phone(mut self, phone: impl Into<String>) -> Self {
        self.phone = phone.into();
        self
    }

    /// Sets the date of birth
    pub fn with_date_of_birth(mut self, dob: NaiveDate) -> Self {
        self.date_of_birth = dob;
        self
    }

    /// Sets the age (calculates DOB from current date)
    pub fn with_age(mut self, age: u32) -> Self {
        let today = Utc::now().date_naive();
        self.date_of_birth = today - chrono::Duration::days(age as i64 * 365);
        self
    }

    /// Builds the test party data
    pub fn build(self) -> TestPartyData {
        TestPartyData {
            party_id: self.party_id,
            first_name: self.first_name,
            last_name: self.last_name,
            email: self.email,
            phone: self.phone,
            date_of_birth: self.date_of_birth,
        }
    }
}

/// Test party data structure
#[derive(Debug, Clone)]
pub struct TestPartyData {
    pub party_id: PartyId,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: String,
    pub date_of_birth: NaiveDate,
}

/// Builder for constructing test account data (for billing)
pub struct TestAccountDataBuilder {
    account_id: AccountId,
    account_code: String,
    account_name: String,
    account_type: String,
    currency: Currency,
}

impl Default for TestAccountDataBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TestAccountDataBuilder {
    /// Creates a new builder with default values
    pub fn new() -> Self {
        Self {
            account_id: AccountId::new(),
            account_code: StringFixtures::account_code().to_string(),
            account_name: "Cash Account".to_string(),
            account_type: "asset".to_string(),
            currency: Currency::USD,
        }
    }

    /// Sets the account ID
    pub fn with_account_id(mut self, id: AccountId) -> Self {
        self.account_id = id;
        self
    }

    /// Sets the account code
    pub fn with_account_code(mut self, code: impl Into<String>) -> Self {
        self.account_code = code.into();
        self
    }

    /// Sets the account name
    pub fn with_account_name(mut self, name: impl Into<String>) -> Self {
        self.account_name = name.into();
        self
    }

    /// Sets the account type
    pub fn with_account_type(mut self, account_type: impl Into<String>) -> Self {
        self.account_type = account_type.into();
        self
    }

    /// Sets the currency
    pub fn with_currency(mut self, currency: Currency) -> Self {
        self.currency = currency;
        self
    }

    /// Builds an asset account
    pub fn asset() -> Self {
        Self::new().with_account_type("asset")
    }

    /// Builds a liability account
    pub fn liability() -> Self {
        Self::new()
            .with_account_code("2000-LIAB")
            .with_account_name("Liability Account")
            .with_account_type("liability")
    }

    /// Builds a revenue account
    pub fn revenue() -> Self {
        Self::new()
            .with_account_code("4000-REV")
            .with_account_name("Premium Revenue")
            .with_account_type("revenue")
    }

    /// Builds an expense account
    pub fn expense() -> Self {
        Self::new()
            .with_account_code("5000-EXP")
            .with_account_name("Claims Expense")
            .with_account_type("expense")
    }

    /// Builds the test account data
    pub fn build(self) -> TestAccountData {
        TestAccountData {
            account_id: self.account_id,
            account_code: self.account_code,
            account_name: self.account_name,
            account_type: self.account_type,
            currency: self.currency,
        }
    }
}

/// Test account data structure
#[derive(Debug, Clone)]
pub struct TestAccountData {
    pub account_id: AccountId,
    pub account_code: String,
    pub account_name: String,
    pub account_type: String,
    pub currency: Currency,
}

/// Builder for constructing test fund data
pub struct TestFundDataBuilder {
    fund_id: FundId,
    fund_code: String,
    fund_name: String,
    fund_type: String,
    nav_value: Decimal,
    currency: Currency,
}

impl Default for TestFundDataBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TestFundDataBuilder {
    /// Creates a new builder with default values
    pub fn new() -> Self {
        Self {
            fund_id: FundId::new(),
            fund_code: StringFixtures::fund_code().to_string(),
            fund_name: "Growth Equity Fund".to_string(),
            fund_type: "equity".to_string(),
            nav_value: dec!(15.4532),
            currency: Currency::USD,
        }
    }

    /// Sets the fund ID
    pub fn with_fund_id(mut self, id: FundId) -> Self {
        self.fund_id = id;
        self
    }

    /// Sets the fund code
    pub fn with_fund_code(mut self, code: impl Into<String>) -> Self {
        self.fund_code = code.into();
        self
    }

    /// Sets the fund name
    pub fn with_fund_name(mut self, name: impl Into<String>) -> Self {
        self.fund_name = name.into();
        self
    }

    /// Sets the fund type
    pub fn with_fund_type(mut self, fund_type: impl Into<String>) -> Self {
        self.fund_type = fund_type.into();
        self
    }

    /// Sets the NAV value
    pub fn with_nav_value(mut self, nav: Decimal) -> Self {
        self.nav_value = nav;
        self
    }

    /// Sets the currency
    pub fn with_currency(mut self, currency: Currency) -> Self {
        self.currency = currency;
        self
    }

    /// Builds an equity fund
    pub fn equity() -> Self {
        Self::new()
    }

    /// Builds a bond fund
    pub fn bond() -> Self {
        Self::new()
            .with_fund_code("BD-STABLE-01")
            .with_fund_name("Stable Bond Fund")
            .with_fund_type("bond")
            .with_nav_value(dec!(10.2500))
    }

    /// Builds a money market fund
    pub fn money_market() -> Self {
        Self::new()
            .with_fund_code("MM-LIQUID-01")
            .with_fund_name("Liquid Money Market Fund")
            .with_fund_type("money_market")
            .with_nav_value(dec!(1.0000))
    }

    /// Builds the test fund data
    pub fn build(self) -> TestFundData {
        TestFundData {
            fund_id: self.fund_id,
            fund_code: self.fund_code,
            fund_name: self.fund_name,
            fund_type: self.fund_type,
            nav_value: self.nav_value,
            currency: self.currency,
        }
    }
}

/// Test fund data structure
#[derive(Debug, Clone)]
pub struct TestFundData {
    pub fund_id: FundId,
    pub fund_code: String,
    pub fund_name: String,
    pub fund_type: String,
    pub nav_value: Decimal,
    pub currency: Currency,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_builder_defaults() {
        let policy = TestPolicyDataBuilder::new().build();
        assert_eq!(policy.product_code, "TERM_LIFE_20");
        assert!(policy.premium.amount() > Decimal::ZERO);
    }

    #[test]
    fn test_policy_builder_customization() {
        let policy = TestPolicyDataBuilder::new()
            .with_product_code("WHOLE_LIFE")
            .with_term_years(30)
            .build();

        assert_eq!(policy.product_code, "WHOLE_LIFE");
    }

    #[test]
    fn test_party_builder_age() {
        let party = TestPartyDataBuilder::new()
            .with_age(30)
            .build();

        let age_days = (Utc::now().date_naive() - party.date_of_birth).num_days();
        let age_years = age_days / 365;
        assert_eq!(age_years, 30);
    }

    #[test]
    fn test_account_builder_types() {
        let asset = TestAccountDataBuilder::asset().build();
        let liability = TestAccountDataBuilder::liability().build();
        let revenue = TestAccountDataBuilder::revenue().build();

        assert_eq!(asset.account_type, "asset");
        assert_eq!(liability.account_type, "liability");
        assert_eq!(revenue.account_type, "revenue");
    }
}
