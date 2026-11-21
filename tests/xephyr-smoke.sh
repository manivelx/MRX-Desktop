#!/usr/bin/env bash
# Run the complete MRX X11 integration suite inside a nested Xephyr server.
# The host Xvfb is only the display surface for Xephyr; all test clients and
# the MRX WM connect to the nested X server.
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT/tests/lib/xvfb-fonts.sh"
FONT_PATH="$(mrx_test_font_path)"
for c in Xvfb Xephyr xdpyinfo; do
  command -v "$c" >/dev/null 2>&1 || { echo "Missing test dependency: $c" >&2; exit 2; }
done
RUNTIME="$(mktemp -d "${TMPDIR:-/tmp}/mrx-xephyr.XXXXXX")"
HOST_PID=""
NESTED_PID=""
cleanup() {
  [[ -n "$NESTED_PID" ]] && kill "$NESTED_PID" 2>/dev/null || true
  [[ -n "$HOST_PID" ]] && kill "$HOST_PID" 2>/dev/null || true
  [[ -n "$NESTED_PID" ]] && wait "$NESTED_PID" 2>/dev/null || true
  [[ -n "$HOST_PID" ]] && wait "$HOST_PID" 2>/dev/null || true
  rm -rf "$RUNTIME"
}
trap cleanup EXIT INT TERM

Xvfb -displayfd 3 -screen 0 1920x1080x24 -fp "$FONT_PATH" -nolisten tcp -ac >"$RUNTIME/host-xvfb.log" 2>&1 3>"$RUNTIME/host-display" &
HOST_PID=$!
for _ in $(seq 1 50); do [[ -s "$RUNTIME/host-display" ]] && break; sleep 0.1; done
[[ -s "$RUNTIME/host-display" ]] || { cat "$RUNTIME/host-xvfb.log" >&2; exit 1; }
export DISPLAY=":$(cat "$RUNTIME/host-display")"
for _ in $(seq 1 50); do xdpyinfo >/dev/null 2>&1 && break; sleep 0.1; done
xdpyinfo >/dev/null || { cat "$RUNTIME/host-xvfb.log" >&2; exit 1; }

NESTED_DISPLAY=""
for candidate in $(seq 100 199); do
  if ! DISPLAY=":$candidate" xdpyinfo >/dev/null 2>&1; then
    NESTED_DISPLAY=":$candidate"
    break
  fi
done
[[ -n "$NESTED_DISPLAY" ]] || { echo "No free nested X display found" >&2; exit 1; }
Xephyr "$NESTED_DISPLAY" -screen 1920x1080 -fp "$FONT_PATH" -nolisten tcp -ac +extension RANDR +extension Composite +extension DAMAGE >"$RUNTIME/xephyr.log" 2>&1 &
NESTED_PID=$!
READY=0
for _ in $(seq 1 100); do
  if DISPLAY="$NESTED_DISPLAY" xdpyinfo >/dev/null 2>&1; then READY=1; break; fi
  if ! kill -0 "$NESTED_PID" 2>/dev/null; then break; fi
  sleep 0.1
done
[[ "$READY" == "1" ]] || { cat "$RUNTIME/xephyr.log" >&2; exit 1; }
echo "PASS: nested Xephyr X11 server started on $NESTED_DISPLAY"
DISPLAY="$NESTED_DISPLAY" MRX_USE_EXISTING_DISPLAY=1 bash "$ROOT/tests/xvfb-smoke.sh"
echo "PASS: complete MRX integration suite passed inside Xephyr"
