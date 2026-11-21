#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/mock" "$TMP/home/.cache/mrx"
cat > "$TMP/mock/mrxctl" <<'MOCK'
#!/usr/bin/env bash
if [[ "${MRX_DIAG_MOCK_FAILURE:-0}" == 1 ]]; then
  echo "mock IPC unavailable" >&2
  exit 1
fi
case "${1:-}" in
  ping) echo pong ;;
  health) echo '{"status":"ok","pid":123,"uptime_seconds":4,"managed_clients":1,"monitors":1,"active_ipc_clients":0}' ;;
  snapshot) echo '{"monitors":[{"name":"MRX-LEFT","x":0,"y":0,"width":1920,"height":1080}],"workspaces":[{"id":1,"monitor":"MRX-LEFT","active":true}],"windows":[{"id":42,"title":"private-window-title"}]}' ;;
  *) exit 2 ;;
esac
MOCK
cat > "$TMP/mock/xrandr" <<'MOCK'
#!/usr/bin/env bash
echo "Monitors: 1"
echo " 0: +*MRX-LEFT 1920/500x1080/280+0+0"
MOCK
cat > "$TMP/mock/qs" <<'MOCK'
#!/usr/bin/env bash
echo "quickshell 0.2.1"
MOCK
cat > "$TMP/mock/picom" <<'MOCK'
#!/usr/bin/env bash
exit 0
MOCK
chmod +x "$TMP/mock/"*
export HOME="$TMP/home"
export XDG_CACHE_HOME="$HOME/.cache"
export DISPLAY=":99"
export XDG_SESSION_TYPE="x11"
export XDG_CURRENT_DESKTOP="MRX"
export PATH="$TMP/mock:$PATH"

REPORT="$TMP/report.txt"
bash "$ROOT/scripts/mrx-diagnose.sh" --reason "test startup failure" --output "$REPORT"
[[ -s "$REPORT" ]]
[[ "$(stat -c '%a' "$REPORT")" == "600" ]]
grep -Fq "Reason: test startup failure" "$REPORT"
grep -Fq "MRX IPC: pong" "$REPORT"
grep -Fq "MRX monitors: MRX-LEFT" "$REPORT"
grep -Fq "MRX managed windows: 1" "$REPORT"
grep -Fq "X11 monitor topology" "$REPORT"
grep -Fq "Privacy" "$REPORT"
if grep -Fq "private-window-title" "$REPORT"; then
  echo "FAIL: diagnostic report leaked a window title" >&2
  exit 1
fi

export MRX_DIAG_MOCK_FAILURE=1
bash "$ROOT/scripts/mrx-diagnose.sh" --output "$TMP/failure-report.txt"
grep -Fq "MRX IPC: unavailable" "$TMP/failure-report.txt"
grep -Fq "MRX snapshot: unavailable" "$TMP/failure-report.txt"
echo "PASS: startup diagnostics collect safe health data, redact titles and handle unavailable IPC"
