# Justfile for Elizabeth - Rust File Sharing Platform
# 自动加载 .env 环境变量，包含开发、数据库与 Docker 任务。

set dotenv-load := true
set shell := ["bash", "-euo", "pipefail", "-c"]

# === 默认任务 ===

# 显示所有可用任务（命令帮助）
default:
    @just --list

# === 代码质量与构建检查 ===

# 格式化所有 Rust 代码
fmt:
    @echo "格式化代码..."
    cargo fmt --all

# 使用 Clippy 进行严格的静态检查
clippy: fmt
    @echo "运行 Clippy 检查..."
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# 快速编译检查（不生成可执行文件）
check: fmt
    @echo "编译检查..."
    cargo check --workspace --all-targets --all-features

# 运行所有测试（含工作区）
test: fmt
    @echo "运行测试..."
    cargo test --workspace -- --nocapture

# 完整代码验证（check + test）
verify: check test
    @echo "验证通过"

# 运行 pre-commit 检查
prek: fmt clippy
    @echo "pre-commit 检查..."
    prek run -a

# === 数据库操作 ===

# 检查必要环境变量是否存在
_check-env:
    #!/usr/bin/env bash
    : "${DATABASE_URL:?错误: DATABASE_URL 环境变量未设置}"
    : "${DATABASE_FILE:?错误: DATABASE_FILE 环境变量未设置}"
    : "${MIGRATIONS_DIR:?错误: MIGRATIONS_DIR 环境变量未设置}"

# 创建数据库文件（如不存在）
db-create: _check-env
    #!/usr/bin/env bash
    if [[ ! -f "$DATABASE_FILE" ]]; then
        echo "创建数据库文件: $DATABASE_FILE"
        mkdir -p "$(dirname "$DATABASE_FILE")"
        sqlite3 "$DATABASE_FILE" "SELECT 1;" >/dev/null
    else
        echo "数据库文件已存在: $DATABASE_FILE"
    fi

# 检查 migration 目录与文件状态
_check-migrations: _check-env
    #!/usr/bin/env bash
    if [[ ! -d "$MIGRATIONS_DIR" ]]; then
        echo "错误: Migration 目录不存在: $MIGRATIONS_DIR"
        exit 1
    fi
    count=$(find "$MIGRATIONS_DIR" -name "*.sql" | wc -l)
    if [[ $count -eq 0 ]]; then
        echo "未找到 migration 文件"
    else
        echo "找到 $count 个 migration 文件"
    fi

# 执行数据库迁移（sqlx migrate run）
migrate: db-create _check-migrations
    #!/usr/bin/env bash
    echo "运行数据库迁移..."
    if sqlx migrate run --source "$MIGRATIONS_DIR"; then
        echo "Migration 执行成功"
    else
        echo "Migration 执行失败"
        exit 1
    fi

# 重新生成 SQLx 查询缓存（sqlx prepare）
sqlx-prepare: migrate
    #!/usr/bin/env bash
    echo "重新生成 SQLx 查询缓存..."
    if cargo sqlx prepare --workspace; then
        echo "SQLx 缓存生成成功"
    else
        echo "SQLx prepare 失败"
        exit 1
    fi

# 检查 SQLx 缓存是否最新
sqlx-check:
    @echo "检查 SQLx 缓存..."
    cargo sqlx prepare --workspace --check

# 删除并重建数据库（重新执行迁移）
db-reset: _check-env
    #!/usr/bin/env bash
    echo "重置数据库..."
    if [[ -f "$DATABASE_FILE" ]]; then
        rm -f "$DATABASE_FILE"
        echo "已删除旧数据库文件"
    fi
    just migrate
    rm ./app.db*
    cp -f "$DATABASE_FILE" .
    echo "数据库重置完成"

# 数据库初始化（含 prepare 缓存）
db-bootstrap: sqlx-prepare
    @echo "数据库初始化完成"

# === 开发工作流 ===

# 完整开发验证流程（fmt + sqlx + test）
dev: fmt sqlx-prepare verify
    @echo "开发验证完成"

# 快速检查流程（fmt + sqlx + check）
dev-quick: fmt sqlx-prepare check
    @echo "快速检查完成"

# === 实用工具 ===

# 显示项目环境信息
info: _check-env
    #!/usr/bin/env bash
    echo "项目信息:"
    echo "  当前目录: $(pwd)"
    echo "  数据库文件: $DATABASE_FILE"
    echo "  Migration 目录: $MIGRATIONS_DIR"
    echo "  数据库 URL: $DATABASE_URL"
    if [[ -f "$DATABASE_FILE" ]]; then
        echo "  数据库状态: 存在"
        echo "  数据库大小: $(du -h "$DATABASE_FILE" | cut -f1)"
    else
        echo "  数据库状态: 不存在"
    fi

# 列出所有 migration 文件
list-migrations: _check-env
    #!/usr/bin/env bash
    echo "Migration 文件列表:"
    if [[ -d "$MIGRATIONS_DIR" ]]; then
        find "$MIGRATIONS_DIR" -name "*.sql" | sort | sed 's/^/  /'
    else
        echo "  Migration 目录不存在"
    fi

# 清理构建产物（cargo clean）
clean:
    @echo "清理构建产物..."
    cargo clean
    @echo "清理完成"

# 完全清理（包含数据库文件）
clean-all: clean
    #!/usr/bin/env bash
    if [[ -f "$DATABASE_FILE" ]]; then
        rm -f "$DATABASE_FILE"
        echo "数据库文件已删除"
    fi
    echo "完全清理完成"

# === Docker 操作 ===

# 构建后端镜像
docker-build-backend:
    @echo "构建后端镜像..."
    docker build --target runtime -f Dockerfile.backend -t elizabeth-backend:latest .

# 构建前端镜像
docker-build-frontend:
    @echo "构建前端镜像..."
    docker build --target runner -f Dockerfile.frontend -t elizabeth-frontend:latest .

# 构建所有镜像
docker-build: docker-build-backend docker-build-frontend
    @echo "所有镜像构建完成"

# 启动所有服务
docker-up:
    @echo "启动所有服务..."
    ./scripts/docker_prepare_volumes.sh
    docker compose up -d

# 停止所有服务
docker-down:
    @echo "停止所有服务..."
    docker compose down

# 重建并启动所有服务
docker-rebuild:
    @echo "重建并启动所有服务..."
    ./scripts/docker_prepare_volumes.sh
    docker compose up -d --force-recreate --build

# 查看服务日志
docker-logs:
    @echo "查看服务日志..."
    docker compose logs -f

# 查看后端日志
docker-logs-backend:
    @echo "查看后端日志..."
    docker compose logs -f backend

# 查看前端日志
docker-logs-frontend:
    @echo "查看前端日志..."
    docker compose logs -f frontend

# 清理 Docker 资源
docker-clean:
    @echo "清理 Docker 资源..."
    docker compose down -v
    docker system prune -f

# === 命令别名 ===

alias f := fmt
alias c := check
alias t := test
alias p := prek
alias m := migrate
alias d := dev
alias dq := dev-quick
alias i := info

# Docker 别名

alias db := docker-build
alias du := docker-up
alias dd := docker-down
alias dr := docker-rebuild
alias dl := docker-logs
