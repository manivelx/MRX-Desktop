#!/usr/bin/env bash
set -u
missing=0
for c in cargo rustc qs; do
  if command -v "$c" >/dev/null 2>&1; then
    printf 'OK   %-14s %s\n' "$c" "$(command -v "$c")"
  else
    printf 'MISS %-14s\n' "$c"
    missing=1
  fi
done

# Required by the isolated Xvfb integration smoke test.
for c in Xvfb Xephyr xdpyinfo xprop xmessage xwininfo xrandr python3 jq xdotool slop ffmpeg; do
  if command -v "$c" >/dev/null 2>&1; then
    printf 'OK   %-14s %s\n' "$c" "$(command -v "$c")"
  else
    printf 'MISS %-14s\n' "$c"
    missing=1
  fi
done

for c in picom notify-send loginctl; do
  if command -v "$c" >/dev/null 2>&1; then
    printf 'OPTIONAL %-10s %s\n' "$c" "$(command -v "$c")"
  else
    printf 'OPTIONAL %-10s missing\n' "$c"
  fi
done
for f in /usr/include/X11/Xlib.h /usr/include/xcb/xcb.h; do
  [[ -e "$f" ]] && echo "OK   $f" || echo "INFO optional header missing: $f"
done
exit "$missing"
