#!/bin/bash
# Memory that stays allocated at exit, after a short and a long run of the same load: what grows with time is a leak,
# the rest is memory a process keeps for its whole life (Qt's types, fonts, mpv's scripts). Needs heaptrack.
# Usage: leak-check.sh <out-dir> [short-secs] [long-secs]
set -euo pipefail
out=$(realpath -m "$1"); short=${2:-60}; long=${3:-300}
root=$(cd "$(dirname "$0")/../../.." && pwd)
target=${CARGO_TARGET_DIR:-$root/target}
mkdir -p "$out"
cd "$root"
cargo build -p rela_desktop
test_bin=$(cargo test -p rela-mpv --test mpv_deck --no-run --message-format=json 2>/dev/null \
    | sed -n 's/.*"executable":"\([^"]*\)".*/\1/p' | tail -1)

# Our own crates' allocations still held at exit, in bytes (heaptrack_print's "leaked" per top-level allocator)
ours() {
    heaptrack_print -f "$1" --print-leaks 1 --print-peaks 0 --print-allocators 0 --print-temporary 0 \
        --print-histogram /dev/null --print-flamegraph /dev/null -n 1000 -s 0 2>/dev/null \
        | awk '/leaked over [0-9]+ calls from$/ {size=$1; getline; if ($1 ~ /re-la|libmpv2/) print size}' \
        | awk '{v=$1; m=1; if (v ~ /K$/) m=1e3; if (v ~ /M$/) m=1e6; if (v ~ /G$/) m=1e9; sub(/[KMGB]+$/, "", v); s+=v*m} END {printf "%d\n", s}'
}
total() {
    heaptrack_print -f "$1" --print-leaks 0 --print-peaks 0 --print-allocators 0 --print-temporary 0 \
        --print-histogram /dev/null --print-flamegraph /dev/null 2>/dev/null | sed -n 's/^total memory leaked: //p'
}

soak() { # $1 = seconds
    RELA_SOAK_SECS=$1 heaptrack --record-only -o "$out/soak-$1" "$test_bin" --ignored --exact soak --nocapture \
        > "$out/soak-$1.log" 2>&1
}
app() { # $1 = seconds; its own dirs, no session bus, no sound, no window
    local t; t=$(mktemp -d /tmp/re-la-leak.XXXX)
    mkdir -p "$t"/{run,cfg,cache,data}
    env XDG_RUNTIME_DIR="$t/run" XDG_CONFIG_HOME="$t/cfg" XDG_CACHE_HOME="$t/cache" XDG_DATA_HOME="$t/data" \
        QT_QPA_PLATFORM=offscreen RELA_AO=null DBUS_SESSION_BUS_ADDRESS="unix:path=$t/nobus" \
        heaptrack --record-only -o "$out/app-$1" "$target/debug/re-la" --demo > "$out/app-$1.log" 2>&1 &
    local h=$!
    sleep "$1"
    pkill -TERM -f "^$target/debug/re-la --demo" || true
    wait $h || true
    rm -rf "$t"
}

for s in "$short" "$long"; do soak "$s"; app "$s"; done
printf '%-10s %-12s %s\n' run total-at-exit re-la-bytes
for f in "$out"/soak-"$short" "$out"/soak-"$long" "$out"/app-"$short" "$out"/app-"$long"; do
    printf '%-10s %-12s %s\n' "$(basename "$f")" "$(total "$f.zst")" "$(ours "$f.zst")"
done
grep -h "soak:" "$out"/soak-*.log || true
