#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATA_DIR="$ROOT_DIR/docker/backend/data"
STORAGE_ROOMS_DIR="$ROOT_DIR/docker/backend/storage/rooms"
CONFIG_FILE="$ROOT_DIR/docker/backend/config/backend.yaml"
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
