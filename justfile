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

# 快速编译检查（不生成可执行文件）
check-only:
    @echo "编译检查..."
    cargo check --workspace --all-targets --all-features

# 使用 Clippy 进行严格的静态检查
clippy-only:
    @echo "运行 Clippy 检查..."
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# 运行所有测试（含工作区）
test-only:
    @echo "运行测试..."
    cargo test --workspace --all-targets --all-features

# 运行测试并输出日志
test-nocapture-only:
    @echo "运行测试（--nocapture）..."
    cargo test --workspace --all-targets --all-features -- --nocapture

# Release 构建（用于发布/性能验证）
build-release-only:
    @echo "Release 构建..."
    cargo build --workspace --all-targets --all-features --release

# 使用 Clippy 进行严格的静态检查
clippy: fmt clippy-only
    @:

# 快速编译检查（不生成可执行文件）
check: fmt check-only
    @:

# 运行所有测试（含工作区）
test: fmt test-only
    @:

# 运行测试并输出日志
test-nocapture: fmt test-nocapture-only
    @:

# Release 构建（用于发布/性能验证）
build-release: fmt build-release-only
    @:

# 完整代码验证（fmt + check + test + clippy + build）
verify: fmt check-only test-only clippy-only build-release-only
    @echo "验证通过"

# 运行 pre-commit 检查
prek: fmt clippy-only
    @echo "pre-commit 检查..."
    prek run -a

# === 数据库操作 ===

# 说明：
# - 运行时迁移由后端自动执行（启动时会按 DATABASE_URL 选择迁移目录）。见 `crates/board/src/db/mod.rs`。
# - 下面命令仅用于“本地手工运行 sqlx-cli”场景（需要自行安装 sqlx-cli）。

db-migrate:
    #!/usr/bin/env bash
    set -euo pipefail
    : "${DATABASE_URL:?错误: DATABASE_URL 环境变量未设置}"
    if ! command -v sqlx >/dev/null 2>&1; then
        echo "未找到 sqlx-cli。安装："
        echo "  cargo install sqlx-cli --no-default-features --features sqlite,postgres"
        exit 1
    fi
    url="${DATABASE_URL}"
    src="crates/board/migrations"
    if [[ "$url" == postgres* ]]; then
        src="crates/board/migrations_pg"
    fi
    echo "运行迁移目录: $src"
    sqlx migrate run --source "$src"

# === 开发工作流 ===

# 完整开发验证流程（门禁全套）
dev: verify
    @echo "开发验证完成"

# 快速检查流程（编译 + clippy）
dev-quick: fmt check-only clippy-only
    @echo "快速检查完成"

# === 实用工具 ===

# 显示项目环境信息
info:
    #!/usr/bin/env bash
    echo "项目信息:"
    echo "  当前目录: $(pwd)"
    echo "  数据库 URL: ${DATABASE_URL:-<unset>}"

# 列出迁移文件（按 DATABASE_URL 选择目录）
list-migrations:
    #!/usr/bin/env bash
    set -euo pipefail
    url="${DATABASE_URL:-sqlite}"
    dir="crates/board/migrations"
    if [[ "$url" == postgres* ]]; then
        dir="crates/board/migrations_pg"
    fi
    echo "Migration 目录: $dir"
    find "$dir" -name "*.sql" | sort | sed 's/^/  /'

# 清理构建产物（cargo clean）
clean:
    @echo "清理构建产物..."
    cargo clean
    @echo "清理完成"

# 完全清理（包含数据库文件）
clean-all: clean
    #!/usr/bin/env bash
    set -euo pipefail
    url="${DATABASE_URL:-}"
    if [[ -n "$url" && "$url" == sqlite* ]]; then
        path="${url#sqlite://}"
        path="${path#sqlite:}"
        path="${path%%\\?*}"
        if [[ -n "$path" ]]; then
            if [[ "$path" == /* ]]; then
                echo "跳过删除绝对路径 SQLite 文件（安全）：$path"
            else
                rm -f "$path" "$path-shm" "$path-wal"
                echo "已删除 SQLite 文件：$path"
            fi
        fi
    fi
    rm -f app.db app.db-shm app.db-wal elizabeth.db elizabeth.db-shm elizabeth.db-wal || true
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
    docker compose up -d --build

# 启动所有服务（PostgreSQL）
docker-up-pg:
    @echo "启动所有服务（PostgreSQL）..."
    ./scripts/docker_prepare_volumes.sh
    docker compose -f docker-compose.yml -f docker-compose.postgres.yml up -d --build

# 停止所有服务
docker-down:
    @echo "停止所有服务..."
    docker compose down --remove-orphans

# 重建并启动所有服务
docker-rebuild:
    @echo "重建并启动所有服务..."
    ./scripts/docker_prepare_volumes.sh
    docker compose up -d --force-recreate --build

# 重建并启动所有服务（PostgreSQL）
docker-rebuild-pg:
    @echo "重建并启动所有服务（PostgreSQL）..."
    ./scripts/docker_prepare_volumes.sh
    docker compose -f docker-compose.yml -f docker-compose.postgres.yml up -d --force-recreate --build

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
    docker compose down -v --remove-orphans
    docker system prune -f

# === 命令别名 ===

alias f := fmt
alias c := check
alias t := test
alias p := prek
alias m := db-migrate
alias d := dev
alias dq := dev-quick
alias i := info

# Docker 别名

alias db := docker-build
alias du := docker-up
alias dd := docker-down
alias dr := docker-rebuild
alias dl := docker-logs
alias dupg := docker-up-pg
alias drpg := docker-rebuild-pg
