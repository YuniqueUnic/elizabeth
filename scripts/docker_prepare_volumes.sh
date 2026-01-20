#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

read_env_var() {
  local key="$1"
  local file="$2"
  grep -E "^${key}=" "$file" 2>/dev/null | tail -n1 | cut -d'=' -f2- || true
}

ENV_FILE="$ROOT_DIR/.env"
ENV_DATA_DIR=""
ENV_STORAGE_DIR=""
ENV_CONFIG_FILE=""
if [ -f "$ENV_FILE" ]; then
  ENV_DATA_DIR="$(read_env_var "ELIZABETH_DATA_DIR" "$ENV_FILE")"
  ENV_STORAGE_DIR="$(read_env_var "ELIZABETH_STORAGE_DIR" "$ENV_FILE")"
  ENV_CONFIG_FILE="$(read_env_var "ELIZABETH_BACKEND_CONFIG" "$ENV_FILE")"
fi

resolve_path() {
  local raw="$1"
  if [[ "$raw" = /* ]]; then
    echo "$raw"
  else
    echo "$ROOT_DIR/$raw"
  fi
}

DATA_DIR="$(resolve_path "${ELIZABETH_DATA_DIR:-${ENV_DATA_DIR:-docker/backend/data}}")"
STORAGE_DIR="$(resolve_path "${ELIZABETH_STORAGE_DIR:-${ENV_STORAGE_DIR:-docker/backend/storage}}")"
STORAGE_ROOMS_DIR="$STORAGE_DIR/rooms"
CONFIG_FILE="$(resolve_path "${ELIZABETH_BACKEND_CONFIG:-${ENV_CONFIG_FILE:-docker/backend/config/backend.yaml}}")"
DB_FILE="$DATA_DIR/elizabeth.db"

mkdir -p "$DATA_DIR"
mkdir -p "$STORAGE_ROOMS_DIR"

if [ ! -f "$CONFIG_FILE" ]; then
  cat >&2 <<'EOF'
[ERROR] 后端配置文件缺失: docker/backend/config/backend.yaml
请复制模板或从版本库恢复该文件后再启动 Docker。
EOF
  exit 1
fi

if command -v lsof >/dev/null 2>&1; then
  if [ -f "$DB_FILE" ] && lsof "$DB_FILE" >/dev/null 2>&1; then
    cat >&2 <<EOF
[ERROR] 侦测到 SQLite 数据库文件正在被占用：
  $DB_FILE

在 macOS Docker (virtiofs/gRPC FUSE) 环境下，SQLite 文件被其它进程占用时可能触发
“Device busy or not ready” 等错误。请先停止占用该文件的进程后再继续。
EOF
    exit 1
  fi
fi

echo "Docker 后端挂载目录就绪："
echo "  - $DATA_DIR"
echo "  - $STORAGE_ROOMS_DIR"
echo "  - $CONFIG_FILE"
