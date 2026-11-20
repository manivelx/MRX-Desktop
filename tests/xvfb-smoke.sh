#!/usr/bin/env bash
# Isolated X11 smoke test. Requires built MRX binaries and Xvfb utilities.
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
WM="$ROOT/target/debug/mrx-wm"
CTL="$ROOT/target/debug/mrxctl"
for f in "$WM" "$CTL"; do
  [[ -x "$f" ]] || { echo "Missing $f; run cargo build --workspace first." >&2; exit 2; }
done
for c in Xvfb xdpyinfo xprop xmessage xrandr python3; do
  command -v "$c" >/dev/null 2>&1 || { echo "Missing test dependency: $c" >&2; exit 2; }
done

RUNTIME="$(mktemp -d "${TMPDIR:-/tmp}/mrx-xvfb.XXXXXX")"
export XDG_RUNTIME_DIR="$RUNTIME/runtime"
export MRX_CONFIG_HOME="$RUNTIME/config"
mkdir -p "$XDG_RUNTIME_DIR" "$MRX_CONFIG_HOME"
chmod 700 "$XDG_RUNTIME_DIR"
XVFB_PID=""; WM_PID=""; XTERM_PID=""
cleanup() {
  [[ -n "$XTERM_PID" ]] && kill "$XTERM_PID" 2>/dev/null || true
  [[ -n "$WM_PID" ]] && kill "$WM_PID" 2>/dev/null || true
  [[ -n "$XVFB_PID" ]] && kill "$XVFB_PID" 2>/dev/null || true
  rm -rf "$RUNTIME"
}
trap cleanup EXIT INT TERM

Xvfb -displayfd 3 -screen 0 1920x1080x24 -nolisten tcp -ac >"$RUNTIME/xvfb.log" 2>&1 3>"$RUNTIME/display-number" &
XVFB_PID=$!
for _ in $(seq 1 50); do [[ -s "$RUNTIME/display-number" ]] && break; sleep 0.1; done
[[ -s "$RUNTIME/display-number" ]] || { cat "$RUNTIME/xvfb.log" >&2; exit 1; }
export DISPLAY=":$(cat "$RUNTIME/display-number")"
for _ in $(seq 1 50); do
  if xdpyinfo >/dev/null 2>&1; then break; fi
  sleep 0.1
done
xdpyinfo >/dev/null || { cat "$RUNTIME/xvfb.log" >&2; exit 1; }
echo "PASS: Xvfb 1920x1080 X11 server started"

# Extensions needed for later compositing experiments. Their presence does not
# imply hardware acceleration; Xvfb is a software test server.
EXTENSIONS="$(xdpyinfo -queryExtensions)"
for ext in Composite DAMAGE RANDR XFIXES GLX; do
  if grep -qi "$ext" <<<"$EXTENSIONS"; then echo "PASS: X11 extension advertised: $ext";
  else echo "WARN: X11 extension unavailable in Xvfb: $ext"; fi
done

"$WM" >"$RUNTIME/wm.log" 2>&1 &
WM_PID=$!
for _ in $(seq 1 50); do
  if "$CTL" ping >/dev/null 2>&1; then break; fi
  sleep 0.1
done
"$CTL" ping >/dev/null || { cat "$RUNTIME/wm.log" >&2; exit 1; }
echo "PASS: MRX WM IPC responds"

# Exercise the live RandR notification path by resizing the virtual X screen.
# Unlike xrandr --setmonitor, this changes the active framebuffer geometry and
# emits a RandR ScreenChangeNotify event that every conforming WM must handle.
xrandr --fb 1600x900
SCREEN_UPDATED=0
for _ in $(seq 1 30); do
  "$CTL" snapshot >"$RUNTIME/snapshot.json"
  if python3 - "$RUNTIME/snapshot.json" <<'PY'
import json, sys
with open(sys.argv[1], encoding='utf-8') as f: data = json.load(f)
monitors = data.get('monitors', [])
raise SystemExit(0 if any(m['width'] == 1600 and m['height'] == 900 for m in monitors) else 1)
PY
  then SCREEN_UPDATED=1; break; fi
  sleep 0.1
done
[[ "$SCREEN_UPDATED" == 1 ]] || {
  echo "FAIL: RandR screen-change notification did not refresh the WM monitor snapshot" >&2
  xrandr --query >&2 || true
  cat "$RUNTIME/wm.log" >&2
  exit 1
}
echo "PASS: live RandR screen-change notification refreshes WM monitor geometry"
# Restore the standard test geometry for the remaining smoke checks.
xrandr --fb 1920x1080
xprop -root _NET_NUMBER_OF_DESKTOPS _NET_DESKTOP_NAMES _NET_WORKAREA _NET_CLIENT_LIST_STACKING _NET_SUPPORTED >"$RUNTIME/ewmh.txt"
grep -q '_NET_NUMBER_OF_DESKTOPS' "$RUNTIME/ewmh.txt"
grep -q '_NET_DESKTOP_NAMES' "$RUNTIME/ewmh.txt"
grep -q '_NET_WORKAREA' "$RUNTIME/ewmh.txt"
grep -q '_NET_CLIENT_LIST_STACKING' "$RUNTIME/ewmh.txt"
echo "PASS: EWMH desktop and client-stacking metadata is published"

"$CTL" snapshot >"$RUNTIME/snapshot.json"
python3 - "$RUNTIME/snapshot.json" <<'PY'
import json, sys
with open(sys.argv[1], encoding='utf-8') as f:
    data = json.load(f)
assert data['protocol_version'] == 1, data
assert data['active_workspace'] == 1, data
assert len(data['workspaces']) >= 1, data
assert data.get('monitors'), data
assert all(m['width'] > 0 and m['height'] > 0 for m in data['monitors']), data
assert any(m['focused'] for m in data['monitors']), data
print('PASS: snapshot protocol, initial workspace and RandR monitor geometry')
PY

"$CTL" workspace 2 >/dev/null
sleep 0.2
"$CTL" snapshot >"$RUNTIME/snapshot.json"
python3 - "$RUNTIME/snapshot.json" <<'PY'
import json, sys
with open(sys.argv[1], encoding='utf-8') as f:
    data = json.load(f)
assert data['active_workspace'] == 2, data
print('PASS: workspace switch command')
PY
xprop -root _NET_CURRENT_DESKTOP | grep -q '1'
echo "PASS: EWMH current desktop tracks IPC workspace switch"
python3 - "$RUNTIME/snapshot.json" <<'PY'
import json, sys
with open(sys.argv[1], encoding='utf-8') as f: data=json.load(f)
assert data['active_workspace'] == 2, data
print('PASS: snapshot remains valid after EWMH assertion')
PY

"$CTL" set appearance.gaps 12 >/dev/null
# IPC acknowledges that the action was queued; wait for the event loop to
# apply and persist it instead of racing the asynchronous state transition.
SETTING_APPLIED=0
for _ in $(seq 1 30); do
  "$CTL" snapshot >"$RUNTIME/snapshot.json"
  if python3 - "$RUNTIME/snapshot.json" <<'PY'
import json, sys
with open(sys.argv[1], encoding='utf-8') as f: data = json.load(f)
raise SystemExit(0 if data['settings']['appearance']['gaps'] == 12 else 1)
PY
  then SETTING_APPLIED=1; break; fi
  sleep 0.1
done
[[ "$SETTING_APPLIED" == 1 ]] || { echo "FAIL: setting update was not applied" >&2; cat "$RUNTIME/wm.log" >&2; exit 1; }
echo "PASS: live setting update and persistence"

# Start a client on the currently active workspace after the WM has completed
# workspace and settings operations. Keep it alive so ICCCM close can be tested.
xmessage -title 'MRX Xvfb test client' -buttons Close:0 -default Close 'MRX smoke test' >"$RUNTIME/xterm.log" 2>&1 &
XTERM_PID=$!
sleep 0.5

# Verify a real mapped client is tracked and can be asked to close through ICCCM.
for _ in $(seq 1 30); do
  "$CTL" snapshot >"$RUNTIME/snapshot.json"
  if python3 - "$RUNTIME/snapshot.json" <<'PY'
import json, sys
with open(sys.argv[1], encoding='utf-8') as f: data=json.load(f)
raise SystemExit(0 if data.get('windows') else 1)
PY
  then break; fi
  sleep 0.1
done
WINDOW_ID="$(python3 - "$RUNTIME/snapshot.json" <<'PY'
import json, sys
with open(sys.argv[1], encoding='utf-8') as f: data=json.load(f)
print(data['windows'][0]['id'] if data.get('windows') else '')
PY
)"
[[ -n "$WINDOW_ID" ]] || { echo "FAIL: xterm was not managed" >&2; echo '--- X11 client log ---' >&2; cat "$RUNTIME/xterm.log" >&2; echo '--- root tree ---' >&2; xwininfo -root -tree >&2 || true; echo '--- WM log ---' >&2; cat "$RUNTIME/wm.log" >&2; exit 1; }
python3 - "$RUNTIME/snapshot.json" "$WINDOW_ID" <<'PY'
import json, sys
with open(sys.argv[1], encoding='utf-8') as f: data = json.load(f)
window = next(w for w in data['windows'] if str(w['id']) == sys.argv[2])
assert window['monitor'] in {m['name'] for m in data['monitors']}, data
print('PASS: managed window has a live monitor assignment')
PY
"$CTL" close "$WINDOW_ID" >/dev/null
for _ in $(seq 1 30); do
  "$CTL" snapshot >"$RUNTIME/snapshot.json"
  if ! python3 - "$RUNTIME/snapshot.json" "$WINDOW_ID" <<'PY'
import json, sys
with open(sys.argv[1], encoding='utf-8') as f: data=json.load(f)
raise SystemExit(0 if any(str(w['id']) == sys.argv[2] for w in data.get('windows', [])) else 1)
PY
  then break; fi
  sleep 0.1
done
"$CTL" snapshot >"$RUNTIME/snapshot.json"
python3 - "$RUNTIME/snapshot.json" "$WINDOW_ID" <<'PY'
import json, sys
with open(sys.argv[1], encoding='utf-8') as f: data=json.load(f)
assert all(str(w['id']) != sys.argv[2] for w in data.get('windows', [])), data
print('PASS: ICCCM close request removed the managed client')
PY

echo "PASS: isolated Xvfb smoke test complete"
echo "NOTE: this does not validate Quickshell rendering, GLX acceleration or NVIDIA hardware."
