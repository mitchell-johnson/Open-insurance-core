#!/bin/bash
# Open Insurance Core - Development Entrypoint Script
#
# This script initializes the development environment and runs the requested command.
#
# Commands:
#   serve       - Start the API server (production binary)
#   dev         - Start with hot reload using cargo-watch
#   shell       - Interactive shell
#   test        - Run test suite
#   migrate     - Run database migrations only
#   psql        - Connect to PostgreSQL CLI

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

print_banner() {
    echo -e "${CYAN}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║     Open Insurance Core - Development Environment            ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Start PostgreSQL service
start_postgres() {
    print_info "Starting PostgreSQL..."

    # Ensure directories exist with correct permissions
    sudo mkdir -p /var/run/postgresql /var/log/postgresql
    sudo chown -R postgres:postgres /var/run/postgresql /var/log/postgresql

    # Start PostgreSQL
    sudo -u postgres /usr/lib/postgresql/16/bin/pg_ctl \
        -D /var/lib/postgresql/16/main \
        -l /var/log/postgresql/postgresql.log \
        start

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
        sudo cat /var/log/postgresql/postgresql.log
        exit 1
    fi

    # Create database and user if they don't exist
    setup_database
}

# Set up database user and schema
setup_database() {
    print_info "Setting up database..."

    # Create user if not exists
    sudo -u postgres psql -tc "SELECT 1 FROM pg_roles WHERE rolname = '${POSTGRES_USER}'" | grep -q 1 || \
        sudo -u postgres psql -c "CREATE USER ${POSTGRES_USER} WITH PASSWORD '${POSTGRES_PASSWORD}';"

    # Create database if not exists
    sudo -u postgres psql -tc "SELECT 1 FROM pg_database WHERE datname = '${POSTGRES_DB}'" | grep -q 1 || \
        sudo -u postgres psql -c "CREATE DATABASE ${POSTGRES_DB} OWNER ${POSTGRES_USER};"

    # Grant privileges
    sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE ${POSTGRES_DB} TO ${POSTGRES_USER};" 2>/dev/null || true

    # Set DATABASE_URL
    export DATABASE_URL="postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@localhost:5432/${POSTGRES_DB}"

    print_success "Database configured: ${POSTGRES_DB}"
}

# Run database migrations
run_migrations() {
    print_info "Running database migrations..."

    if [ -f "/app/migrations/20240101_000001_initial_schema.sql" ]; then
        # Run migrations as the application user
        PGPASSWORD="${POSTGRES_PASSWORD}" psql \
            -h localhost \
            -U "${POSTGRES_USER}" \
            -d "${POSTGRES_DB}" \
            -f /app/migrations/20240101_000001_initial_schema.sql 2>/dev/null || true
        print_success "Migrations applied"
    else
        print_warning "No migration files found"
    fi
}

# Stop PostgreSQL gracefully
stop_postgres() {
    print_info "Stopping PostgreSQL..."
    sudo -u postgres /usr/lib/postgresql/16/bin/pg_ctl \
        -D /var/lib/postgresql/16/main \
        stop -m fast 2>/dev/null || true
}

# Cleanup handler for graceful shutdown
cleanup() {
    print_info "Shutting down..."
    stop_postgres
    exit 0
}

# Set up signal handlers
trap cleanup SIGTERM SIGINT

# Print environment info
print_environment() {
    echo ""
    print_info "Environment Configuration:"
    echo "  API_HOST:     ${API_HOST:-0.0.0.0}"
    echo "  API_PORT:     ${API_PORT:-8080}"
    echo "  DATABASE:     ${POSTGRES_DB}"
    echo "  LOG_LEVEL:    ${API_LOG_LEVEL:-info}"
    echo ""
    print_info "Endpoints:"
    echo "  Health:       http://localhost:${API_PORT:-8080}/health"
    echo "  API:          http://localhost:${API_PORT:-8080}/api/v1"
    echo "  PostgreSQL:   localhost:5432"
    echo ""
}

# Main entrypoint logic
main() {
    print_banner
    print_info "Command: $@"

    # Start PostgreSQL for all commands except help
    if [ "$1" != "help" ] && [ "$1" != "--help" ] && [ "$1" != "-h" ]; then
        start_postgres
        run_migrations
    fi

    case "$1" in
        serve)
            # Run the pre-built release binary
            print_info "Starting API server (release build)..."
            print_environment
            exec /usr/local/bin/insurance-api
            ;;

        dev)
            # Run with hot reload using cargo-watch
            print_info "Starting development server with hot reload..."
            print_environment
            print_warning "Watching for file changes..."
            cd /app
            exec cargo watch -x "run --bin insurance-api"
            ;;

        build)
            # Build the project
            print_info "Building project..."
            cd /app
            exec cargo build --release --bin insurance-api
            ;;

        test|tests)
            # Run tests
            print_info "Running tests..."
            shift
            exec /app/scripts/run-tests.sh "${@:-all}"
            ;;

        unit)
            # Run unit tests only
            print_info "Running unit tests..."
            exec /app/scripts/run-tests.sh unit
            ;;

        integration)
            # Run integration tests
            print_info "Running integration tests..."
            exec /app/scripts/run-tests.sh integration
            ;;

        migrate|migrations)
            # Already ran migrations above
            print_success "Migrations completed"
            ;;

        psql)
            # Connect to PostgreSQL CLI
            print_info "Connecting to PostgreSQL..."
            exec psql -h localhost -U "${POSTGRES_USER}" -d "${POSTGRES_DB}"
            ;;

        editor)
            # Start JDM Editor
            print_info "Starting JDM Editor on port 5173..."
            print_info "Access at: http://localhost:5173"
            cd /opt/jdm-editor
            exec npm run dev
            ;;

        all)
            # Start everything: API server and JDM Editor
            print_info "Starting API server and JDM Editor..."
            print_environment
            print_info "JDM Editor: http://localhost:5173"

            # Start JDM Editor in background
            cd /opt/jdm-editor
            npm run dev &

            # Start API server in foreground
            cd /app
            exec /usr/local/bin/insurance-api
            ;;

        shell|bash)
            # Interactive shell
            print_info "Starting interactive shell..."
            print_environment
            exec /bin/bash
            ;;

        help|--help|-h)
            echo "Usage: docker run [OPTIONS] insurance-dev COMMAND"
            echo ""
            echo "Commands:"
            echo "  serve       Start the API server (production binary)"
            echo "  dev         Start with hot reload (cargo-watch)"
            echo "  editor      Start JDM Editor on port 5173"
            echo "  all         Start API server + JDM Editor"
            echo "  build       Build the project"
            echo "  test        Run all tests"
            echo "  unit        Run unit tests only"
            echo "  integration Run integration tests"
            echo "  migrate     Run database migrations"
            echo "  psql        Connect to PostgreSQL CLI"
            echo "  shell       Start interactive bash shell"
            echo ""
            echo "Examples:"
            echo "  docker run -p 8080:8080 insurance-dev serve"
            echo "  docker run -p 8080:8080 -p 5173:5173 insurance-dev all"
            echo "  docker run -p 5173:5173 insurance-dev editor"
            echo "  docker run -p 8080:8080 -v \$(pwd):/app insurance-dev dev"
            echo "  docker run insurance-dev test"
            echo ""
            ;;

        *)
            # Run custom command
            print_info "Running custom command: $@"
            exec "$@"
            ;;
    esac
}

main "$@"
