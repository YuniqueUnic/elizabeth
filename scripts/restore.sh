#!/bin/bash
# ============================================================================
# Elizabeth Restore Script
# ============================================================================
# This script restores the database and storage bind mounts from a backup

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
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

# Check arguments
if [ $# -ne 1 ]; then
    log_error "Usage: $0 <backup_name>"
    log_info "Example: $0 elizabeth_backup_20240101_120000"
    log_info "Available backups:"
    ls -1 ./backups/ | grep "elizabeth_backup_" | sed 's/_data.tar.gz//' | sed 's/_storage.tar.gz//' | sort -u
    exit 1
fi

BACKUP_NAME=$1
BACKUP_DIR="./backups"
DATA_DIR="./docker/backend/data"
STORAGE_DIR="./docker/backend/storage"
CONFIG_DIR="./docker/backend/config"

# Check if backup files exist
if [ ! -f "${BACKUP_DIR}/${BACKUP_NAME}_data.tar.gz" ]; then
    log_error "Database backup not found: ${BACKUP_DIR}/${BACKUP_NAME}_data.tar.gz"
    exit 1
fi

if [ ! -f "${BACKUP_DIR}/${BACKUP_NAME}_storage.tar.gz" ]; then
    log_error "Storage backup not found: ${BACKUP_DIR}/${BACKUP_NAME}_storage.tar.gz"
    exit 1
fi

# Confirm restore
log_warn "This will REPLACE all current data with the backup!"
log_warn "Backup: ${BACKUP_NAME}"
read -p "Are you sure you want to continue? (yes/no): " -r
if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
    log_info "Restore cancelled"
    exit 0
fi

# Stop containers
log_info "Stopping containers..."
if command -v docker-compose &> /dev/null; then
    docker-compose down
else
    docker compose down
fi

# Restore database
log_info "Restoring database..."
mkdir -p "${DATA_DIR}"
rm -rf "${DATA_DIR:?}/"*
tar xzf "${BACKUP_DIR}/${BACKUP_NAME}_data.tar.gz" -C "${DATA_DIR}"
log_info "Database restored successfully"

# Restore storage
log_info "Restoring storage..."
mkdir -p "${STORAGE_DIR}"
rm -rf "${STORAGE_DIR:?}/"*
tar xzf "${BACKUP_DIR}/${BACKUP_NAME}_storage.tar.gz" -C "${STORAGE_DIR}"
log_info "Storage restored successfully"

# Restore runtime config (optional)
if [ -f "${BACKUP_DIR}/${BACKUP_NAME}_config.tar.gz" ]; then
    log_info "Restoring config..."
    mkdir -p "${CONFIG_DIR}"
    tar xzf "${BACKUP_DIR}/${BACKUP_NAME}_config.tar.gz" -C "${CONFIG_DIR}"
    log_info "Config restored successfully"
fi

# Start containers
log_info "Starting containers..."
if command -v docker-compose &> /dev/null; then
    docker-compose up -d
else
    docker compose up -d
fi

log_info "Restore completed successfully!"
log_info "Please check the application status:"
if command -v docker-compose &> /dev/null; then
    log_info "  docker-compose ps"
    log_info "  docker-compose logs -f"
else
    log_info "  docker compose ps"
    log_info "  docker compose logs -f"
fi

exit 0
