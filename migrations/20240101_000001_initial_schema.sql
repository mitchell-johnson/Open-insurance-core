-- Initial schema setup for Open Insurance Core
-- Implements bi-temporal data model with PostgreSQL range types

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS btree_gist;
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Custom enum types
CREATE TYPE policy_status AS ENUM (
    'quoted', 'in_force', 'lapsed', 'terminated',
    'cancelled', 'expired', 'pending_underwriting'
);

CREATE TYPE party_type AS ENUM ('individual', 'corporate', 'agent', 'broker');
CREATE TYPE kyc_status AS ENUM ('pending', 'in_progress', 'verified', 'failed', 'expired');
CREATE TYPE claim_status AS ENUM (
    'fnol', 'under_investigation', 'pending_documentation',
    'under_review', 'approved', 'partially_approved', 'denied',
    'closed', 'withdrawn', 'reopened'
);
CREATE TYPE loss_type AS ENUM (
    'death', 'disability', 'critical_illness', 'hospitalization',
    'accident', 'property', 'liability', 'other'
);
CREATE TYPE reserve_type AS ENUM ('case_reserve', 'ibnr', 'legal_expense', 'expense');
CREATE TYPE payment_type AS ENUM ('indemnity', 'expense', 'partial', 'final_settlement');
CREATE TYPE payment_method AS ENUM ('bank_transfer', 'check', 'direct_deposit', 'wire');
CREATE TYPE account_type AS ENUM ('asset', 'liability', 'equity', 'revenue', 'expense');
CREATE TYPE posting_type AS ENUM ('debit', 'credit');
CREATE TYPE fund_type AS ENUM (
    'equity', 'bond', 'balanced', 'money_market', 'index', 'sector'
);
CREATE TYPE risk_level AS ENUM ('low', 'medium_low', 'medium', 'medium_high', 'high');
CREATE TYPE unit_transaction_type AS ENUM (
    'allocation', 'redemption', 'switch_in', 'switch_out',
    'mortality_charge', 'policy_fee', 'management_fee', 'bonus'
);

-- Party table (bi-temporal)
CREATE TABLE party_versions (
    version_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    party_id UUID NOT NULL,
    party_type party_type NOT NULL,
    first_name VARCHAR(100),
    last_name VARCHAR(100),
    company_name VARCHAR(200),
    email VARCHAR(255),
    phone VARCHAR(50),
    date_of_birth DATE,
    tax_id VARCHAR(50),
    kyc_status kyc_status NOT NULL DEFAULT 'pending',

    -- Bi-temporal columns
    valid_period tstzrange NOT NULL DEFAULT tstzrange(CURRENT_TIMESTAMP, NULL),
    sys_period tstzrange NOT NULL DEFAULT tstzrange(CURRENT_TIMESTAMP, NULL),

    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Prevent overlapping valid periods for active records
    EXCLUDE USING gist (
        party_id WITH =,
        valid_period WITH &&
    ) WHERE (upper(sys_period) IS NULL)
);

CREATE INDEX idx_party_versions_party_id ON party_versions(party_id);
CREATE INDEX idx_party_versions_email ON party_versions(lower(email));
CREATE INDEX idx_party_versions_current ON party_versions(party_id) WHERE upper(sys_period) IS NULL;

-- Policy table (bi-temporal)
CREATE TABLE policy_versions (
    version_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    policy_id UUID NOT NULL,
    policy_number VARCHAR(50) NOT NULL,
    product_code VARCHAR(50) NOT NULL,
    policyholder_id UUID NOT NULL REFERENCES party_versions(party_id),
    status policy_status NOT NULL DEFAULT 'quoted',
    effective_date TIMESTAMPTZ NOT NULL,
    expiry_date TIMESTAMPTZ NOT NULL,
    premium NUMERIC(20, 2) NOT NULL,
    sum_assured NUMERIC(20, 2) NOT NULL,
    currency VARCHAR(3) NOT NULL DEFAULT 'USD',

    -- Bi-temporal columns
    valid_period tstzrange NOT NULL,
    sys_period tstzrange NOT NULL DEFAULT tstzrange(CURRENT_TIMESTAMP, NULL),

    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,

    EXCLUDE USING gist (
        policy_id WITH =,
        valid_period WITH &&
    ) WHERE (upper(sys_period) IS NULL)
);

CREATE INDEX idx_policy_versions_policy_id ON policy_versions(policy_id);
CREATE INDEX idx_policy_versions_number ON policy_versions(policy_number);
CREATE INDEX idx_policy_versions_policyholder ON policy_versions(policyholder_id);
CREATE INDEX idx_policy_versions_current ON policy_versions(policy_id) WHERE upper(sys_period) IS NULL;
CREATE INDEX idx_policy_versions_expiring ON policy_versions(expiry_date) WHERE upper(sys_period) IS NULL;

-- Claims table
CREATE TABLE claims (
    claim_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    claim_number VARCHAR(50) NOT NULL UNIQUE,
    policy_id UUID NOT NULL,
    claimant_id UUID NOT NULL,
    status claim_status NOT NULL DEFAULT 'fnol',
    loss_date DATE NOT NULL,
    notification_date TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    loss_type loss_type NOT NULL,
    loss_description TEXT,
    loss_location VARCHAR(500),
    claimed_amount NUMERIC(20, 2),
    approved_amount NUMERIC(20, 2),
    paid_amount NUMERIC(20, 2) DEFAULT 0,
    currency VARCHAR(3) NOT NULL DEFAULT 'USD',
    assigned_to VARCHAR(100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_claims_policy ON claims(policy_id);
CREATE INDEX idx_claims_status ON claims(status);

-- Claim status history
CREATE TABLE claim_status_history (
    history_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    claim_id UUID NOT NULL REFERENCES claims(claim_id),
    status claim_status NOT NULL,
    reason TEXT,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Claim reserves
CREATE TABLE claim_reserves (
    reserve_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    claim_id UUID NOT NULL REFERENCES claims(claim_id),
    reserve_type reserve_type NOT NULL,
    amount NUMERIC(20, 2) NOT NULL,
    currency VARCHAR(3) NOT NULL DEFAULT 'USD',
    reason TEXT,
    created_by VARCHAR(100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Claim payments
CREATE TABLE claim_payments (
    payment_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    claim_id UUID NOT NULL REFERENCES claims(claim_id),
    payee_id UUID NOT NULL,
    amount NUMERIC(20, 2) NOT NULL,
    currency VARCHAR(3) NOT NULL DEFAULT 'USD',
    payment_type payment_type NOT NULL,
    payment_method payment_method NOT NULL,
    reference VARCHAR(100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Chart of accounts
CREATE TABLE accounts (
    account_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    account_code VARCHAR(20) NOT NULL UNIQUE,
    account_name VARCHAR(200) NOT NULL,
    account_type account_type NOT NULL,
    parent_id UUID REFERENCES accounts(account_id),
    description TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Journal entries
CREATE TABLE journal_entries (
    entry_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    entry_date TIMESTAMPTZ NOT NULL,
    description TEXT NOT NULL,
    reference_type VARCHAR(50),
    reference_id UUID,
    created_by VARCHAR(100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Postings
CREATE TABLE postings (
    posting_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    entry_id UUID NOT NULL REFERENCES journal_entries(entry_id),
    account_id UUID NOT NULL REFERENCES accounts(account_id),
    amount NUMERIC(20, 4) NOT NULL,
    currency VARCHAR(3) NOT NULL DEFAULT 'USD',
    posting_type posting_type NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_postings_entry ON postings(entry_id);
CREATE INDEX idx_postings_account ON postings(account_id);

-- Funds
CREATE TABLE funds (
    fund_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    fund_code VARCHAR(20) NOT NULL UNIQUE,
    fund_name VARCHAR(200) NOT NULL,
    fund_type fund_type NOT NULL,
    currency VARCHAR(3) NOT NULL DEFAULT 'USD',
    risk_level risk_level NOT NULL,
    management_fee NUMERIC(10, 6) NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Fund NAVs
CREATE TABLE fund_navs (
    nav_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    fund_id UUID NOT NULL REFERENCES funds(fund_id),
    nav_date DATE NOT NULL,
    nav_value NUMERIC(20, 6) NOT NULL,
    currency VARCHAR(3) NOT NULL DEFAULT 'USD',
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(fund_id, nav_date)
);

CREATE INDEX idx_fund_navs_fund_date ON fund_navs(fund_id, nav_date DESC);

-- Unit holdings
CREATE TABLE unit_holdings (
    holding_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    policy_id UUID NOT NULL,
    fund_id UUID NOT NULL REFERENCES funds(fund_id),
    units NUMERIC(20, 6) NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(policy_id, fund_id)
);

-- Unit transactions
CREATE TABLE unit_transactions (
    transaction_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    policy_id UUID NOT NULL,
    fund_id UUID NOT NULL REFERENCES funds(fund_id),
    units NUMERIC(20, 6) NOT NULL,
    transaction_type unit_transaction_type NOT NULL,
    transaction_date TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_unit_transactions_policy ON unit_transactions(policy_id);

-- Audit log
CREATE TABLE audit_log (
    audit_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id VARCHAR(100) NOT NULL,
    action VARCHAR(50) NOT NULL,
    entity_type VARCHAR(50) NOT NULL,
    entity_id UUID,
    old_values JSONB,
    new_values JSONB,
    ip_address INET,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_audit_log_entity ON audit_log(entity_type, entity_id);
CREATE INDEX idx_audit_log_user ON audit_log(user_id);
CREATE INDEX idx_audit_log_created ON audit_log(created_at);
