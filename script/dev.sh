#!/bin/sh
# Fast dev loop for vterm: rebuilds and relaunches the app whenever a source
# file changes (GPUI apps can't hot-swap code at runtime, so we relaunch).
#
# Uses cargo-watch if available; otherwise falls back to a portable
# find-based watcher. Debug profile keeps rebuilds fast.
set -e

cd "$(dirname "$0")/.."

dev_id="$(git rev-parse --short HEAD 2>/dev/null || printf 'nogit')-$(date -u +%Y%m%d%H%M%S)"
export VTERM_APP_NAME="${VTERM_APP_NAME:-vterm-dev}"
export VTERM_DEV_VERSION="${VTERM_DEV_VERSION:-dev-${dev_id}}"
export VTERM_DEV_BUILD=1
export VTERM_CONFIG_NAME="${VTERM_CONFIG_NAME:-vterm-dev}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/dev}"

case "$CARGO_TARGET_DIR" in
    /*) dev_binary="$CARGO_TARGET_DIR/debug/vterm" ;;
    *) dev_binary="$PWD/$CARGO_TARGET_DIR/debug/vterm" ;;
esac
dev_pid_file="$CARGO_TARGET_DIR/vterm-dev.pid"

stop_dev() {
    if [ ! -f "$dev_pid_file" ]; then
        return
    fi

    dev_pid=$(sed -n '1p' "$dev_pid_file")
    process=$(ps -p "$dev_pid" -o command= 2>/dev/null || true)
    case "$process" in
        *"$dev_binary")
            kill "$dev_pid" 2>/dev/null || true
            wait "$dev_pid" 2>/dev/null || true
            ;;
    esac
    rm -f "$dev_pid_file"
}

start_dev() {
    cargo build -p vterm
    "$dev_binary" &
    dev_pid=$!
    printf '%s\n' "$dev_pid" > "$dev_pid_file"
}

cleanup() {
    stop_dev
}

trap cleanup INT TERM EXIT

if command -v cargo-watch >/dev/null 2>&1; then
    trap - INT TERM EXIT
    exec cargo watch -q -c -x run
fi

echo "tip: 'cargo install cargo-watch' gives instant, debounced watching" >&2

# Start the first debug build immediately. The fallback watcher cannot observe
# changes while cargo run is in the foreground, so keep the app in the
# background and let the loop handle relaunches.
stop_dev
start_dev

prev=$(find Cargo.toml crates build.rs -type f 2>/dev/null | sort | xargs stat -f %m 2>/dev/null | md5)
while true; do
    sleep 1
    curr=$(find Cargo.toml crates build.rs -type f 2>/dev/null | sort | xargs stat -f %m 2>/dev/null | md5)
    if [ "$curr" != "$prev" ]; then
        prev=$curr
        stop_dev
        start_dev
    fi
done
