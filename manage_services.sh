#!/bin/bash

# Service Management Script

# --- Configuration ---
BACKEND_DIR="/Users/unic/dev/projs/rs/elizabeth"
BACKEND_CMD="cargo run -p elizabeth-board -- run"
BACKEND_LOG_FILE="$BACKEND_DIR/backend.log"
BACKEND_PID_FILE="$BACKEND_DIR/backend.pid"

FRONTEND_DIR="/Users/unic/dev/projs/rs/elizabeth/web"
FRONTEND_BUILD_CMD="pnpm build"
FRONTEND_CMD="node .next/standalone/server.js"
FRONTEND_LOG_FILE="$FRONTEND_DIR/frontend.log"
FRONTEND_PID_FILE="$FRONTEND_DIR/frontend.pid"


ENV_FILE="$BACKEND_DIR/.env"
if [ -f "$ENV_FILE" ]; then
    # shellcheck disable=SC1090
    set -a
    source "$ENV_FILE"
    set +a
fi

# --- Functions ---

start_backend() {
    echo "Ensuring port 4092 is free..."
    lsof -t -i:4092 | xargs kill -9 2>/dev/null || true

    if [ -f "$BACKEND_PID_FILE" ] && ps -p $(cat "$BACKEND_PID_FILE") > /dev/null; then
        echo "Backend is already running (PID: $(cat $BACKEND_PID_FILE))."
        return
    fi
    echo "Starting backend..."
    cd "$BACKEND_DIR" || exit 1
    nohup $BACKEND_CMD > "$BACKEND_LOG_FILE" 2>&1 &
    echo $! > "$BACKEND_PID_FILE"
    sleep 2 # Give it a moment to start
    if ps -p $(cat "$BACKEND_PID_FILE") > /dev/null; then
        echo "Backend started with PID $(cat $BACKEND_PID_FILE). Log: $BACKEND_LOG_FILE"
    else
        echo "Failed to start backend. Check log for details: $BACKEND_LOG_FILE"
        rm "$BACKEND_PID_FILE"
    fi
}

stop_backend() {
    if [ ! -f "$BACKEND_PID_FILE" ]; then
        echo "Backend is not running."
        return
    fi
    PID=$(cat "$BACKEND_PID_FILE")
    echo "Stopping backend (PID: $PID)..."
    if ps -p $PID > /dev/null; then
        kill "$PID"
        # Wait for the process to terminate
        for i in {1..5}; do
            if ! ps -p $PID > /dev/null; then
                break
            fi
            sleep 1
        done
    fi
    # Force kill if still running
    if ps -p $PID > /dev/null; then
        echo "Backend did not stop gracefully, forcing..."
        kill -9 "$PID"
    fi
    rm "$BACKEND_PID_FILE"
    echo "Backend stopped."
}

start_frontend() {
    echo "Ensuring port 4001 is free..."
    lsof -t -i:4001 | xargs kill -9 2>/dev/null || true

    if [ -f "$FRONTEND_PID_FILE" ] && ps -p $(cat "$FRONTEND_PID_FILE") > /dev/null; then
        echo "Frontend is already running (PID: $(cat $FRONTEND_PID_FILE))."
        return
    fi

    # For local development, always use localhost regardless of .env file
    local internal_api_url="http://localhost:4092/api/v1"
    local public_api_url="${MANAGER_NEXT_PUBLIC_API_URL:-${NEXT_PUBLIC_API_URL:-/api/v1}}"
    local app_url="${MANAGER_NEXT_PUBLIC_APP_URL:-${NEXT_PUBLIC_APP_URL:-http://localhost:4001}}"

    echo "Starting frontend..."
    cd "$FRONTEND_DIR" || exit 1

    if [ "${MANAGER_SKIP_FRONTEND_BUILD:-0}" != "1" ]; then
        echo "  Building frontend (production bundle)..."
        if ! $FRONTEND_BUILD_CMD > "$FRONTEND_DIR/frontend.build.log" 2>&1; then
            echo "Frontend build failed. Check $FRONTEND_DIR/frontend.build.log"
            return 1
        fi
        local standalone_dir="$FRONTEND_DIR/.next/standalone"
        local standalone_next_dir="$standalone_dir/.next"
        if [ -d "$standalone_dir" ]; then
            mkdir -p "$standalone_next_dir/static"
            rsync -a --delete "$FRONTEND_DIR/.next/static/" "$standalone_next_dir/static/" >/dev/null 2>&1
            if [ -d "$FRONTEND_DIR/public" ]; then
                mkdir -p "$standalone_dir/public"
                rsync -a --delete "$FRONTEND_DIR/public/" "$standalone_dir/public/" >/dev/null 2>&1
            fi
        else
            echo "  WARNING: Standalone output not found at $standalone_dir"
        fi
    else
        echo "  Skipping frontend build (MANAGER_SKIP_FRONTEND_BUILD=1)."
    fi

    echo "  NEXT_PUBLIC_API_URL=$public_api_url"
    echo "  INTERNAL_API_URL=$internal_api_url"
    echo "  NEXT_PUBLIC_APP_URL=$app_url"
    NODE_ENV=production \
    HOSTNAME=0.0.0.0 \
    PORT=4001 \
    NEXT_PUBLIC_API_URL="$public_api_url" \
    INTERNAL_API_URL="$internal_api_url" \
    NEXT_PUBLIC_APP_URL="$app_url" \
        nohup $FRONTEND_CMD > "$FRONTEND_LOG_FILE" 2>&1 &
    echo $! > "$FRONTEND_PID_FILE"
    sleep 2 # Give it a moment to start
    if ps -p $(cat "$FRONTEND_PID_FILE") > /dev/null; then
        echo "Frontend started with PID $(cat $FRONTEND_PID_FILE). Log: $FRONTEND_LOG_FILE"
    else
        echo "Failed to start frontend. Check log for details: $FRONTEND_LOG_FILE"
        rm "$FRONTEND_PID_FILE"
    fi
}

stop_frontend() {
    if [ ! -f "$FRONTEND_PID_FILE" ]; then
        echo "Frontend is not running."
        return
    fi
    PID=$(cat "$FRONTEND_PID_FILE")
    echo "Stopping frontend (PID: $PID)..."
    if ps -p $PID > /dev/null; then
       kill "$PID"
    fi
    rm "$FRONTEND_PID_FILE"
    echo "Frontend stopped."
}

# --- Main Logic ---

COMMAND=$1
SERVICE=$2

case "$COMMAND" in
    start)
        case "$SERVICE" in
            backend) start_backend ;;
            frontend) start_frontend ;;
            all) start_backend; start_frontend ;;
            *) echo "Usage: $0 start [backend|frontend|all]" >&2; exit 1 ;;
        esac
        ;;
    stop)
        case "$SERVICE" in
            backend) stop_backend ;;
            frontend) stop_frontend ;;
            all) stop_backend; stop_frontend ;;
            *) echo "Usage: $0 stop [backend|frontend|all]" >&2; exit 1 ;;
        esac
        ;;
    restart)
        case "$SERVICE" in
            backend) stop_backend; sleep 1; start_backend ;;
            frontend) stop_frontend; sleep 1; start_frontend ;;
            all) stop_backend; stop_frontend; sleep 1; start_backend; start_frontend ;;
            *) echo "Usage: $0 restart [backend|frontend|all]" >&2; exit 1 ;;
        esac
        ;;
    status)
        echo "--- Service Status ---"
        if [ -f "$BACKEND_PID_FILE" ] && ps -p $(cat "$BACKEND_PID_FILE") > /dev/null; then
            echo "Backend: RUNNING (PID: $(cat $BACKEND_PID_FILE))"
        else
            echo "Backend: STOPPED"
        fi
        if [ -f "$FRONTEND_PID_FILE" ] && ps -p $(cat "$FRONTEND_PID_FILE") > /dev/null; then
            echo "Frontend: RUNNING (PID: $(cat $FRONTEND_PID_FILE))"
        else
            echo "Frontend: STOPPED"
        fi
        ;;
    *)
        echo "Usage: $0 {start|stop|restart|status} [backend|frontend|all]" >&2
        exit 1
        ;;
esac

exit 0
