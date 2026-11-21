#!/usr/bin/env bash
# Launch the installed MRX session under a disposable Xvfb + DBus session.
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
DISPLAY_NUMBER="${MRX_TEST_DISPLAY:-97}"
SESSION_PID=""
XVFB_PID=""

fail() {
  echo "FAIL: $*" >&2
  [[ -f "$TMP/session.log" ]] && tail -n 100 "$TMP/session.log" >&2 || true
  if [[ -n "${XDG_RUNTIME_DIR:-}" && -d "$XDG_RUNTIME_DIR/quickshell" ]]; then
    find "$XDG_RUNTIME_DIR/quickshell" -type f -name 'log.qslog' -exec sh -c 'echo "--- Quickshell log: $1"; tail -n 100 "$1"' _ {} \; >&2 || true
  fi
  exit 1
}
stop_process_tree() {
  local signal="$1" parent="$2" child
  while IFS= read -r child; do
    [[ -n "$child" ]] && stop_process_tree "$signal" "$child"
  done < <(pgrep -P "$parent" || true)
  kill -"$signal" "$parent" 2>/dev/null || true
}
process_tree_has_name() {
  local parent="$1" wanted="$2" child child_name
  while IFS= read -r child; do
    [[ -n "$child" ]] || continue
    child_name="$(cat "/proc/$child/comm" 2>/dev/null || true)"
    if [[ "$child_name" == "$wanted" ]] || process_tree_has_name "$child" "$wanted"; then
      return 0
    fi
  done < <(pgrep -P "$parent" || true)
  return 1
}
cleanup() {
  # Stop the session and every descendant before removing the temporary HOME.
  # This avoids killing unrelated Quickshell/Picom processes on the host.
  if [[ -n "$SESSION_PID" ]]; then
    stop_process_tree TERM "$SESSION_PID"
    for _ in $(seq 1 30); do
      kill -0 "$SESSION_PID" 2>/dev/null || break
      sleep 0.1
    done
    if kill -0 "$SESSION_PID" 2>/dev/null; then
      stop_process_tree KILL "$SESSION_PID"
    fi
    wait "$SESSION_PID" 2>/dev/null || true
  fi
  if [[ -n "$XVFB_PID" ]]; then
    kill "$XVFB_PID" 2>/dev/null || true
    wait "$XVFB_PID" 2>/dev/null || true
  fi
  rm -rf "$TMP" 2>/dev/null || {
    sleep 0.2
    rm -rf "$TMP" 2>/dev/null || true
  }
}
trap cleanup EXIT INT TERM

command -v qs >/dev/null 2>&1 || fail "Quickshell (qs) is required for this runtime smoke test"
command -v Xvfb >/dev/null 2>&1 || fail "Xvfb is required for this runtime smoke test"
command -v dbus-run-session >/dev/null 2>&1 || fail "dbus-run-session is required for this runtime smoke test"
command -v pgrep >/dev/null 2>&1 || fail "procps-ng is required for this runtime smoke test"
[[ -x "$ROOT/target/debug/mrx-wm" && -x "$ROOT/target/debug/mrxctl" ]] || fail "debug MRX binaries are missing; run cargo build --workspace first"

export HOME="$TMP/home"
export USER="${USER:-root}"
export XDG_CONFIG_HOME="$HOME/.config"
export XDG_DATA_HOME="$HOME/.local/share"
export XDG_CACHE_HOME="$HOME/.cache"
export XDG_RUNTIME_DIR="$TMP/runtime"
export XDG_CURRENT_DESKTOP=MRX
export XDG_SESSION_DESKTOP=mrx
export XDG_SESSION_TYPE=x11
export QT_QPA_PLATFORM=xcb
export QT_QUICK_BACKEND=software
export QT_AUTO_SCREEN_SCALE_FACTOR=0
export MRX_SESSION=1
export PATH="$HOME/.local/bin:$PATH"
mkdir -p "$HOME" "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"

Xvfb ":$DISPLAY_NUMBER" -screen 0 1600x900x24 -nolisten tcp -ac >"$TMP/xvfb.log" 2>&1 &
XVFB_PID=$!
export DISPLAY=":$DISPLAY_NUMBER"
for _ in $(seq 1 50); do
  if xdpyinfo -display "$DISPLAY" >/dev/null 2>&1; then break; fi
  sleep 0.1
done
xdpyinfo -display "$DISPLAY" >/dev/null 2>&1 || fail "Xvfb did not become ready"

MRX_BIN_DIR="$ROOT/target/debug" bash "$ROOT/install.sh" >"$TMP/install.log" 2>&1 || {
  cat "$TMP/install.log" >&2
  fail "isolated MRX installation failed"
}

dbus-run-session -- "$HOME/.local/bin/mrx-session" >"$TMP/session.log" 2>&1 &
SESSION_PID=$!

READY=0
for _ in $(seq 1 120); do
  if ! kill -0 "$SESSION_PID" 2>/dev/null; then
    fail "MRX session exited before Quickshell became ready"
  fi
  if "$HOME/.local/bin/mrxctl" ping >/dev/null 2>&1 && { process_tree_has_name "$SESSION_PID" qs >/dev/null 2>&1 || process_tree_has_name "$SESSION_PID" quickshell >/dev/null 2>&1; }; then
    READY=1
    break
  fi
  sleep 0.25
done
[[ "$READY" == 1 ]] || fail "MRX WM and Quickshell did not both become ready within 30 seconds"

sleep 3
kill -0 "$SESSION_PID" 2>/dev/null || fail "MRX session exited after startup"
{ pgrep -g "$SESSION_PID" -x qs >/dev/null 2>&1 || pgrep -g "$SESSION_PID" -x quickshell >/dev/null 2>&1; } || fail "Quickshell exited after startup"
"$HOME/.local/bin/mrxctl" health >/dev/null 2>&1 || fail "MRX IPC health check failed after Quickshell startup"

if grep -Eiq 'QQmlApplicationEngine failed|Failed to load root component|module "[^"]+" is not installed|MRX session startup failed' "$TMP/session.log"; then
  fail "Quickshell reported a fatal QML startup error"
fi

echo "PASS: Quickshell loaded the MRX configuration under Xvfb, connected to MRX IPC and remained alive"
