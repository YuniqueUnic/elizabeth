#!/bin/bash
set -euo pipefail

# ─────────────────────────────────────────────────────
# Elizabeth Service Management Script
#
# Architecture: single binary — the Rust `board` process
# serves both the API (/api/v1/*) and the embedded SPA
# frontend on the same port (default 4092).
#
# Usage:
#   ./manage_services.sh build          # build frontend + backend
#   ./manage_services.sh start          # start the server
#   ./manage_services.sh stop           # stop the server
#   ./manage_services.sh restart        # stop + start
#   ./manage_services.sh status         # check if running
#   ./manage_services.sh logs           # tail the log file
# ─────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_FILE="$SCRIPT_DIR/server.log"
PID_FILE="$SCRIPT_DIR/server.pid"

# Load .env if present
ENV_FILE="$SCRIPT_DIR/.env"
if [ -f "$ENV_FILE" ]; then
    set -a
    # shellcheck disable=SC1090
    source "$ENV_FILE"
    set +a
fi

PORT="${PORT:-4092}"
LISTEN_ADDR="${LISTEN_ADDR:-127.0.0.1}"

# ── Helpers ──────────────────────────────────────────

is_running() {
    [ -f "$PID_FILE" ] && ps -p "$(cat "$PID_FILE")" > /dev/null 2>&1
}

kill_port() {
    local port="$1"
    local pids
    pids=$(lsof -t -i:"$port" 2>/dev/null || true)
    if [ -n "$pids" ]; then
        echo "  Killing processes on port $port: $pids"
        echo "$pids" | xargs kill -9 2>/dev/null || true
    fi
}

# ── Commands ─────────────────────────────────────────

cmd_build() {
    echo "==> Building frontend (static export)..."
    cd "$SCRIPT_DIR/web" || exit 1
    if ! bun run build:embedded; then
        echo "Frontend build failed." >&2
        exit 1
    fi

    echo "==> Building backend (debug binary with embedded SPA)..."
    cd "$SCRIPT_DIR" || exit 1
    cargo build -p elizabeth-board
    echo "==> Build complete. Binary: target/debug/board"
}

cmd_start() {
    if is_running; then
        echo "Server is already running (PID: $(cat "$PID_FILE"))."
        return
    fi

    kill_port "$PORT"

    echo "Starting Elizabeth server..."
    echo "  PORT=$PORT"
    echo "  LISTEN_ADDR=$LISTEN_ADDR"
    echo "  LOG=$LOG_FILE"

    cd "$SCRIPT_DIR" || exit 1

    # Prefer the release binary if it exists, otherwise use cargo run
    local cmd
    if [ -x "$SCRIPT_DIR/target/debug/board" ]; then
        cmd="$SCRIPT_DIR/target/debug/board run"
    else
        cmd="cargo run -p elizabeth-board -- run"
    fi

    PORT="$PORT" \
    LISTEN_ADDR="$LISTEN_ADDR" \
        nohup $cmd > "$LOG_FILE" 2>&1 &
    echo $! > "$PID_FILE"

    sleep 2
    if is_running; then
        echo "Server started (PID: $(cat "$PID_FILE"))."
    else
        echo "Failed to start. Check log: $LOG_FILE" >&2
        rm -f "$PID_FILE"
        exit 1
    fi
}

cmd_stop() {
    if ! is_running; then
        echo "Server is not running."
        rm -f "$PID_FILE"
        return
    fi

    local pid
    pid=$(cat "$PID_FILE")
    echo "Stopping server (PID: $pid)..."
    kill "$pid" 2>/dev/null || true

    for _ in {1..5}; do
        if ! ps -p "$pid" > /dev/null 2>&1; then
            break
        fi
        sleep 1
    done

    if ps -p "$pid" > /dev/null 2>&1; then
        echo "Graceful stop failed, sending SIGKILL..."
        kill -9 "$pid" 2>/dev/null || true
    fi

    rm -f "$PID_FILE"
    echo "Server stopped."
}

cmd_restart() {
    cmd_stop
    sleep 1
    cmd_start
}

cmd_status() {
    if is_running; then
        echo "Elizabeth server: RUNNING (PID: $(cat "$PID_FILE"), port $PORT)"
    else
        echo "Elizabeth server: STOPPED"
    fi
}

cmd_logs() {
    if [ -f "$LOG_FILE" ]; then
        tail -f "$LOG_FILE"
    else
        echo "No log file found at $LOG_FILE"
        exit 1
    fi
}

# ── Main ─────────────────────────────────────────────

case "${1:-}" in
    build)   cmd_build   ;;
    start)   cmd_start   ;;
    stop)    cmd_stop    ;;
    restart) cmd_restart ;;
    status)  cmd_status  ;;
    logs)    cmd_logs    ;;
    *)
        echo "Usage: $0 {build|start|stop|restart|status|logs}"
        echo ""
        echo "Commands:"
        echo "  build    Build frontend (static export) + backend (release binary)"
        echo "  start    Start the server (single binary, serves API + SPA on port $PORT)"
        echo "  stop     Stop the server"
        echo "  restart  Stop then start"
        echo "  status   Check if the server is running"
        echo "  logs     Tail the server log"
        exit 1
        ;;
esac

exit 0
