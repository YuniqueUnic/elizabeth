# ============================================================================
# Elizabeth Makefile
# ============================================================================
# Alternative to justfile for users who prefer make
# For better experience, consider using just: https://github.com/casey/just

.PHONY: help docker-deploy docker-build docker-up docker-down docker-restart \
        docker-status docker-logs docker-backup docker-restore docker-clean \
        docker-shell-backend docker-shell-frontend docker-init docker-validate \
        docker-stats

# Default target
help:
	@echo "Elizabeth Docker Deployment Commands"
	@echo "====================================="
	@echo ""
	@echo "Deployment:"
	@echo "  make docker-deploy          - One-click deployment"
	@echo "  make docker-build           - Build Docker images"
	@echo "  make docker-up              - Start services"
	@echo "  make docker-down            - Stop services"
	@echo "  make docker-restart         - Restart services"
	@echo ""
	@echo "Monitoring:"
	@echo "  make docker-status          - View service status"
	@echo "  make docker-logs            - View all logs"
	@echo "  make docker-logs-backend    - View backend logs"
	@echo "  make docker-logs-frontend   - View frontend logs"
	@echo "  make docker-stats           - View resource usage"
	@echo ""
	@echo "Maintenance:"
	@echo "  make docker-backup          - Backup data"
	@echo "  make docker-restore NAME=<backup_name> - Restore data"
	@echo "  make docker-clean           - Clean up resources"
	@echo ""
	@echo "Debugging:"
	@echo "  make docker-shell-backend   - Enter backend container"
	@echo "  make docker-shell-frontend  - Enter frontend container"
	@echo "  make docker-validate        - Validate configuration"
	@echo ""
	@echo "Setup:"
	@echo "  make docker-init            - Initialize environment"
	@echo ""
	@echo "For more information, see docs/DOCKER_QUICK_START.md"

# ============================================================================
# Deployment Commands
# ============================================================================

docker-deploy:
	@echo "🚀 Deploying Elizabeth..."
	@./scripts/deploy.sh

docker-build:
	@echo "🏗️ Building Docker images..."
	@docker-compose build

# Optimized build commands for development
docker-build-binary:
	@echo "🔨 Building Rust binary (cached)..."
	@docker build --target binary-builder -t elizabeth-backend-binary:latest .

docker-build-backend:
	@echo "🐳 Building backend container with cached binary..."
	@docker build --target runtime -t elizabeth-backend:latest .

docker-rebuild-binary:
	@echo "🔄 Force rebuilding Rust binary (no cache)..."
	@docker build --target binary-builder --no-cache -t elizabeth-backend-binary:latest .

docker-up:
	@echo "▶️ Starting Docker services..."
	@docker-compose up -d

docker-down:
	@echo "⏹️ Stopping Docker services..."
	@docker-compose down

docker-restart:
	@echo "🔄 Restarting Docker services..."
	@docker-compose restart

# ============================================================================
# Monitoring Commands
# ============================================================================

docker-status:
	@echo "📊 Docker service status:"
	@docker-compose ps

docker-logs:
	@echo "📜 Viewing all service logs..."
	@docker-compose logs -f

docker-logs-backend:
	@echo "📜 Viewing backend logs..."
	@docker-compose logs -f backend

docker-logs-frontend:
	@echo "📜 Viewing frontend logs..."
	@docker-compose logs -f frontend

docker-stats:
	@echo "📈 Docker resource usage:"
	@docker stats --no-stream

# ============================================================================
# Maintenance Commands
# ============================================================================

docker-backup:
	@echo "💾 Backing up Docker data..."
	@./scripts/backup.sh

docker-restore:
	@if [ -z "$(NAME)" ]; then \
		echo "❌ Error: Please specify backup name with NAME=<backup_name>"; \
		echo "Example: make docker-restore NAME=elizabeth_backup_20240101_120000"; \
		exit 1; \
	fi
	@echo "🔙 Restoring Docker data: $(NAME)"
	@./scripts/restore.sh $(NAME)

docker-clean:
	@echo "🧹 Cleaning up Docker resources..."
	@docker-compose down -v
	@docker system prune -f

# ============================================================================
# Debugging Commands
# ============================================================================

docker-shell-backend:
	@echo "🔍 Entering backend container..."
	@docker-compose exec backend sh

docker-shell-frontend:
	@echo "🔍 Entering frontend container..."
	@docker-compose exec frontend sh

docker-validate:
	@echo "🔧 Validating Docker configuration..."
	@docker-compose config

# ============================================================================
# Setup Commands
# ============================================================================

docker-init:
	@if [ ! -f .env ]; then \
		echo "📦 Creating .env file..."; \
		cp .env.docker .env; \
		echo "⚠️  Please edit .env file and set JWT_SECRET!"; \
		echo "💡 Generate secret: openssl rand -base64 48"; \
	else \
		echo "✅ .env file already exists"; \
	fi
