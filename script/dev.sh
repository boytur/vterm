#!/bin/sh
# Fast dev loop for vterm: rebuilds and relaunches the app whenever a source
# file changes (GPUI apps can't hot-swap code at runtime, so we relaunch).
#
# Uses cargo-watch if available; otherwise falls back to a portable
# find-based watcher. Debug profile keeps rebuilds fast.
set -e

cd "$(dirname "$0")/.."

if command -v cargo-watch >/dev/null 2>&1; then
    exec cargo watch -q -c -x run
fi

echo "tip: 'cargo install cargo-watch' gives instant, debounced watching" >&2

prev=$(find Cargo.toml crates build.rs -type f 2>/dev/null | sort | xargs stat -f %m 2>/dev/null | md5)
while true; do
    sleep 1
    curr=$(find Cargo.toml crates build.rs -type f 2>/dev/null | sort | xargs stat -f %m 2>/dev/null | md5)
    if [ "$curr" != "$prev" ]; then
        prev=$curr
        pkill -x vterm 2>/dev/null || true
        cargo run
    fi
done
