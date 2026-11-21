#!/usr/bin/env bash
# Validates the available virtual X server without requiring MRX binaries.
set -euo pipefail
for c in Xvfb xdpyinfo xmessage xwininfo; do
  command -v "$c" >/dev/null 2>&1 || { echo "Missing test dependency: $c" >&2; exit 2; }
done
TMP="$(mktemp -d "${TMPDIR:-/tmp}/mrx-xserver.XXXXXX")"
PID=""
cleanup() { [[ -n "$PID" ]] && kill "$PID" 2>/dev/null || true; rm -rf "$TMP"; }
trap cleanup EXIT INT TERM
Xvfb -displayfd 3 -screen 0 1920x1080x24 -nolisten tcp -ac >"$TMP/xvfb.log" 2>&1 3>"$TMP/display-number" & PID=$!
for _ in $(seq 1 50); do [[ -s "$TMP/display-number" ]] && break; sleep 0.1; done
[[ -s "$TMP/display-number" ]] || { cat "$TMP/xvfb.log" >&2; exit 1; }
export DISPLAY=":$(cat "$TMP/display-number")"
for _ in $(seq 1 50); do xdpyinfo >/dev/null 2>&1 && break; sleep 0.1; done
xdpyinfo >/dev/null
echo 'PASS: Xvfb server starts at 1920x1080x24'
EXTENSIONS="$(xdpyinfo -queryExtensions)"
grep -qi Composite <<<"$EXTENSIONS" && echo 'PASS: Composite extension advertised' || echo 'WARN: Composite extension missing'
grep -qi GLX <<<"$EXTENSIONS" && echo 'PASS: GLX extension advertised (software Xvfb only)' || echo 'WARN: GLX extension missing'
xmessage -title 'MRX X11 probe' -buttons Close:0 'MRX X11 probe' >"$TMP/xmessage.log" 2>&1 & XPID=$!
for _ in $(seq 1 30); do xwininfo -root -tree 2>/dev/null | grep -q 'MRX X11 probe' && break; sleep 0.1; done
if ! xwininfo -root -tree | grep 'MRX X11 probe' >/dev/null; then
  echo 'FAIL: X11 probe client did not appear in the root window tree' >&2
  cat "$TMP/xmessage.log" >&2 || true
  cat "$TMP/xvfb.log" >&2 || true
  xwininfo -root -tree >&2 || true
  exit 1
fi
echo 'PASS: X11 client creates a mapped window'
kill "$XPID" 2>/dev/null || true
echo 'PASS: virtual X11 server probe complete'
echo 'LIMIT: no Rust WM, Quickshell or proprietary NVIDIA driver was tested.'
