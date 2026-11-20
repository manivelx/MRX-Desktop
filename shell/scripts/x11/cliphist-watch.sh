#!/usr/bin/env bash
set -euo pipefail
last_hash=""
tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT
while :; do
    targets="$(xclip -selection clipboard -o -t TARGETS 2>/dev/null || true)"
    : > "$tmp"
    if grep -qi 'image/png' <<<"$targets"; then
        xclip -selection clipboard -o -t image/png > "$tmp" 2>/dev/null || true
    else
        xclip -selection clipboard -o -t UTF8_STRING > "$tmp" 2>/dev/null || true
    fi
    if [[ -s "$tmp" ]]; then
        hash="$(sha256sum "$tmp" | awk '{print $1}')"
        if [[ "$hash" != "$last_hash" ]]; then cliphist store < "$tmp" || true; last_hash="$hash"; fi
    fi
    sleep 0.8
done
