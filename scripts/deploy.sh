#!/bin/bash
# ============================================================================
# Elizabeth Deployment Script
# ============================================================================
# This script automates the deployment process

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

compose() {
    if command -v docker-compose &> /dev/null; then
        docker-compose "$@"
        return
    fi
    docker compose "$@"
}

read_env_var() {
    local key="$1"
    local file="$2"
    grep -E "^${key}=" "$file" 2>/dev/null | tail -n1 | cut -d'=' -f2- || true
}

# Check if .env file exists
if [ ! -f .env ]; then
    log_warn ".env file not found. Creating from template..."
    if [ -f .env.docker ]; then
        cp .env.docker .env
        log_info "Created .env from .env.docker"
        log_warn "Please edit .env file and set appropriate values, especially JWT_SECRET!"
        log_warn "Run this script again after editing .env"
        exit 1
    else
        log_error ".env.docker template not found!"
        exit 1
    fi
fi

# Check JWT_SECRET
JWT_SECRET=$(read_env_var "JWT_SECRET" ".env")
if [ "$JWT_SECRET" = "please-change-this-secret-in-production-min-32-chars-long-for-security" ]; then # pragma: allowlist secret
    log_error "JWT_SECRET is still using the default value!"
    log_error "Please generate a secure secret and update .env file"
    log_info "You can generate a secure secret with: openssl rand -base64 48"
    exit 1
fi

# Deployment steps
log_step "1/6 Checking Docker and Docker Compose..."
if ! command -v docker &> /dev/null; then
    log_error "Docker is not installed!"
    exit 1
fi

if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
    log_error "Docker Compose is not installed! (need docker compose v2 or docker-compose v1)"
    exit 1
fi

log_info "Docker version: $(docker --version)"
log_info "Docker Compose version: $(compose version)"

log_step "2/6 Creating backup of existing data (if any)..."
DATA_DIR="${ELIZABETH_DATA_DIR:-$(read_env_var "ELIZABETH_DATA_DIR" ".env")}"
STORAGE_DIR="${ELIZABETH_STORAGE_DIR:-$(read_env_var "ELIZABETH_STORAGE_DIR" ".env")}"
DATA_DIR="${DATA_DIR:-./docker/backend/data}"
STORAGE_DIR="${STORAGE_DIR:-./docker/backend/storage}"

if [ -d "${DATA_DIR}" ] || [ -d "${STORAGE_DIR}" ]; then
    if [ "$(ls -A "${DATA_DIR}" 2>/dev/null | wc -l | tr -d ' ')" != "0" ] || \
       [ "$(ls -A "${STORAGE_DIR}" 2>/dev/null | wc -l | tr -d ' ')" != "0" ]; then
        log_info "Existing data found. Creating backup..."
        ./scripts/backup.sh || log_warn "Backup failed, continuing anyway..."
    else
        log_info "No existing data found, skipping backup"
    fi
else
    log_info "No existing data found, skipping backup"
fi

log_step "3/6 Building Docker images..."
compose build --no-cache

log_step "4/6 Stopping existing containers..."
compose down

log_step "5/6 Starting services..."
compose up -d

log_step "6/6 Waiting for backend to be healthy..."
sleep 5

# Check service health
MAX_RETRIES=30
RETRY_COUNT=0

while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
    BACKEND_HEALTH=$(docker inspect elizabeth --format='{{.State.Health.Status}}' 2>/dev/null || echo "unknown")

    if [ "$BACKEND_HEALTH" = "healthy" ]; then
        log_info "Backend is healthy!"
        break
    fi

    log_info "Waiting for backend to be healthy... (${RETRY_COUNT}/${MAX_RETRIES})"
    log_info "  Backend: ${BACKEND_HEALTH}"

    sleep 2
    RETRY_COUNT=$((RETRY_COUNT + 1))
done

if [ $RETRY_COUNT -eq $MAX_RETRIES ]; then
    log_error "Backend did not become healthy in time!"
    log_error "Check logs with: docker compose logs"
    exit 1
fi

# Display service information
log_info "Deployment completed successfully!"
echo ""
log_info "Service URLs (single container on port ${PORT:-4092}):"
log_info "  Web UI: http://localhost:${PORT:-4092}"
log_info "  API Docs: http://localhost:${PORT:-4092}/api/v1/scalar"
log_info "  Health: http://localhost:${PORT:-4092}/api/v1/health"
echo ""
log_info "Useful commands:"
if command -v docker-compose &> /dev/null; then
    log_info "  View logs: docker-compose logs -f"
    log_info "  Check status: docker-compose ps"
    log_info "  Stop services: docker-compose down"
    log_info "  Restart services: docker-compose restart"
else
    log_info "  View logs: docker compose logs -f"
    log_info "  Check status: docker compose ps"
    log_info "  Stop services: docker compose down"
    log_info "  Restart services: docker compose restart"
fi

exit 0
