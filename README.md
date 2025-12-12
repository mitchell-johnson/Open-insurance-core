# Open Insurance Core

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![CI](https://github.com/Open-insurance-core/Open-insurance-core/actions/workflows/rust.yml/badge.svg)](https://github.com/Open-insurance-core/Open-insurance-core/actions)

A next-generation, open-source insurance core system built in Rust. Designed for mathematical correctness, regulatory compliance, and high-performance processing of life, health, and investment insurance products.

## Table of Contents

- [Overview](#overview)
- [Key Features](#key-features)
- [Architecture](#architecture)
- [Technology Stack](#technology-stack)
- [Getting Started](#getting-started)
- [Project Structure](#project-structure)
- [Domain Modules](#domain-modules)
- [API Reference](#api-reference)
- [Database Schema](#database-schema)
- [Testing](#testing)
- [Docker Development](#docker-development)
- [Configuration](#configuration)
- [Product Configuration](#product-configuration)
- [Contributing](#contributing)
- [License](#license)

---

## Overview

Open Insurance Core is a **modular monolith** insurance administration system engineered for:

- **Mathematical Precision**: All financial calculations use `rust_decimal` (128-bit fixed-point) - zero floating-point errors
- **Regulatory Compliance**: Bi-temporal data model enables full audit trails and "time travel" queries
- **Type-Safe Domain Logic**: Rust's type system prevents invalid policy states at compile time
- **High Performance**: Async runtime with deterministic latency - no garbage collection pauses
- **Extensibility**: Hexagonal architecture with swappable adapters for external integrations

### Why Rust?

Insurance core systems are long-lived (20+ year) assets where defects carry catastrophic costs. Rust provides:

| Challenge | Rust Solution |
|-----------|---------------|
| Memory corruption bugs | Ownership system eliminates at compile time |
| Concurrency issues | Borrow checker prevents data races |
| Runtime crashes | `Option`/`Result` types force error handling |
| Invalid business states | Algebraic types encode valid states only |
| Performance variability | No GC = predictable latency |

---

## Key Features

### Insurance Products

| Product Type | Capabilities |
|--------------|--------------|
| **Term Life** | Time-limited death benefit, renewable terms, conversion options |
| **Whole Life** | Permanent coverage, cash value accumulation, surrender values |
| **Critical Illness** | Lump-sum benefits on diagnosis of covered conditions |
| **ULIP** | Unit-linked plans with investment flexibility, fund switching |

### Policy Administration

- **Full Lifecycle Management**: Quote → Underwrite → Issue → Endorse → Renew → Terminate
- **Retroactive Endorsements**: Bi-temporal model supports backdated corrections
- **State Machine Enforcement**: Type system prevents invalid state transitions
- **Multi-Coverage Policies**: Multiple benefits per policy with independent terms

### Party Management

- **Complex Ownership**: Joint owners, trusts, partnerships with ownership percentages
- **Role-Based Relationships**: Primary owner, co-owner, trustee, beneficiary, managing partner
- **KYC Workflows**: Document collection, verification status tracking
- **Corporate Entities**: Companies, partnerships, LLPs with tax IDs

### Claims Processing

- **FNOL to Settlement**: Complete claims workflow with adjudication
- **Loss Reserving**: Case reserves, IBNR, legal expense reserves
- **Multiple Loss Types**: Death, disability, critical illness, hospitalization, property
- **Claim Payments**: Partial settlements, indemnity, expense payments

### Fund Management (ULIP)

- **Multiple Fund Types**: Equity, bond, balanced, money market, index, sector
- **Daily NAV Tracking**: Historical NAV with time-series queries
- **Unit Allocation**: Premium → charges → fund allocation with 6-decimal precision
- **Fund Switching**: Transfer units between funds with transaction history

### Financial Integrity

- **Double-Entry Bookkeeping**: Every transaction balanced (debits = credits)
- **Chart of Accounts**: Assets, liabilities, equity, revenue, expense
- **Immutable Ledger**: Append-only journal entries with audit trail
- **Multi-Currency**: 10 currencies supported (USD, EUR, GBP, JPY, CHF, INR, AUD, CAD, SGD, HKD)

### Dynamic Product Configuration

- **Rules Engine**: Zen-engine with JSON Decision Model (JDM) format
- **Hot Deployment**: New products without recompilation
- **Business-Friendly**: Excel-like decision tables for underwriting rules
- **Pricing Factors**: Configurable rate tables and limits

---

## Architecture

### Modular Monolith

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Interface Layer                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                        interface_api (Axum)                             ││
│  │  /api/v1/policies  /api/v1/claims  /api/v1/parties  /api/v1/funds      ││
│  └─────────────────────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────────────────────┤
│                              Domain Layer                                    │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐       │
│  │domain_policy │ │domain_claims │ │domain_billing│ │ domain_fund  │       │
│  │              │ │              │ │              │ │              │       │
│  │ - Aggregate  │ │ - FNOL       │ │ - Ledger     │ │ - NAV        │       │
│  │ - Coverage   │ │ - Reserves   │ │ - Accounts   │ │ - Units      │       │
│  │ - Premium    │ │ - Adjudicate │ │ - Invoices   │ │ - Allocation │       │
│  │ - Rules     │ │ - Payments   │ │ - Payments   │ │ - Switching  │       │
│  └──────────────┘ └──────────────┘ └──────────────┘ └──────────────┘       │
│                         ┌──────────────┐                                    │
│                         │ domain_party │                                    │
│                         │              │                                    │
│                         │ - Individual │                                    │
│                         │ - Corporate  │                                    │
│                         │ - Joint/Trust│                                    │
│                         │ - KYC        │                                    │
│                         └──────────────┘                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                              Core Layer                                      │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                          core_kernel                                    ││
│  │  Money | Temporal | Identifiers | Ports & Adapters | Registry          ││
│  └─────────────────────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────────────────────┤
│                           Infrastructure Layer                               │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                            infra_db                                     ││
│  │  PostgreSQL | SQLx | Bi-temporal Queries | Repositories                 ││
│  └─────────────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────────────┘
```

### Hexagonal Architecture (Ports & Adapters)

The system implements hexagonal architecture for maximum flexibility:

```
                    ┌─────────────────────────────────────┐
                    │         Domain Core (Pure)          │
                    │                                     │
     ┌──────────────┤  Business Logic  |  Domain Events  ├──────────────┐
     │              │                                     │              │
     │              └─────────────────────────────────────┘              │
     │                              │                                    │
     ▼                              ▼                                    ▼
┌─────────┐                  ┌─────────────┐                     ┌─────────────┐
│  Ports  │                  │    Ports    │                     │    Ports    │
│ (Trait) │                  │   (Trait)   │                     │   (Trait)   │
└────┬────┘                  └──────┬──────┘                     └──────┬──────┘
     │                              │                                    │
     ▼                              ▼                                    ▼
┌─────────────┐             ┌─────────────────┐              ┌─────────────────┐
│  Postgres   │             │   External CRM  │              │   Mock Adapter  │
│   Adapter   │             │     Adapter     │              │    (Testing)    │
└─────────────┘             └─────────────────┘              └─────────────────┘
```

### Bi-Temporal Data Model

Every record tracks two time dimensions:

| Dimension | Purpose | Example Query |
|-----------|---------|---------------|
| **Valid Time** | When the fact is true in business reality | "What was the premium on March 15?" |
| **System Time** | When the fact was recorded in the database | "What did we think the premium was before the correction?" |

```
Timeline Example: Premium Correction

Original Entry (Dec 15):
  Valid: Jan 1 - Dec 31, 2024  |  System: Dec 15, 2023 - ∞
  Premium: $1,000

After Correction (Feb 1):
  Old Row: Valid: Jan 1 - Dec 31  |  System: Dec 15 - Feb 1  |  $1,000 (closed)
  New Row: Valid: Jan 1 - Dec 31  |  System: Feb 1 - ∞      |  $1,200 (current)

Both versions preserved for audit compliance.
```

---

## Technology Stack

### Core Runtime

| Component | Technology | Purpose |
|-----------|------------|---------|
| Language | Rust 2021 Edition | Memory safety, performance |
| Async Runtime | Tokio 1.41 | High-concurrency async I/O |
| Financial Math | rust_decimal 1.36 | 128-bit fixed-point precision |
| Date/Time | chrono 0.4 + chrono-tz | Timezone-aware temporal handling |
| Identifiers | uuid 1.11 | UUID v4/v7 for all entities |

### Web & API

| Component | Technology | Purpose |
|-----------|------------|---------|
| Framework | Axum 0.7 | Type-safe, high-performance HTTP |
| Middleware | Tower 0.5 | Composable service layers |
| HTTP Utils | tower-http 0.6 | Tracing, CORS, request IDs |
| Auth | jsonwebtoken 9.3 | JWT validation |
| Validation | validator 0.18 | Input validation with derives |

### Database

| Component | Technology | Purpose |
|-----------|------------|---------|
| Database | PostgreSQL 16+ | Range types, GiST indexes |
| Driver | SQLx 0.8 | Compile-time query verification |
| Extensions | btree_gist, uuid-ossp | Temporal integrity, UUID generation |

### Domain-Specific

| Component | Technology | Purpose |
|-----------|------------|---------|
| Rules Engine | zen-engine 0.25 | JSON Decision Model evaluation |
| i18n | fluent 0.16 | Localization for documents |
| Error Handling | thiserror 2.0 | Derive-based error types |
| Logging | tracing 0.1 | Structured, async-aware logging |

### Testing

| Component | Technology | Purpose |
|-----------|------------|---------|
| Property Testing | proptest 1.5 | Mathematical invariant verification |
| Fake Data | fake 3.0 | Realistic test data generation |
| Containers | testcontainers 0.23 | PostgreSQL integration tests |
| HTTP Testing | axum-test 16 | API endpoint testing |

---

## Getting Started

### Prerequisites

- **Rust**: 1.75 or later ([install](https://rustup.rs/))
- **PostgreSQL**: 16+ with extensions
- **Docker** (optional): For containerized development

### Installation

```bash
# Clone the repository
git clone https://github.com/Open-insurance-core/Open-insurance-core.git
cd Open-insurance-core

# Build all crates
cargo build

# Run tests
cargo test
```

### Database Setup

```bash
# Start PostgreSQL (Docker)
docker-compose up -d postgres

# Or use existing PostgreSQL and create database
createdb insurance_dev

# Run migrations
sqlx migrate run
```

### Running the API Server

```bash
# Set environment variables
export DATABASE_URL="postgres://insurance_user:insurance_pass@localhost:5432/insurance_dev"
export RUST_LOG=info

# Run the server
cargo run --bin insurance-api
```

The API will be available at `http://localhost:8080`.

### Quick Verification

```bash
# Health check
curl http://localhost:8080/health

# Expected response
{"status": "healthy"}
```

---

## Project Structure

```
Open-insurance-core/
├── crates/                          # Cargo workspace members
│   ├── core_kernel/                 # Foundational types
│   │   ├── src/
│   │   │   ├── lib.rs              # Module exports
│   │   │   ├── money.rs            # Currency & Money types
│   │   │   ├── temporal.rs         # Bi-temporal primitives
│   │   │   ├── identifiers.rs      # Strongly-typed IDs
│   │   │   ├── ports.rs            # Hexagonal port traits
│   │   │   ├── registry.rs         # Adapter registry
│   │   │   └── error.rs            # Core errors
│   │   └── Cargo.toml
│   │
│   ├── domain_policy/               # Policy administration
│   │   ├── src/
│   │   │   ├── aggregate.rs        # Policy aggregate root
│   │   │   ├── coverage.rs         # Coverage definitions
│   │   │   ├── premium.rs          # Premium calculation
│   │   │   ├── endorsement.rs      # Policy modifications
│   │   │   ├── rules_engine.rs     # Product rules integration
│   │   │   ├── underwriting.rs     # Underwriting logic
│   │   │   ├── services.rs         # Domain services
│   │   │   └── events.rs           # Domain events
│   │   └── Cargo.toml
│   │
│   ├── domain_party/                # Party/customer management
│   │   ├── src/
│   │   │   ├── party.rs            # Party composition model
│   │   │   ├── address.rs          # Address management
│   │   │   ├── kyc.rs              # KYC workflows
│   │   │   ├── agent.rs            # Agent/broker management
│   │   │   ├── validation.rs       # Party validation rules
│   │   │   ├── ports.rs            # Party ports
│   │   │   └── adapters/           # Port implementations
│   │   └── Cargo.toml
│   │
│   ├── domain_billing/              # Financial operations
│   │   ├── src/
│   │   │   ├── ledger.rs           # Double-entry engine
│   │   │   ├── account.rs          # Chart of accounts
│   │   │   ├── transaction.rs      # Journal entries
│   │   │   ├── invoice.rs          # Invoicing
│   │   │   └── payment.rs          # Payment processing
│   │   └── Cargo.toml
│   │
│   ├── domain_claims/               # Claims management
│   │   ├── src/
│   │   │   ├── claim.rs            # Claim aggregate
│   │   │   ├── reserve.rs          # Loss reserving
│   │   │   ├── payment.rs          # Claim payments
│   │   │   ├── adjudication.rs     # Claims decisions
│   │   │   └── workflow.rs         # Claims state machine
│   │   └── Cargo.toml
│   │
│   ├── domain_fund/                 # ULIP fund management
│   │   ├── src/
│   │   │   ├── fund.rs             # Fund definitions
│   │   │   ├── nav.rs              # NAV tracking
│   │   │   ├── unit_holding.rs     # Unit balances
│   │   │   ├── unit_transaction.rs # Unit movements
│   │   │   └── allocation.rs       # Premium allocation
│   │   └── Cargo.toml
│   │
│   ├── infra_db/                    # Database infrastructure
│   │   ├── src/
│   │   │   ├── pool.rs             # Connection management
│   │   │   ├── bitemporal.rs       # Temporal queries
│   │   │   ├── repositories/       # Data access objects
│   │   │   └── adapters/           # Port implementations
│   │   └── Cargo.toml
│   │
│   ├── interface_api/               # HTTP API
│   │   ├── src/
│   │   │   ├── lib.rs              # Router factory
│   │   │   ├── handlers/           # Request handlers
│   │   │   ├── dto/                # Data transfer objects
│   │   │   ├── middleware.rs       # Auth, audit middleware
│   │   │   ├── auth.rs             # JWT authentication
│   │   │   └── error.rs            # Error responses
│   │   └── Cargo.toml
│   │
│   └── test_utils/                  # Test infrastructure
│       ├── src/
│       │   ├── fixtures.rs         # Pre-built test data
│       │   ├── builders.rs         # Fluent test builders
│       │   ├── database.rs         # Test DB utilities
│       │   └── generators.rs       # Proptest generators
│       └── Cargo.toml
│
├── migrations/                      # SQL migrations
│   ├── 20240101_000001_initial_schema.sql
│   └── 20240102_000001_party_composition.sql
│
├── products/                        # Product rule definitions
│   ├── catalog.json                # Product catalog
│   ├── term_life.json              # Term life rules
│   ├── whole_life.json             # Whole life rules
│   └── critical_illness.json       # Critical illness rules
│
├── scripts/                         # Operational scripts
├── .github/workflows/               # CI/CD pipelines
├── Cargo.toml                       # Workspace manifest
├── docker-compose.yml               # Development containers
├── Dockerfile                       # Production build
└── Plan.md                          # Architecture blueprint
```

---

## Domain Modules

### core_kernel

The foundational crate providing building blocks for all domain modules.

#### Money Type

Type-safe monetary values with currency enforcement:

```rust
use core_kernel::{Money, Currency};
use rust_decimal_macros::dec;

// Create money values
let premium = Money::new(dec!(1000.00), Currency::USD);
let fee = Money::new(dec!(50.00), Currency::USD);

// Safe arithmetic (same currency only)
let total = premium.add(&fee)?;  // Ok(Money { amount: 1050.00, currency: USD })

// Currency mismatch prevented at runtime
let eur = Money::new(dec!(100.00), Currency::EUR);
let result = premium.add(&eur);  // Err(CurrencyMismatch)
```

#### Bi-Temporal Records

```rust
use core_kernel::{ValidPeriod, SystemPeriod, BiTemporalRecord};

let record = BiTemporalRecord {
    valid_period: ValidPeriod::new(start_date, end_date),
    sys_period: SystemPeriod::current(),  // From now until superseded
    data: policy_version,
};
```

#### Strongly-Typed Identifiers

```rust
use core_kernel::{PolicyId, ClaimId, PartyId};

let policy_id = PolicyId::new();  // UUID v4
let claim_id = ClaimId::new();

// Type system prevents mixing IDs
fn get_policy(id: PolicyId) -> Policy;  // Only accepts PolicyId
```

### domain_policy

Policy administration with state machine enforcement.

#### Policy States

```rust
pub enum PolicyState {
    Quoted,
    PendingUnderwriting,
    InForce { start_date: Date, renewal_date: Date },
    Lapsed { reason: LapseReason, effective_date: Date },
    Reinstated { reinstated_date: Date },
    Terminated { reason: TerminationReason, effective_date: Date },
    Cancelled { reason: CancellationReason, effective_date: Date },
    Expired { expiry_date: Date },
}
```

#### Policy Operations

```rust
// Create a quote
let quote = Policy::create_quote(product_id, coverages, insured_party)?;

// Underwriting
let underwritten = quote.submit_for_underwriting(application)?;

// Issue policy
let policy = underwritten.issue(effective_date)?;

// Endorsement (modification)
let endorsed = policy.apply_endorsement(endorsement)?;

// Invalid transitions fail at compile time or return Err
```

### domain_party

Complex party composition supporting joint ownership and trusts.

#### Party Types

```rust
pub enum PartyComposition {
    Individual(Individual),
    Corporate(Corporate),
    Joint(JointDetails),
    Trust(TrustDetails),
    Partnership(PartnershipDetails),
}

// Joint ownership with percentages
let joint = PartyComposition::Joint(JointDetails {
    joint_type: JointType::JointTenants,
    members: vec![
        PartyMember { party_id, role: MemberRole::PrimaryOwner, ownership_pct: dec!(50) },
        PartyMember { party_id, role: MemberRole::CoOwner, ownership_pct: dec!(50) },
    ],
});
```

### domain_billing

Double-entry bookkeeping for financial integrity.

#### Ledger Operations

```rust
// Every transaction must balance
let entry = JournalEntry::new("Premium Receipt - Policy P-101");

let postings = vec![
    Posting::debit(accounts::CASH, dec!(1000.00)),
    Posting::credit(accounts::PREMIUM_REVENUE, dec!(1000.00)),
];

// Validates balance before commit
ledger.post_transaction(entry, postings).await?;

// Unbalanced transactions rejected
let bad_postings = vec![
    Posting::debit(accounts::CASH, dec!(1000.00)),
    Posting::credit(accounts::PREMIUM_REVENUE, dec!(999.00)),  // Won't balance!
];
ledger.post_transaction(entry, bad_postings).await;  // Err(UnbalancedTransaction)
```

### domain_fund

Unit-linked product fund management.

#### Unit Precision

```rust
const UNIT_PRECISION: u32 = 6;  // 6 decimal places for units
const CURRENCY_PRECISION: u32 = 2;  // 2 decimal places for money

pub fn calculate_units(amount: Decimal, nav: Decimal) -> Decimal {
    (amount / nav).round_dp(UNIT_PRECISION)
}

// Example:
// Premium: $1,000, Charges: 2% ($20), Investable: $980
// NAV: $15.45
// Units: 980 / 15.45 = 63.430421 units
```

### domain_claims

Claims lifecycle from FNOL to settlement.

#### Claim Workflow

```rust
pub enum ClaimStatus {
    FNOL,                    // First Notice of Loss
    UnderInvestigation,
    PendingDocumentation,
    UnderReview,
    Approved,
    PartiallyApproved,
    Denied,
    Closed,
    Withdrawn,
    Reopened,
}

// State machine transitions
let claim = Claim::fnol(policy_id, loss_type, loss_date)?;
let claim = claim.begin_investigation(investigator_id)?;
let claim = claim.approve(adjudicator_id, approved_amount)?;
let claim = claim.close(settlement_date)?;
```

---

## API Reference

### Base URL

```
http://localhost:8080/api/v1
```

### Authentication

JWT Bearer token required for protected endpoints:

```bash
curl -H "Authorization: Bearer <token>" http://localhost:8080/api/v1/policies
```

### Endpoints

#### Health Check

```http
GET /health
```

Response:
```json
{
  "status": "healthy",
  "version": "0.1.0"
}
```

#### Policies

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/policies` | List policies |
| `POST` | `/policies` | Create quote |
| `GET` | `/policies/{id}` | Get policy details |
| `POST` | `/policies/{id}/issue` | Issue policy |
| `POST` | `/policies/{id}/endorsements` | Create endorsement |
| `POST` | `/policies/{id}/lapse` | Lapse policy |
| `POST` | `/policies/{id}/reinstate` | Reinstate policy |

#### Claims

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/claims` | List claims |
| `POST` | `/claims` | Submit FNOL |
| `GET` | `/claims/{id}` | Get claim details |
| `PUT` | `/claims/{id}/status` | Update status |
| `POST` | `/claims/{id}/reserves` | Add reserve |
| `POST` | `/claims/{id}/payments` | Record payment |

#### Parties

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/parties` | List parties |
| `POST` | `/parties` | Create party |
| `GET` | `/parties/{id}` | Get party details |
| `PUT` | `/parties/{id}` | Update party |
| `POST` | `/parties/{id}/kyc` | Submit KYC |

#### Funds

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/funds` | List funds |
| `GET` | `/funds/{id}` | Get fund details |
| `GET` | `/funds/{id}/nav` | Get current NAV |
| `GET` | `/funds/{id}/nav/history` | NAV history |

---

## Database Schema

### Bi-Temporal Tables

All versioned entities use this pattern:

```sql
CREATE TABLE policy_versions (
    version_id UUID PRIMARY KEY,
    policy_id UUID NOT NULL,

    -- Business data
    product_id UUID NOT NULL,
    status policy_status NOT NULL,
    benefit_amount NUMERIC(20, 2) NOT NULL,

    -- Bi-temporal dimensions
    valid_period tstzrange NOT NULL,    -- When true in reality
    sys_period tstzrange NOT NULL,      -- When recorded in system

    -- Prevent overlapping valid periods for current records
    EXCLUDE USING gist (
        policy_id WITH =,
        valid_period WITH &&
    ) WHERE (upper(sys_period) IS NULL)
);
```

### Key Enumerations

```sql
-- Policy lifecycle states
CREATE TYPE policy_status AS ENUM (
    'quoted', 'pending_underwriting', 'in_force',
    'lapsed', 'reinstated', 'terminated', 'cancelled', 'expired'
);

-- Claim workflow states
CREATE TYPE claim_status AS ENUM (
    'fnol', 'under_investigation', 'pending_documentation',
    'under_review', 'approved', 'partially_approved',
    'denied', 'closed', 'withdrawn', 'reopened'
);

-- Account types for double-entry
CREATE TYPE account_type AS ENUM (
    'asset', 'liability', 'equity', 'revenue', 'expense'
);
```

### Required PostgreSQL Extensions

```sql
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";   -- UUID generation
CREATE EXTENSION IF NOT EXISTS "btree_gist";  -- Range type indexing
```

---

## Testing

### Running Tests

```bash
# All tests
cargo test

# Specific crate
cargo test -p domain_policy

# With logging
RUST_LOG=debug cargo test -- --nocapture

# Integration tests only
cargo test --test '*'
```

### Property-Based Testing

Financial invariants verified with proptest:

```rust
proptest! {
    #[test]
    fn premium_allocation_invariant(
        premium in 1000..1000000u64,
        allocation_rate in 0..100u64
    ) {
        let p = Decimal::new(premium as i64, 2);
        let (invested, charges) = allocate(p, allocation_rate);

        // Invariant: invested + charges = premium
        prop_assert_eq!(invested + charges, p);
    }

    #[test]
    fn ledger_always_balanced(
        entries in vec(any::<JournalEntry>(), 1..100)
    ) {
        let ledger = Ledger::new();
        for entry in entries {
            ledger.post(entry)?;
        }

        // Invariant: total debits = total credits
        prop_assert_eq!(ledger.total_debits(), ledger.total_credits());
    }
}
```

### Integration Tests with Testcontainers

```rust
#[tokio::test]
async fn test_policy_creation() {
    // Automatically starts PostgreSQL container
    let db = TestDatabase::new().await;

    let pool = db.pool();
    let repo = PolicyRepository::new(pool);

    let policy = create_test_policy();
    repo.save(&policy).await.unwrap();

    let loaded = repo.find_by_id(policy.id()).await.unwrap();
    assert_eq!(loaded.id(), policy.id());

    // Container automatically cleaned up
}
```

---

## Docker Development

### Quick Start

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f

# Run tests
docker-compose run --rm test

# Cleanup
docker-compose down -v
```

### Available Services

| Service | Port | Description |
|---------|------|-------------|
| `postgres` | 5432 | PostgreSQL database |
| `test` | - | Run all tests |
| `unit-tests` | - | Unit tests only |
| `integration-tests` | - | Integration tests with DB |
| `dev` | 8080, 5432 | Development server |
| `dev-watch` | 8080, 5432 | Dev with hot reload |

### Running Specific Test Types

```bash
# Unit tests (no DB required)
docker-compose run --rm unit-tests

# Integration tests
docker-compose run --rm integration-tests

# All tests
docker-compose run --rm test all
```

### Development Server

```bash
# Start dev server with hot reload
docker-compose up dev-watch

# Access API
curl http://localhost:8080/health
```

---

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | - | PostgreSQL connection string |
| `API_HOST` | `127.0.0.1` | API bind address |
| `API_PORT` | `8080` | API port |
| `API_JWT_SECRET` | - | JWT signing secret |
| `RUST_LOG` | `info` | Log level (trace, debug, info, warn, error) |
| `RUST_BACKTRACE` | `0` | Enable backtraces (0, 1, full) |

### Database URL Format

```
postgres://user:password@host:port/database
```

### Example `.env`

```env
DATABASE_URL=postgres://insurance_user:insurance_pass@localhost:5432/insurance_dev
API_HOST=0.0.0.0
API_PORT=8080
API_JWT_SECRET=your-secret-key-here
RUST_LOG=info
```

---

## Product Configuration

Products are defined in JSON files using the JSON Decision Model (JDM) format.

### Product Catalog

`products/catalog.json` defines available products:

```json
{
  "products": [
    {
      "id": "term-life-20",
      "name": "20-Year Term Life",
      "type": "term_life",
      "rules_file": "term_life.json"
    }
  ]
}
```

### Product Rules

`products/term_life.json` defines underwriting and pricing rules:

```json
{
  "rules": {
    "eligibility": {
      "min_age": 18,
      "max_age": 65,
      "min_sum_assured": 50000,
      "max_sum_assured": 5000000
    },
    "pricing": {
      "base_rate_per_1000": 1.25,
      "age_factors": {
        "18-30": 0.8,
        "31-40": 1.0,
        "41-50": 1.5,
        "51-65": 2.5
      },
      "smoker_loading": 1.5
    }
  }
}
```

### Adding New Products

1. Create product rules JSON in `products/`
2. Add entry to `products/catalog.json`
3. Restart API server (hot reload supported)

No code changes or recompilation required.

---

## Contributing

We welcome contributions! Please follow these guidelines:

### Development Workflow

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make changes with tests
4. Run the full test suite: `cargo test`
5. Run clippy: `cargo clippy -- -D warnings`
6. Format code: `cargo fmt`
7. Commit with conventional commits: `feat: add new feature`
8. Push and create a Pull Request

### Code Standards

- **Documentation**: All public items must have doc comments (see [CLAUDE.md](CLAUDE.md))
- **Testing**: New features require unit tests; financial logic requires property tests
- **Type Safety**: Use strong types; avoid `unwrap()` in library code
- **Error Handling**: Use `Result` with descriptive error types

### Documentation Format

```rust
/// Brief summary of the function.
///
/// Detailed description if needed.
///
/// # Arguments
///
/// * `param1` - Description of parameter
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// * `ErrorType::Variant` - When this error occurs
///
/// # Examples
///
/// ```rust
/// let result = my_function(arg)?;
/// ```
pub fn my_function(param1: Type) -> Result<ReturnType, ErrorType> {
    // implementation
}
```

---

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

```
Copyright 2024 Open Insurance Core Contributors

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```

---

## Acknowledgments

- [Rust Language](https://www.rust-lang.org/) - Systems programming language
- [Tokio](https://tokio.rs/) - Async runtime for Rust
- [SQLx](https://github.com/launchbadge/sqlx) - Async SQL toolkit
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [zen-engine](https://github.com/gorules/zen) - Rules engine
- [proptest](https://proptest-rs.github.io/proptest/) - Property testing framework

---

<p align="center">
  <strong>Built with Rust for reliability, precision, and performance.</strong>
</p>
