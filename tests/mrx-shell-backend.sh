#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "\${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d "\${TMPDIR:-/tmp}/mrx-shell-backend.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/bin"

cat >"$TMP/bin/mrxctl" <<'SH'
#!/usr/bin/env bash
[[ "$1" == "snapshot" ]] || exit 2
cat <<'JSON'
{"monitors":[
  {"name":"DP-1","focused":true,"x":0,"y":0,"width":1920,"height":1080},
  {"name":"HDMI-1","focused":false,"x":1920,"y":0,"width":1280,"height":720}
]}
JSON
SH
cat >"$TMP/bin/xdotool" <<'SH'
#!/usr/bin/env bash
case "$1" in
  getdisplaygeometry) echo "3200 1080" ;;
  getmouselocation) printf 'X=1440\nY=720\nSCREEN=0\nWINDOW=42\n' ;;
  *) exit 2 ;;
esac
SH
cat >"$TMP/bin/hyprctl" <<'SH'
#!/usr/bin/env bash
echo called >>"\${MRX_TEST_HYPRCTL_LOG:?}"
exit 99
SH
cat >"$TMP/bin/bspc" <<'SH'
#!/usr/bin/env bash
echo "BSPWM-1"
SH
cat >"$TMP/bin/niri" <<'SH'
#!/usr/bin/env bash
if [[ "$1 $2 $3" == "msg -j outputs" ]]; then
  echo '{"NIRI-1":{}}'
else
  echo '[{"is_focused":true,"output":"NIRI-1"}]'
fi
SH
chmod +x "$TMP/bin/"*

export PATH="$TMP/bin:$PATH"
export MRX_TEST_HYPRCTL_LOG="$TMP/hyprctl.log"
source "$ROOT/shell/scripts/lib/mrx-session.sh"

assert_eq() {
  local expected="$1" actual="$2" label="$3"
  [[ "$actual" == "$expected" ]] || { printf 'FAIL: %s expected <%s> got <%s>\n' "$label" "$expected" "$actual" >&2; exit 1; }
  printf 'PASS: %s\n' "$label"
}

export XDG_CURRENT_DESKTOP=MRX XDG_SESSION_DESKTOP=mrx
assert_eq mrx "$(mrx_detect_compositor)" "MRX session detection"
mrx_is_x11_session mrx || { echo "FAIL: MRX was not classified as X11" >&2; exit 1; }
assert_eq $'DP-1\nHDMI-1' "$(mrx_get_monitor_names)" "MRX monitor enumeration via mrxctl"
assert_eq DP-1 "$(mrx_get_active_monitor)" "MRX focused monitor selection"
assert_eq $'1\t0\t0\t1920\t1080' "$(mrx_get_focused_monitor_geometry)" "MRX focused monitor geometry"
assert_eq "3200 1080" "$(mrx_get_display_geometry)" "X11 root display geometry"

export XDG_CURRENT_DESKTOP=bspwm XDG_SESSION_DESKTOP=bspwm
assert_eq bspwm "$(mrx_detect_compositor)" "BSPWM session detection"
mrx_is_x11_session bspwm || { echo "FAIL: BSPWM was not classified as X11" >&2; exit 1; }
assert_eq BSPWM-1 "$(mrx_get_active_monitor)" "BSPWM focused monitor selection"

export XDG_CURRENT_DESKTOP=niri XDG_SESSION_DESKTOP=niri
assert_eq niri "$(mrx_detect_compositor)" "Niri session detection"
assert_eq NIRI-1 "$(mrx_get_active_monitor)" "Niri focused monitor selection"

export XDG_CURRENT_DESKTOP=Hyprland XDG_SESSION_DESKTOP=Hyprland
assert_eq hyprland "$(mrx_detect_compositor)" "Hyprland session detection"
[[ ! -e "$MRX_TEST_HYPRCTL_LOG" ]] || { echo "FAIL: X11 session detection invoked hyprctl" >&2; exit 1; }
echo "PASS: MRX/BSPWM helpers do not invoke hyprctl"
