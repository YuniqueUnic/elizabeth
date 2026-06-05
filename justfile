# Elizabeth — 开发、构建、Docker

set dotenv-load := true
set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just --list

# ── 代码质量 ─────────────────────────────────────────

# 格式化 + 编译检查 + clippy
check:
    cargo fmt --all
    cargo check --workspace --all-targets --all-features
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# 运行所有测试
test:
    cargo fmt --all
    cargo test --workspace --all-targets --all-features

# 完整门禁（fmt + check + test + clippy）
verify: check test
    @echo "验证通过"

# ── 开发 ─────────────────────────────────────────────

# 启动开发服务器
dev port="4092" *ARG: build-web
    cargo run -p elizabeth-board -- run --port={{port}} {{ARG}}

# ── 构建 ─────────────────────────────────────────────

# 构建前端静态文件（Next.js → web/out/ → rust-embed）
build-web:
    cd web && bun run build:embedded

# 完整构建（前端 + Rust release 二进制）
build: build-web
    cargo build --release -p elizabeth-board

# ── Docker ───────────────────────────────────────────

# 构建后端镜像
docker-build:
    docker build --target runtime -f Dockerfile.backend -t elizabeth-backend:latest .

# 启动所有服务
docker-up:
    ./scripts/docker_prepare_volumes.sh
    docker compose up -d --build

# 启动所有服务（PostgreSQL）
docker-up-pg:
    ./scripts/docker_prepare_volumes.sh
    docker compose -f docker-compose.yml -f docker-compose.postgres.yml up -d --build

# 停止所有服务
docker-down:
    docker compose down --remove-orphans

# 重建并启动
docker-rebuild:
    ./scripts/docker_prepare_volumes.sh
    docker compose up -d --force-recreate --build

# 查看日志
docker-logs:
    docker compose logs -f

# 清理 Docker 资源
docker-clean:
    docker compose down -v --remove-orphans
    docker system prune -f

# ── 清理 ─────────────────────────────────────────────

# 清理 dev 产物(sqlite3, storage)
clean:
    rm -r storage
    rm app.db
    rm app.db-shm
    rm app.db-wal
    

# ── 别名 ─────────────────────────────────────────────

alias c := check
alias t := test
alias d := dev
alias b := build
alias db := docker-build
alias du := docker-up
alias dd := docker-down
alias dr := docker-rebuild
alias dl := docker-logs
