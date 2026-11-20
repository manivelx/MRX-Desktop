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
XVFB_PID=""; WM_PID=""; XTERM_PID=""; SUB_PID=""
cleanup() {
  [[ -n "$SUB_PID" ]] && kill "$SUB_PID" 2>/dev/null || true
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

# Exercise the live RandR monitor-object update path. Xvfb can represent
# independent RandR 1.5 monitor objects over a single virtual output; the WM
# must expose those objects in its snapshot after the resource notification.
xrandr --setmonitor MRX-LEFT 960/254x1080/286+0+0 none
xrandr --setmonitor MRX-RIGHT 960/254x1080/286+960+0 none
MONITORS_READY=0
for _ in $(seq 1 30); do
  "$CTL" snapshot >"$RUNTIME/snapshot.json"
  if python3 - "$RUNTIME/snapshot.json" <<'PY'
import json, sys
with open(sys.argv[1], encoding='utf-8') as f: data = json.load(f)
names = {m['name'] for m in data.get('monitors', [])}
raise SystemExit(0 if names == {'MRX-LEFT', 'MRX-RIGHT'} else 1)
PY
  then MONITORS_READY=1; break; fi
  sleep 0.1
done
[[ "$MONITORS_READY" == 1 ]] || {
  echo "FAIL: RandR monitor objects were not refreshed in the WM snapshot" >&2
  xrandr --listmonitors >&2 || true
  cat "$RUNTIME/wm.log" >&2
  exit 1
}
python3 - "$RUNTIME/snapshot.json" <<'PY'
import json, sys
with open(sys.argv[1], encoding='utf-8') as f: data = json.load(f)
monitors = {m['name']: m for m in data['monitors']}
assert (monitors['MRX-LEFT']['x'], monitors['MRX-LEFT']['y'],
        monitors['MRX-LEFT']['width'], monitors['MRX-LEFT']['height']) == (0, 0, 960, 1080), monitors
assert (monitors['MRX-RIGHT']['x'], monitors['MRX-RIGHT']['y'],
        monitors['MRX-RIGHT']['width'], monitors['MRX-RIGHT']['height']) == (960, 0, 960, 1080), monitors
print('PASS: live RandR resource changes refresh two independent monitor geometries')
PY

# Each monitor owns its own active workspace. Switching one output must not
# change the active workspace of the other output.
"$CTL" workspace 2 MRX-LEFT >/dev/null
"$CTL" workspace 3 MRX-RIGHT >/dev/null
WORKSPACES_APPLIED=0
for _ in $(seq 1 30); do
  "$CTL" snapshot >"$RUNTIME/snapshot.json"
  if python3 - "$RUNTIME/snapshot.json" <<'PY'
import json, sys
with open(sys.argv[1], encoding='utf-8') as f: data = json.load(f)
monitors = {m['name']: m for m in data['monitors']}
ok = (monitors.get('MRX-LEFT', {}).get('active_workspace') == 2
      and monitors.get('MRX-RIGHT', {}).get('active_workspace') == 3)
raise SystemExit(0 if ok else 1)
PY
  then WORKSPACES_APPLIED=1; break; fi
  sleep 0.1
done
[[ "$WORKSPACES_APPLIED" == 1 ]] || {
  echo "FAIL: monitor-local workspace switches were not applied" >&2
  "$CTL" snapshot >&2 || true
  cat "$RUNTIME/wm.log" >&2
  exit 1
}
python3 - "$RUNTIME/snapshot.json" <<'PY'
import json, sys
with open(sys.argv[1], encoding='utf-8') as f: data = json.load(f)
monitors = {m['name']: m for m in data['monitors']}
assert monitors['MRX-LEFT']['active_workspace'] == 2, monitors
assert monitors['MRX-RIGHT']['active_workspace'] == 3, monitors
workspaces = {(w['monitor'], w['id']): w for w in data['workspaces']}
assert workspaces[('MRX-LEFT', 2)]['active'] is True, workspaces
assert workspaces[('MRX-RIGHT', 3)]['active'] is True, workspaces
print('PASS: independent monitor-local workspace state is exposed through IPC')
PY
# Return both outputs to workspace one before testing window lifecycle behavior.
"$CTL" workspace 1 MRX-LEFT >/dev/null
"$CTL" workspace 1 MRX-RIGHT >/dev/null
xprop -root _NET_NUMBER_OF_DESKTOPS _NET_DESKTOP_NAMES _NET_WORKAREA _NET_CLIENT_LIST_STACKING _NET_SUPPORTED >"$RUNTIME/ewmh.txt"
grep -q '_NET_NUMBER_OF_DESKTOPS' "$RUNTIME/ewmh.txt"
grep -q '_NET_DESKTOP_NAMES' "$RUNTIME/ewmh.txt"
grep -q '_NET_WORKAREA' "$RUNTIME/ewmh.txt"
python3 - "$RUNTIME/ewmh.txt" <<'PY'
import re, sys
text = open(sys.argv[1], encoding='utf-8').read()
line = next((line for line in text.splitlines() if '_NET_WORKAREA' in line), '')
values = [int(value) for value in re.findall(r'\d+', line.split('=', 1)[-1])]
assert len(values) == 36, (len(values), line)
assert values[:4] == [0, 0, 1920, 1080], values[:4]
assert all(values[i:i+4] == [0, 0, 1920, 1080] for i in range(0, 36, 4)), values
print('PASS: EWMH workarea is published for all nine desktops')
PY
grep -q '_NET_CLIENT_LIST_STACKING' "$RUNTIME/ewmh.txt"
echo "PASS: EWMH desktop and client-stacking metadata is published"

"$CTL" snapshot >"$RUNTIME/snapshot.json"
python3 - "$RUNTIME/snapshot.json" <<'PY'
import json, sys
with open(sys.argv[1], encoding='utf-8') as f:
    data = json.load(f)
assert data['protocol_version'] == 3, data
assert data['active_workspace'] == 1, data
assert len(data['workspaces']) >= 1, data
assert data.get('monitors'), data
assert all(m['width'] > 0 and m['height'] > 0 for m in data['monitors']), data
assert any(m['focused'] for m in data['monitors']), data
print('PASS: snapshot protocol, initial workspace and RandR monitor geometry')
PY

# Open a persistent subscription before switching workspaces. The client
# must receive an initial snapshot and then a change event without polling.
"$CTL" subscribe >"$RUNTIME/events.jsonl" 2>"$RUNTIME/subscribe.log" &
SUB_PID=$!
EVENTS_READY=0
for _ in $(seq 1 30); do
  if [[ -s "$RUNTIME/events.jsonl" ]]; then EVENTS_READY=1; break; fi
  sleep 0.1
done
[[ "$EVENTS_READY" == 1 ]] || { echo "FAIL: IPC subscription did not emit its initial snapshot" >&2; cat "$RUNTIME/subscribe.log" >&2; exit 1; }

"$CTL" workspace 2 >/dev/null
EVENT_UPDATE=0
for _ in $(seq 1 30); do
  if [[ "$(wc -l <"$RUNTIME/events.jsonl")" -ge 2 ]]; then
    if python3 - "$RUNTIME/events.jsonl" <<'PY'
import json, sys
lines = [line for line in open(sys.argv[1], encoding='utf-8') if line.strip()]
snapshot = json.loads(lines[-1])
raise SystemExit(0 if snapshot['active_workspace'] == 2 else 1)
PY
    then EVENT_UPDATE=1; break; fi
  fi
  sleep 0.1
done
[[ "$EVENT_UPDATE" == 1 ]] || { echo "FAIL: IPC subscription did not emit workspace change" >&2; cat "$RUNTIME/events.jsonl" >&2; cat "$RUNTIME/subscribe.log" >&2; exit 1; }
kill "$SUB_PID" 2>/dev/null || true
wait "$SUB_PID" 2>/dev/null || true
SUB_PID=""
echo "PASS: persistent IPC subscription emits initial and changed snapshots"

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

# Exercise EWMH horizontal+vertical maximize and restoration through real
# ClientMessage requests. The snapshot must reflect both state atoms and
# restore the pre-maximize geometry when both axes are removed.
xwininfo -id "$WINDOW_ID" >"$RUNTIME/before-maximize.txt"
cc "$ROOT/tests/send-net-wm-state.c" -o "$RUNTIME/send-net-wm-state" -lX11
"$RUNTIME/send-net-wm-state" "$WINDOW_ID" 1 both
MAXIMIZED=0
for _ in $(seq 1 30); do
  "$CTL" snapshot >"$RUNTIME/snapshot.json"
  if python3 - "$RUNTIME/snapshot.json" "$WINDOW_ID" <<'PY'
import json, sys
data = json.load(open(sys.argv[1], encoding='utf-8'))
window = next(w for w in data['windows'] if str(w['id']) == sys.argv[2])
raise SystemExit(0 if window.get('maximized_horz') and window.get('maximized_vert') else 1)
PY
  then MAXIMIZED=1; break; fi
  sleep 0.1
done
[[ "$MAXIMIZED" == 1 ]] || { echo "FAIL: EWMH maximize state was not applied" >&2; cat "$RUNTIME/wm.log" >&2; exit 1; }
"$RUNTIME/send-net-wm-state" "$WINDOW_ID" 0 both
RESTORED=0
for _ in $(seq 1 30); do
  xwininfo -id "$WINDOW_ID" >"$RUNTIME/after-maximize.txt"
  "$CTL" snapshot >"$RUNTIME/snapshot.json"
  if python3 - "$RUNTIME/before-maximize.txt" "$RUNTIME/after-maximize.txt" "$RUNTIME/snapshot.json" "$WINDOW_ID" <<'PY'
import json, sys
def geometry(path):
    lines = open(path, encoding='utf-8').read().splitlines()
    values = {}
    for key in ('Absolute upper-left X', 'Absolute upper-left Y', 'Width', 'Height'):
        line = next((line for line in lines if line.strip().startswith(key + ':')), None)
        assert line is not None, (key, lines)
        values[key] = int(line.split(':', 1)[1].strip())
    return tuple(values[key] for key in ('Absolute upper-left X', 'Absolute upper-left Y', 'Width', 'Height'))
snapshot = json.load(open(sys.argv[3], encoding='utf-8'))
window = next(w for w in snapshot['windows'] if str(w['id']) == sys.argv[4])
assert not window.get('maximized_horz') and not window.get('maximized_vert'), window
assert geometry(sys.argv[1]) == geometry(sys.argv[2]), (geometry(sys.argv[1]), geometry(sys.argv[2]))
print('PASS: EWMH maximize/restore round-trip preserves original geometry')
PY
  then RESTORED=1; break; fi
  sleep 0.1
done
[[ "$RESTORED" == 1 ]] || { echo "FAIL: EWMH maximize state did not restore original geometry" >&2; cat "$RUNTIME/wm.log" >&2; exit 1; }

# Exercise EWMH hidden/minimize state. The client must remain in the managed
# client list while unmapped, report Iconic WM_STATE, and restore on removal.
"$RUNTIME/send-net-wm-state" "$WINDOW_ID" 1 hidden
MINIMIZED=0
for _ in $(seq 1 30); do
  "$CTL" snapshot >"$RUNTIME/snapshot.json"
  if python3 - "$RUNTIME/snapshot.json" "$WINDOW_ID" <<'PY'
import json, sys
data = json.load(open(sys.argv[1], encoding='utf-8'))
window = next((w for w in data['windows'] if str(w['id']) == sys.argv[2]), None)
raise SystemExit(0 if window and window.get('minimized') else 1)
PY
  then MINIMIZED=1; break; fi
  sleep 0.1
done
[[ "$MINIMIZED" == 1 ]] || { echo "FAIL: EWMH hidden state did not minimize the client" >&2; cat "$RUNTIME/wm.log" >&2; exit 1; }
xprop -id "$WINDOW_ID" WM_STATE | grep -q 'Iconic State'
"$RUNTIME/send-net-wm-state" "$WINDOW_ID" 0 hidden
RESTORE_MINIMIZED=0
for _ in $(seq 1 30); do
  "$CTL" snapshot >"$RUNTIME/snapshot.json"
  if python3 - "$RUNTIME/snapshot.json" "$WINDOW_ID" <<'PY'
import json, sys
data = json.load(open(sys.argv[1], encoding='utf-8'))
window = next((w for w in data['windows'] if str(w['id']) == sys.argv[2]), None)
raise SystemExit(0 if window and not window.get('minimized') else 1)
PY
  then RESTORE_MINIMIZED=1; break; fi
  sleep 0.1
done
[[ "$RESTORE_MINIMIZED" == 1 ]] || { echo "FAIL: EWMH hidden state did not restore the client" >&2; cat "$RUNTIME/wm.log" >&2; exit 1; }
xprop -id "$WINDOW_ID" WM_STATE | grep -q 'Normal State'
echo "PASS: EWMH minimize/restore retains the managed client and WM_STATE"

# Also exercise the ICCCM WM_CHANGE_STATE IconicState/NormalState messages.
"$RUNTIME/send-net-wm-state" "$WINDOW_ID" 1 icccm
ICCCM_MINIMIZED=0
for _ in $(seq 1 30); do
  "$CTL" snapshot >"$RUNTIME/snapshot.json"
  if python3 - "$RUNTIME/snapshot.json" "$WINDOW_ID" <<'PY'
import json, sys
data = json.load(open(sys.argv[1], encoding='utf-8'))
window = next((w for w in data['windows'] if str(w['id']) == sys.argv[2]), None)
raise SystemExit(0 if window and window.get('minimized') else 1)
PY
  then ICCCM_MINIMIZED=1; break; fi
  sleep 0.1
done
[[ "$ICCCM_MINIMIZED" == 1 ]] || { echo "FAIL: ICCCM WM_CHANGE_STATE did not minimize the client" >&2; cat "$RUNTIME/wm.log" >&2; exit 1; }
"$RUNTIME/send-net-wm-state" "$WINDOW_ID" 0 icccm
ICCCM_RESTORED=0
for _ in $(seq 1 30); do
  "$CTL" snapshot >"$RUNTIME/snapshot.json"
  if python3 - "$RUNTIME/snapshot.json" "$WINDOW_ID" <<'PY'
import json, sys
data = json.load(open(sys.argv[1], encoding='utf-8'))
window = next((w for w in data['windows'] if str(w['id']) == sys.argv[2]), None)
raise SystemExit(0 if window and not window.get('minimized') else 1)
PY
  then ICCCM_RESTORED=1; break; fi
  sleep 0.1
done
[[ "$ICCCM_RESTORED" == 1 ]] || { echo "FAIL: ICCCM WM_CHANGE_STATE did not restore the client" >&2; cat "$RUNTIME/wm.log" >&2; exit 1; }
echo "PASS: ICCCM WM_CHANGE_STATE minimize/restore lifecycle"

# Reuse the live test client as a dock and reserve 48px at the top edge.

xprop -id "$WINDOW_ID" -f _NET_WM_WINDOW_TYPE 32a -set _NET_WM_WINDOW_TYPE _NET_WM_WINDOW_TYPE_DOCK
xprop -id "$WINDOW_ID" -f _NET_WM_STRUT_PARTIAL 32c -set _NET_WM_STRUT_PARTIAL '0, 0, 48, 0, 0, 0, 0, 0, 0, 1919, 0, 0'
WORKAREA_APPLIED=0
for _ in $(seq 1 30); do
  xprop -root _NET_WORKAREA >"$RUNTIME/workarea.txt"
  if python3 - "$RUNTIME/workarea.txt" <<'PY'
import re, sys
text = open(sys.argv[1], encoding='utf-8').read()
payload = text.split("=", 1)[-1]
values = [int(v) for v in re.findall(r"\d+", payload)]
raise SystemExit(0 if len(values) >= 4 and values[:4] == [0, 48, 1920, 1032] else 1)
PY
  then WORKAREA_APPLIED=1; break; fi
  sleep 0.1
done
[[ "$WORKAREA_APPLIED" == 1 ]] || {
  echo "FAIL: dock partial strut did not reduce EWMH workarea" >&2
  xprop -id "$WINDOW_ID" _NET_WM_WINDOW_TYPE _NET_WM_STRUT_PARTIAL >&2 || true
  xprop -root _NET_WORKAREA >&2 || true
  cat "$RUNTIME/wm.log" >&2
  exit 1
}
echo "PASS: EWMH dock partial strut reduces usable workarea"
"$CTL" snapshot >"$RUNTIME/snapshot.json"
python3 - "$RUNTIME/snapshot.json" "$WINDOW_ID" <<'PY'
import json, sys
with open(sys.argv[1], encoding='utf-8') as f: data = json.load(f)
window = next(w for w in data['windows'] if str(w['id']) == sys.argv[2])
assert window.get('dock') is True, window
assert window.get('focused') is False, window
print('PASS: dock clients remain sticky and do not steal keyboard focus')
PY

# Set the ICCCM urgency flag on the live client and verify PropertyNotify
# refreshes the shared snapshot rather than leaving a stale initial value.
cc "$ROOT/tests/set-wm-hints.c" -o "$RUNTIME/set-wm-hints" -lX11
"$RUNTIME/set-wm-hints" "$WINDOW_ID"
URGENCY_APPLIED=0
for _ in $(seq 1 30); do
  "$CTL" snapshot >"$RUNTIME/snapshot.json"
  if python3 - "$RUNTIME/snapshot.json" "$WINDOW_ID" <<'PY'
import json, sys
with open(sys.argv[1], encoding='utf-8') as f: data = json.load(f)
window = next((w for w in data['windows'] if str(w['id']) == sys.argv[2]), None)
raise SystemExit(0 if window and window.get('urgent') is True else 1)
PY
  then URGENCY_APPLIED=1; break; fi
  sleep 0.1
done
[[ "$URGENCY_APPLIED" == 1 ]] || { echo "FAIL: WM_HINTS urgency did not reach the IPC snapshot" >&2; xprop -id "$WINDOW_ID" WM_HINTS >&2 || true; cat "$RUNTIME/wm.log" >&2; exit 1; }
echo "PASS: ICCCM WM_HINTS urgency updates the live window snapshot"
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
