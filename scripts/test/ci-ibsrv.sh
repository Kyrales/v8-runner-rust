#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ACTION="${1:-}"
RUNTIME_ROOT="${V8TR_CI_RUNTIME_ROOT:-$ROOT_DIR/target/manual-tests/live-cli-designer-runtime}"
IBSRV_PATH="${V8TR_IBSRV_PATH:-}"
INFOBASE_PATH="${V8TR_INFOBASE_PATH:-}"
DATA_DIR="${V8TR_IBSRV_DATA_DIR:-$RUNTIME_ROOT/ibsrv-data}"
LOG_PATH="${V8TR_IBSRV_LOG_PATH:-$RUNTIME_ROOT/ibsrv.log}"
PID_FILE="${V8TR_IBSRV_PID_FILE:-$RUNTIME_ROOT/ibsrv.pid}"

die() {
    echo "$*" >&2
    exit 2
}

write_env() {
    local key="$1"
    local value="$2"
    local delimiter="__V8TR_ENV__"

    if [[ -n "${GITHUB_ENV:-}" ]]; then
        {
            printf '%s<<%s\n' "$key" "$delimiter"
            printf '%s\n' "$value"
            printf '%s\n' "$delimiter"
        } >> "$GITHUB_ENV"
    fi
}

process_alive() {
    local pid="$1"
    kill -0 "$pid" >/dev/null 2>&1
}

stop_pid() {
    local pid="$1"

    if ! process_alive "$pid"; then
        return 0
    fi

    kill "$pid" >/dev/null 2>&1 || true

    for _ in $(seq 1 10); do
        if ! process_alive "$pid"; then
            return 0
        fi
        sleep 1
    done

    if command -v taskkill >/dev/null 2>&1; then
        taskkill //PID "$pid" //T //F >/dev/null 2>&1 || true
    fi

    kill -9 "$pid" >/dev/null 2>&1 || true
}

start_server() {
    [[ -n "$IBSRV_PATH" ]] || die "V8TR_IBSRV_PATH must be set before starting ibsrv"
    [[ -n "$INFOBASE_PATH" ]] || die "V8TR_INFOBASE_PATH must be set before starting ibsrv"

    mkdir -p "$RUNTIME_ROOT" "$DATA_DIR" "$INFOBASE_PATH" "$(dirname "$LOG_PATH")"

    if [[ -f "$PID_FILE" ]]; then
        stop_pid "$(cat "$PID_FILE")"
        rm -f "$PID_FILE"
    fi

    "$IBSRV_PATH" "--data=$DATA_DIR" "--db-path=$INFOBASE_PATH" >"$LOG_PATH" 2>&1 &
    local pid=$!
    printf '%s\n' "$pid" > "$PID_FILE"

    sleep 3
    if ! process_alive "$pid"; then
        [[ -f "$LOG_PATH" ]] && cat "$LOG_PATH" >&2
        die "ibsrv exited before the happy-path run started"
    fi

    write_env V8TR_IBSRV_DATA_DIR "$DATA_DIR"
    write_env V8TR_IBSRV_LOG_PATH "$LOG_PATH"
    write_env V8TR_IBSRV_PID "$pid"
    write_env V8TR_IBSRV_PID_FILE "$PID_FILE"

    echo "Started ibsrv sidecar on file infobase path: $INFOBASE_PATH"
}

stop_server() {
    local pid=""

    if [[ -f "$PID_FILE" ]]; then
        pid="$(cat "$PID_FILE")"
    elif [[ -n "${V8TR_IBSRV_PID:-}" ]]; then
        pid="${V8TR_IBSRV_PID}"
    fi

    if [[ -z "$pid" ]]; then
        echo "No ibsrv pid was recorded; skipping stop."
        return 0
    fi

    stop_pid "$pid"
    rm -f "$PID_FILE"
    echo "Stopped ibsrv sidecar (pid: $pid)"
}

case "$ACTION" in
    start)
        start_server
        ;;
    stop)
        stop_server
        ;;
    *)
        die "Usage: $0 start|stop"
        ;;
esac
