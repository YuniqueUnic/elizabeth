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
ENV_UID=""
ENV_GID=""
if [ -f "$ENV_FILE" ]; then
  ENV_DATA_DIR="$(read_env_var "ELIZABETH_DATA_DIR" "$ENV_FILE")"
  ENV_STORAGE_DIR="$(read_env_var "ELIZABETH_STORAGE_DIR" "$ENV_FILE")"
  ENV_CONFIG_FILE="$(read_env_var "ELIZABETH_BACKEND_CONFIG" "$ENV_FILE")"
  ENV_UID="$(read_env_var "ELIZABETH_UID" "$ENV_FILE")"
  ENV_GID="$(read_env_var "ELIZABETH_GID" "$ENV_FILE")"
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

# Must match the `user:` directive in docker-compose.yml.
CONTAINER_UID="${ELIZABETH_UID:-${ENV_UID:-65532}}"
CONTAINER_GID="${ELIZABETH_GID:-${ENV_GID:-65532}}"

mkdir -p "$DATA_DIR"
mkdir -p "$STORAGE_ROOMS_DIR"

# The image runs as the distroless `nonroot` user, and on Linux a bind mount keeps
# the host ownership, so directories created here would belong to the invoking user
# and the container could not write to them. Docker Desktop (macOS/Windows)
# virtualises bind-mount ownership and needs no chown.
if [ "$(uname -s)" = "Linux" ]; then
  if [ "$(id -u)" -eq 0 ]; then
    chown -R "$CONTAINER_UID:$CONTAINER_GID" "$DATA_DIR" "$STORAGE_DIR"
  else
    echo "将挂载目录移交给容器用户 $CONTAINER_UID:$CONTAINER_GID（需要 sudo）："
    sudo chown -R "$CONTAINER_UID:$CONTAINER_GID" "$DATA_DIR" "$STORAGE_DIR"
  fi
fi

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
echo "容器运行身份：$CONTAINER_UID:$CONTAINER_GID"
