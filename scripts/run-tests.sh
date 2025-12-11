#!/bin/bash
# Open Insurance Core - Test Runner Script
#
# This script runs the test suite with various options.
#
# Usage:
#   ./run-tests.sh          # Run all tests
#   ./run-tests.sh unit     # Run unit tests only
#   ./run-tests.sh integration  # Run integration tests only
#   ./run-tests.sh coverage # Run tests with coverage
#
# Environment variables:
#   DATABASE_URL    - PostgreSQL connection string
#   RUST_LOG        - Log level (trace, debug, info, warn, error)
#   TEST_THREADS    - Number of test threads (default: 4)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
TEST_MODE="${1:-all}"
TEST_THREADS="${TEST_THREADS:-4}"
RESULTS_DIR="${RESULTS_DIR:-./test-results}"

# Ensure results directory exists
mkdir -p "$RESULTS_DIR"

# Print banner
print_banner() {
    echo -e "${BLUE}"
    echo "╔════════════════════════════════════════════════════════════╗"
    echo "║         Open Insurance Core - Test Suite                   ║"
    echo "╚════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

# Print section header
print_section() {
    echo -e "\n${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}  $1${NC}"
    echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
}

# Print success message
print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

# Print error message
print_error() {
    echo -e "${RED}✗ $1${NC}"
}

# Print info message
print_info() {
    echo -e "${BLUE}ℹ $1${NC}"
}

# Wait for database to be ready
wait_for_db() {
    if [ -z "$DATABASE_URL" ]; then
        print_info "No DATABASE_URL set, skipping database wait"
        return 0
    fi

    print_section "Waiting for Database"

    local max_attempts=30
    local attempt=1

    while [ $attempt -le $max_attempts ]; do
        if pg_isready -d "$DATABASE_URL" >/dev/null 2>&1; then
            print_success "Database is ready"
            return 0
        fi

        echo "Attempt $attempt/$max_attempts: Database not ready, waiting..."
        sleep 2
        attempt=$((attempt + 1))
    done

    print_error "Database failed to become ready"
    return 1
}

# Run database migrations
run_migrations() {
    if [ -z "$DATABASE_URL" ]; then
        print_info "No DATABASE_URL set, skipping migrations"
        return 0
    fi

    print_section "Running Database Migrations"

    # Check if migration file exists
    if [ -f "/app/migrations/20240101_000001_initial_schema.sql" ]; then
        psql "$DATABASE_URL" -f /app/migrations/20240101_000001_initial_schema.sql 2>/dev/null || true
        print_success "Migrations applied"
    else
        print_info "No migration files found"
    fi
}

# Run unit tests
run_unit_tests() {
    print_section "Running Unit Tests"

    local start_time=$(date +%s)

    # Run tests for each crate
    local crates=(
        "core_kernel"
        "domain_policy"
        "domain_billing"
        "domain_fund"
        "domain_claims"
        "domain_party"
    )

    local failed=0

    for crate in "${crates[@]}"; do
        echo -e "\nTesting ${BLUE}$crate${NC}..."
        if cargo test -p "$crate" --release -- --test-threads="$TEST_THREADS" 2>&1 | tee "$RESULTS_DIR/$crate-unit.log"; then
            print_success "$crate tests passed"
        else
            print_error "$crate tests failed"
            failed=1
        fi
    done

    local end_time=$(date +%s)
    local duration=$((end_time - start_time))

    echo -e "\n${BLUE}Unit tests completed in ${duration}s${NC}"

    return $failed
}

# Run integration tests
run_integration_tests() {
    print_section "Running Integration Tests"

    if [ -z "$DATABASE_URL" ]; then
        print_error "DATABASE_URL required for integration tests"
        return 1
    fi

    local start_time=$(date +%s)

    # Run integration tests
    if cargo test --release --test '*' -- --test-threads="$TEST_THREADS" 2>&1 | tee "$RESULTS_DIR/integration.log"; then
        print_success "Integration tests passed"
    else
        print_error "Integration tests failed"
        return 1
    fi

    local end_time=$(date +%s)
    local duration=$((end_time - start_time))

    echo -e "\n${BLUE}Integration tests completed in ${duration}s${NC}"
}

# Run API tests
run_api_tests() {
    print_section "Running API Tests"

    if cargo test -p interface_api --release -- --test-threads="$TEST_THREADS" 2>&1 | tee "$RESULTS_DIR/api.log"; then
        print_success "API tests passed"
    else
        print_error "API tests failed"
        return 1
    fi
}

# Run property-based tests
run_property_tests() {
    print_section "Running Property-Based Tests"

    # Set more iterations for property tests in CI
    export PROPTEST_CASES=${PROPTEST_CASES:-1000}

    if cargo test --release -- --test-threads="$TEST_THREADS" proptest 2>&1 | tee "$RESULTS_DIR/proptest.log"; then
        print_success "Property tests passed"
    else
        print_error "Property tests failed"
        return 1
    fi
}

# Generate test summary
generate_summary() {
    print_section "Test Summary"

    local total_tests=0
    local passed_tests=0
    local failed_tests=0

    # Count results from log files
    for log in "$RESULTS_DIR"/*.log; do
        if [ -f "$log" ]; then
            local test_count=$(grep -c "test result:" "$log" 2>/dev/null || echo "0")
            local passed=$(grep "test result: ok" "$log" | grep -oP '\d+ passed' | grep -oP '\d+' || echo "0")
            local failed=$(grep "test result:" "$log" | grep -oP '\d+ failed' | grep -oP '\d+' || echo "0")

            passed_tests=$((passed_tests + passed))
            failed_tests=$((failed_tests + failed))
        fi
    done

    total_tests=$((passed_tests + failed_tests))

    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "Total Tests:  $total_tests"
    echo -e "Passed:       ${GREEN}$passed_tests${NC}"
    echo -e "Failed:       ${RED}$failed_tests${NC}"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    # Generate JSON summary
    cat > "$RESULTS_DIR/summary.json" << EOF
{
    "timestamp": "$(date -Iseconds)",
    "total_tests": $total_tests,
    "passed": $passed_tests,
    "failed": $failed_tests,
    "success_rate": $(echo "scale=2; $passed_tests * 100 / ($total_tests + 1)" | bc 2>/dev/null || echo "0")
}
EOF

    if [ $failed_tests -gt 0 ]; then
        return 1
    fi
    return 0
}

# Main execution
main() {
    print_banner

    local exit_code=0

    case "$TEST_MODE" in
        unit)
            run_unit_tests || exit_code=1
            ;;
        integration)
            wait_for_db || exit 1
            run_migrations
            run_integration_tests || exit_code=1
            ;;
        api)
            wait_for_db || exit 1
            run_migrations
            run_api_tests || exit_code=1
            ;;
        property)
            run_property_tests || exit_code=1
            ;;
        all)
            run_unit_tests || exit_code=1

            if [ -n "$DATABASE_URL" ]; then
                wait_for_db || exit 1
                run_migrations
                run_integration_tests || exit_code=1
                run_api_tests || exit_code=1
            fi

            run_property_tests || exit_code=1
            ;;
        *)
            echo "Usage: $0 {unit|integration|api|property|all}"
            exit 1
            ;;
    esac

    generate_summary

    if [ $exit_code -eq 0 ]; then
        print_success "All tests passed!"
    else
        print_error "Some tests failed"
    fi

    exit $exit_code
}

main "$@"
