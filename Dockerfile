# Open Insurance Core - Multi-stage Dockerfile
# This Dockerfile creates a complete test harness image including:
# - The built Rust application
# - All test binaries
# - PostgreSQL for integration tests
#
# Usage:
#   Build: docker build -t open-insurance-core:test .
#   Run tests: docker run --rm open-insurance-core:test

# ============================================
# Stage 1: Build Stage
# ============================================
FROM rust:1.83-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Create a new directory for the app
WORKDIR /app

# Copy the workspace files first for dependency caching
COPY Cargo.toml Cargo.lock ./

# Copy all crate Cargo.toml files
COPY crates/core_kernel/Cargo.toml crates/core_kernel/
COPY crates/infra_db/Cargo.toml crates/infra_db/
COPY crates/domain_policy/Cargo.toml crates/domain_policy/
COPY crates/domain_billing/Cargo.toml crates/domain_billing/
COPY crates/domain_fund/Cargo.toml crates/domain_fund/
COPY crates/domain_claims/Cargo.toml crates/domain_claims/
COPY crates/domain_party/Cargo.toml crates/domain_party/
COPY crates/interface_api/Cargo.toml crates/interface_api/
COPY crates/test_utils/Cargo.toml crates/test_utils/

# Create dummy source files for dependency caching
RUN mkdir -p crates/core_kernel/src && echo "pub fn dummy() {}" > crates/core_kernel/src/lib.rs
RUN mkdir -p crates/infra_db/src && echo "pub fn dummy() {}" > crates/infra_db/src/lib.rs
RUN mkdir -p crates/domain_policy/src && echo "pub fn dummy() {}" > crates/domain_policy/src/lib.rs
RUN mkdir -p crates/domain_billing/src && echo "pub fn dummy() {}" > crates/domain_billing/src/lib.rs
RUN mkdir -p crates/domain_fund/src && echo "pub fn dummy() {}" > crates/domain_fund/src/lib.rs
RUN mkdir -p crates/domain_claims/src && echo "pub fn dummy() {}" > crates/domain_claims/src/lib.rs
RUN mkdir -p crates/domain_party/src && echo "pub fn dummy() {}" > crates/domain_party/src/lib.rs
RUN mkdir -p crates/interface_api/src && echo "pub fn dummy() {}" > crates/interface_api/src/lib.rs
RUN mkdir -p crates/test_utils/src && echo "pub fn dummy() {}" > crates/test_utils/src/lib.rs

# Build dependencies only (this layer will be cached)
RUN cargo build --release 2>/dev/null || true
RUN cargo build --release --tests 2>/dev/null || true

# Remove the dummy source files
RUN rm -rf crates/*/src

# Copy actual source code
COPY crates/ crates/
COPY migrations/ migrations/

# Build the release binaries and test binaries
RUN cargo build --release --all-targets

# Run clippy for lint checks
RUN cargo clippy --all-targets --all-features -- -D warnings || true

# Build test binaries
RUN cargo test --release --no-run

# ============================================
# Stage 2: Test Runtime Image
# ============================================
FROM debian:bookworm-slim AS test-runtime

# Install runtime dependencies and PostgreSQL
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    postgresql \
    postgresql-contrib \
    sudo \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -s /bin/bash appuser && \
    echo "appuser ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

# Set up PostgreSQL
ENV POSTGRES_USER=insurance_user
ENV POSTGRES_PASSWORD=insurance_pass
ENV POSTGRES_DB=insurance_test
ENV PGDATA=/var/lib/postgresql/data

# Configure PostgreSQL
RUN mkdir -p /var/lib/postgresql/data && \
    chown -R postgres:postgres /var/lib/postgresql && \
    mkdir -p /run/postgresql && \
    chown -R postgres:postgres /run/postgresql

# Initialize PostgreSQL database
USER postgres
RUN /usr/lib/postgresql/15/bin/initdb -D /var/lib/postgresql/data && \
    echo "host all all 0.0.0.0/0 md5" >> /var/lib/postgresql/data/pg_hba.conf && \
    echo "listen_addresses='*'" >> /var/lib/postgresql/data/postgresql.conf

USER root

# Create application directory
WORKDIR /app

# Copy built artifacts from builder stage
COPY --from=builder /app/target/release/ /app/target/release/
COPY --from=builder /app/migrations/ /app/migrations/

# Copy source code for test discovery
COPY --from=builder /app/crates/ /app/crates/
COPY --from=builder /app/Cargo.toml /app/Cargo.toml
COPY --from=builder /app/Cargo.lock /app/Cargo.lock

# Copy test runner script
COPY scripts/run-tests.sh /app/scripts/run-tests.sh
RUN chmod +x /app/scripts/run-tests.sh

# Set ownership
RUN chown -R appuser:appuser /app

# Environment variables for database connection
ENV DATABASE_URL=postgres://insurance_user:insurance_pass@localhost:5432/insurance_test
ENV RUST_BACKTRACE=1
ENV RUST_LOG=info

# Expose PostgreSQL port
EXPOSE 5432

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD pg_isready -U postgres || exit 1

# Entry point
COPY scripts/docker-entrypoint.sh /docker-entrypoint.sh
RUN chmod +x /docker-entrypoint.sh

ENTRYPOINT ["/docker-entrypoint.sh"]
CMD ["test"]
