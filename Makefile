.PHONY: help \
        docker-backend-cache docker-frontend-cache \
        docker-backend-binary docker-frontend-binary \
        docker-backend-image docker-frontend-image \
        docker-backend-up docker-frontend-up \
        docker-backend-stop docker-frontend-stop \
        docker-backend-recreate docker-frontend-recreate

help:
	@echo "Elizabeth Docker Commands"
	@echo "=========================="
	@echo "Cache layers:"
	@echo "  make docker-backend-cache    # Build backend cache (planner)"
	@echo "  make docker-frontend-cache   # Build frontend cache (deps)"
	@echo "Binaries:"
	@echo "  make docker-backend-binary   # Build backend builder image"
	@echo "  make docker-frontend-binary  # Build frontend builder image"
	@echo "Images:"
	@echo "  make docker-backend-image    # Build backend runtime image"
	@echo "  make docker-frontend-image   # Build frontend runtime image"
	@echo "Containers:"
	@echo "  make docker-backend-up       # Start backend container"
	@echo "  make docker-frontend-up      # Start frontend container"
	@echo "  make docker-backend-stop     # Stop backend container"
	@echo "  make docker-frontend-stop    # Stop frontend container"
	@echo "  make docker-backend-recreate # Recreate backend container"
	@echo "  make docker-frontend-recreate # Recreate frontend container"

# Cache layers

docker-backend-cache:
	@echo "🔧 构建后端 cache 层 (planner)..."
	docker build --target planner -f Dockerfile.backend -t elizabeth-backend-cache:latest .

docker-frontend-cache:
	@echo "🔧 构建前端 cache 层 (deps)..."
	docker build --target deps -f Dockerfile.frontend -t elizabeth-frontend-cache:latest .

# Binaries

docker-backend-binary:
	@echo "🔨 构建后端二进制 (builder)..."
	docker build --target builder -f Dockerfile.backend -t elizabeth-backend-builder:latest .

docker-frontend-binary:
	@echo "🔨 构建前端二进制 (builder)..."
	docker build --target builder -f Dockerfile.frontend -t elizabeth-frontend-builder:latest .

# Runtime images

docker-backend-image:
	@echo "🐳 构建后端运行时镜像..."
	docker build --target runtime -f Dockerfile.backend -t elizabeth-backend:latest .

docker-frontend-image:
	@echo "🐳 构建前端运行时镜像..."
	docker build --target runner -f Dockerfile.frontend -t elizabeth-frontend:latest .

# Container lifecycle

docker-backend-up:
	@echo "▶️ 启动后端容器..."
	./scripts/docker_prepare_volumes.sh
	docker compose up -d backend

docker-frontend-up: docker-backend-up
	@echo "▶️ 启动前端容器..."
	docker compose up -d frontend

docker-backend-stop:
	@echo "⏹️ 停止后端容器..."
	docker compose stop backend

docker-frontend-stop:
	@echo "⏹️ 停止前端容器..."
	docker compose stop frontend

docker-backend-recreate:
	@echo "🔄 重建后端容器..."
	./scripts/docker_prepare_volumes.sh
	docker compose up -d --force-recreate backend

docker-frontend-recreate: docker-backend-recreate
	@echo "🔄 重建前端容器..."
	docker compose up -d --force-recreate frontend
