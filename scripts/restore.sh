#!/bin/bash
# ============================================================================
# Elizabeth Restore Script
# ============================================================================
# This script restores the database and storage volumes from a backup

set -e

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
docker-compose down

# Restore database
log_info "Restoring database..."
docker run --rm \
    -v elizabeth_backend-data:/data \
    -v "$(pwd)/${BACKUP_DIR}:/backup" \
    alpine sh -c "rm -rf /data/* && tar xzf /backup/${BACKUP_NAME}_data.tar.gz -C /data"

if [ $? -eq 0 ]; then
    log_info "Database restored successfully"
else
    log_error "Database restore failed!"
    exit 1
fi

# Restore storage
log_info "Restoring storage..."
docker run --rm \
    -v elizabeth_backend-storage:/data \
    -v "$(pwd)/${BACKUP_DIR}:/backup" \
    alpine sh -c "rm -rf /data/* && tar xzf /backup/${BACKUP_NAME}_storage.tar.gz -C /data"

if [ $? -eq 0 ]; then
    log_info "Storage restored successfully"
else
    log_error "Storage restore failed!"
    exit 1
fi

# Start containers
log_info "Starting containers..."
docker-compose up -d

log_info "Restore completed successfully!"
log_info "Please check the application status:"
log_info "  docker-compose ps"
log_info "  docker-compose logs -f"

exit 0
