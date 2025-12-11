#!/bin/bash
# Open Insurance Core - Docker Entrypoint Script
#
# This script initializes the container environment and runs the requested command.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Start PostgreSQL if we're running the all-in-one image
start_postgres() {
    if command -v pg_ctl &> /dev/null; then
        print_info "Starting PostgreSQL..."

        # Start PostgreSQL service
        sudo -u postgres /usr/lib/postgresql/15/bin/pg_ctl -D /var/lib/postgresql/data -l /var/log/postgresql/postgresql.log start

        # Wait for PostgreSQL to be ready
        local max_attempts=30
        local attempt=1
        while [ $attempt -le $max_attempts ]; do
            if sudo -u postgres pg_isready -q; then
                print_success "PostgreSQL is ready"
                break
            fi
            sleep 1
            attempt=$((attempt + 1))
        done

        if [ $attempt -gt $max_attempts ]; then
            print_error "PostgreSQL failed to start"
            exit 1
        fi

        # Create database and user if they don't exist
        sudo -u postgres psql -c "CREATE USER ${POSTGRES_USER:-insurance_user} WITH PASSWORD '${POSTGRES_PASSWORD:-insurance_pass}';" 2>/dev/null || true
        sudo -u postgres psql -c "CREATE DATABASE ${POSTGRES_DB:-insurance_test} OWNER ${POSTGRES_USER:-insurance_user};" 2>/dev/null || true
        sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE ${POSTGRES_DB:-insurance_test} TO ${POSTGRES_USER:-insurance_user};" 2>/dev/null || true

        # Set DATABASE_URL for local PostgreSQL
        export DATABASE_URL="postgres://${POSTGRES_USER:-insurance_user}:${POSTGRES_PASSWORD:-insurance_pass}@localhost:5432/${POSTGRES_DB:-insurance_test}"

        print_success "Database initialized"
    fi
}

# Run database migrations
run_migrations() {
    if [ -n "$DATABASE_URL" ] && [ -f "/app/migrations/20240101_000001_initial_schema.sql" ]; then
        print_info "Running database migrations..."
        PGPASSWORD="${POSTGRES_PASSWORD:-insurance_pass}" psql "$DATABASE_URL" -f /app/migrations/20240101_000001_initial_schema.sql 2>/dev/null || true
        print_success "Migrations completed"
    fi
}

# Main entrypoint logic
main() {
    print_info "Open Insurance Core - Container Starting"
    print_info "Command: $@"

    # Check if we should start internal PostgreSQL
    if [ "${START_POSTGRES:-false}" = "true" ] || [ "$1" = "all" ] || [ -z "$DATABASE_URL" ]; then
        start_postgres
    fi

    # Handle different commands
    case "$1" in
        test|unit|integration|api|property|all)
            run_migrations
            exec /app/scripts/run-tests.sh "$1"
            ;;
        shell|bash)
            exec /bin/bash
            ;;
        migrate)
            run_migrations
            print_success "Migrations applied"
            ;;
        serve)
            # Start the API server (if implemented)
            print_info "Starting API server..."
            exec /app/target/release/insurance-api
            ;;
        *)
            # If no recognized command, run it directly
            exec "$@"
            ;;
    esac
}

main "$@"
