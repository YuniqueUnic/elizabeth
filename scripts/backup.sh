#!/bin/bash
# ============================================================================
# Elizabeth Backup Script
# ============================================================================
# This script backs up the database and storage volumes

set -e

# Configuration
BACKUP_DIR="./backups"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_NAME="elizabeth_backup_${TIMESTAMP}"

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

# Create backup directory
mkdir -p "${BACKUP_DIR}"

log_info "Starting backup: ${BACKUP_NAME}"

# Check if containers are running
if ! docker-compose ps | grep -q "elizabeth-backend"; then
    log_warn "Backend container is not running. Backup may be incomplete."
fi

# Backup database
log_info "Backing up database..."
docker run --rm \
    -v elizabeth_backend-data:/data \
    -v "$(pwd)/${BACKUP_DIR}:/backup" \
    alpine tar czf "/backup/${BACKUP_NAME}_data.tar.gz" -C /data .

if [ $? -eq 0 ]; then
    log_info "Database backup completed: ${BACKUP_DIR}/${BACKUP_NAME}_data.tar.gz"
else
    log_error "Database backup failed!"
    exit 1
fi

# Backup storage
log_info "Backing up storage..."
docker run --rm \
    -v elizabeth_backend-storage:/data \
    -v "$(pwd)/${BACKUP_DIR}:/backup" \
    alpine tar czf "/backup/${BACKUP_NAME}_storage.tar.gz" -C /data .

if [ $? -eq 0 ]; then
    log_info "Storage backup completed: ${BACKUP_DIR}/${BACKUP_NAME}_storage.tar.gz"
else
    log_error "Storage backup failed!"
    exit 1
fi

# Create backup info file
cat > "${BACKUP_DIR}/${BACKUP_NAME}_info.txt" << EOF
Backup Information
==================
Timestamp: ${TIMESTAMP}
Date: $(date)
Database: ${BACKUP_NAME}_data.tar.gz
Storage: ${BACKUP_NAME}_storage.tar.gz

Docker Compose Version:
$(docker-compose version)

Container Status:
$(docker-compose ps)
EOF

log_info "Backup info saved: ${BACKUP_DIR}/${BACKUP_NAME}_info.txt"

# Calculate sizes
DATA_SIZE=$(du -h "${BACKUP_DIR}/${BACKUP_NAME}_data.tar.gz" | cut -f1)
STORAGE_SIZE=$(du -h "${BACKUP_DIR}/${BACKUP_NAME}_storage.tar.gz" | cut -f1)

log_info "Backup completed successfully!"
log_info "  Database: ${DATA_SIZE}"
log_info "  Storage: ${STORAGE_SIZE}"
log_info "  Location: ${BACKUP_DIR}"

# Optional: Clean up old backups (keep last 7 days)
log_info "Cleaning up old backups (keeping last 7 days)..."
find "${BACKUP_DIR}" -name "elizabeth_backup_*" -type f -mtime +7 -delete
log_info "Cleanup completed"

exit 0
