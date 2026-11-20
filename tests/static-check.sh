#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }
pass() { printf 'PASS: %s\n' "$*"; }

[[ -f Cargo.toml ]] || fail 'Cargo workspace manifest missing'
[[ -f crates/mrx-core/src/lib.rs ]] || fail 'mrx-core source missing'
[[ -f crates/mrx-wm/src/main.rs ]] || fail 'mrx-wm source missing'
[[ -f crates/mrxctl/src/main.rs ]] || fail 'mrxctl source missing'
pass 'Rust workspace source layout exists'

bash -n install.sh uninstall.sh build.sh check-deps.sh session/mrx-session shell/install.sh shell/check-deps.sh scripts/install-deps-arch.sh scripts/hardware-report.sh tests/xvfb-server-check.sh tests/xvfb-smoke.sh tests/installer-sandbox.sh
pass 'Shell scripts pass bash -n'

for gate in 'cargo fmt --all -- --check' 'cargo test --workspace' 'cargo clippy --workspace --all-targets -- -D warnings' 'cargo build --workspace' 'bash ./tests/static-check.sh' 'bash ./tests/xvfb-server-check.sh' 'bash ./tests/xvfb-smoke.sh' 'cargo build --release --workspace'; do
  grep -Fq "$gate" build.sh || fail "build.sh is missing validation gate: $gate"
done
grep -Fq 'xmessage' check-deps.sh || fail 'dependency checker omits the Xvfb smoke-test client'
grep -Fq 'Xvfb xdpyinfo xprop xmessage xrandr python3' check-deps.sh || fail 'dependency checker does not verify all required Xvfb smoke tools'
grep -Fq 'xrandr --fb 2560x1440' tests/xvfb-smoke.sh || fail 'Xvfb smoke test omits the live RandR screen-change test'
grep -Fq 'SCREEN_UPDATED' tests/xvfb-smoke.sh || fail 'Xvfb smoke test does not assert monitor geometry refresh after RandR event'
grep -Fq 'xrandr' check-deps.sh || fail 'dependency checker omits xrandr'
grep -Fq 'x11-xserver-utils' .github/workflows/ci.yml || fail 'CI omits the xrandr package'
grep -Fq 'xorg-xrandr' scripts/install-deps-arch.sh || fail 'Arch dependency installer omits xrandr'
pass 'developer build script runs the complete validation suite and dependency checks match the smoke test'

python3 - <<'PY'
import tomllib
from pathlib import Path
for path in (Path('Cargo.toml'), Path('config/settings.toml')):
    with path.open('rb') as handle:
        tomllib.load(handle)
print('PASS: TOML files parse')
PY

# Guard against accidental coupling to the user's existing login/session files.
if grep -Eiq '(^|[[:space:]/])(/etc/sddm\.conf|/usr/share/sddm/themes|\.local/share/quickshell-lockscreen)' install.sh uninstall.sh; then
  fail 'installer/session scripts reference protected login or lock-screen paths'
fi
pass 'installer scripts do not modify protected SDDM/lock-screen paths'
if grep -Fq 'install -m644 "$ROOT/session/mrx.desktop" /usr/share/xsessions/mrx.desktop' install.sh; then fail 'installer must not silently overwrite a system-wide session entry'; fi
if grep -Fq 'rm -f /usr/share/xsessions/mrx.desktop' uninstall.sh; then fail 'uninstaller must not remove a potentially pre-existing system-wide session entry'; fi
grep -Fq 'picom.conf" ]] || install' install.sh || fail 'reinstall must preserve the existing Picom config'
pass 'installer preserves user configuration and leaves system-wide session registration opt-in'
bash ./tests/installer-sandbox.sh
grep -Fq 'bash "$ROOT/install.sh"' tests/installer-sandbox.sh || fail 'installer sandbox must invoke non-executable install.sh through bash'
grep -Fq 'bash "$ROOT/uninstall.sh"' tests/installer-sandbox.sh || fail 'installer sandbox must invoke non-executable uninstall.sh through bash'

# The MRX shell must use its own backend rather than launch BSPWM as WM.
grep -q 'mrx-wm' session/mrx-session || fail 'session does not launch MRX WM'
grep -q 'mrxctl' shell/services/MRXBackend.qml || fail 'QML adapter does not use MRX IPC client'
grep -q 'xorg-server-xvfb' scripts/install-deps-arch.sh || fail 'Arch dependency installer omits Xvfb'
grep -q 'xorg-server-xephyr' scripts/install-deps-arch.sh || fail 'Arch dependency installer omits Xephyr'
grep -q 'xorg-fonts-misc' scripts/install-deps-arch.sh || fail 'Arch dependency installer omits core X11 fonts'
grep -q 'xorg-xmessage' scripts/install-deps-arch.sh || fail 'Arch dependency installer omits xmessage smoke-test client'
grep -q 'xmessage -title' tests/xvfb-smoke.sh || fail 'Xvfb smoke test has no minimal X11 client'
if grep -Eiq 'nvidia-470|nvidia-utils|linux-cachyos|grub-install|mkinitcpio' scripts/install-deps-arch.sh; then fail 'dependency installer must not mutate GPU driver/kernel/bootloader'; fi
pass 'session and QML adapter reference MRX services; test dependencies are declared; GPU and bootloader remain untouched'
grep -q 'WM_DELETE_WINDOW' crates/mrx-wm/src/main.rs || fail 'ICCCM polite close protocol missing'
grep -q 'WmHints::get' crates/mrx-wm/src/main.rs || fail 'ICCCM WM_HINTS input model missing'
grep -q 'WM_TAKE_FOCUS' crates/mrx-wm/src/main.rs || fail 'ICCCM WM_TAKE_FOCUS negotiation missing'
grep -q 'query_pointer(root)' crates/mrx-wm/src/main.rs || fail 'new X11 clients are not assigned using the pointer monitor'
grep -q 'focus_client' crates/mrx-wm/src/main.rs || fail 'central ICCCM focus helper missing'
grep -q 'focus_most_recent' crates/mrx-wm/src/main.rs || fail 'focus restoration after client removal missing'
grep -q 'WM_TRANSIENT_FOR' crates/mrx-wm/src/main.rs || fail 'transient-window classification missing'
grep -Fq 'net_close {' crates/mrx-wm/src/main.rs && grep -Fq 'EWMH close request failed' crates/mrx-wm/src/main.rs || fail 'EWMH close client message missing'
grep -Fq 'net_active {' crates/mrx-wm/src/main.rs && grep -Fq 'net_active_window' crates/mrx-wm/src/main.rs || fail 'EWMH active-window client message missing'
grep -Fq 'current {' crates/mrx-wm/src/main.rs || fail 'EWMH desktop client message missing'
grep -Fq 'desktop {' crates/mrx-wm/src/main.rs || fail 'EWMH per-window desktop client message missing'
grep -q '_NET_DESKTOP_NAMES' crates/mrx-wm/src/main.rs || fail 'EWMH desktop names missing'
grep -q '_NET_WORKAREA' crates/mrx-wm/src/main.rs || fail 'EWMH workarea metadata missing'
grep -q '_NET_CLIENT_LIST_STACKING' crates/mrx-wm/src/main.rs || fail 'EWMH client stacking list missing'
grep -q 'query_tree(root)' crates/mrx-wm/src/main.rs || fail 'EWMH stacking order query missing'
grep -q 'Client lists were already published after startup adoption' crates/mrx-wm/src/main.rs || fail 'startup client-list preservation comment missing'
if grep -Fq 'root,clientlist,AtomEnum::WINDOW,&[]' crates/mrx-wm/src/main.rs; then fail 'startup path clears EWMH client list after adoption'; fi
if grep -Fq 'set_wm_state(&conn,target,wm_state,3)' crates/mrx-wm/src/main.rs; then fail 'inactive virtual desktop incorrectly marks client Iconic'; fi
grep -Fq 'WM.compositor !== "mrx" && Config.options.hyprland.autostartApps.enable' shell/shell.qml || fail 'MRX session may invoke Hyprland autostart helper'
grep -Fq 'function relativeWorkspaceDirection(expression)' shell/services/WM.qml || fail 'MRX workspace dispatcher parser missing'
grep -Fq 'Config.ready && WM.compositor !== "mrx"' shell/services/ConflictKiller.qml || fail 'MRX session may terminate tray/notification services'
grep -Fq 'if (WM.compositor !== "mrx")' shell/services/FirstRunExperience.qml || fail 'MRX first run may execute compositor-specific wallpaper script'
grep -Fq 'readonly property bool isMrx: WM.compositor === "mrx"' shell/services/Hyprsunset.qml || fail 'MRX night-light isolation missing'
grep -Fq 'if (root.isMrx) return;' shell/services/Hyprsunset.qml || fail 'MRX may manipulate upstream night-light processes'
grep -Fq 'if (root.compositor === "mrx")' shell/services/WM.qml || fail 'MRX command dispatch isolation missing'
if grep -Eq 'root,clientlist,AtomEnum::WINDOW,&\[\]' crates/mrx-wm/src/main.rs; then fail 'startup path clears EWMH client list after adoption'; fi
if grep -Eq 'set_wm_state\(&conn,target,wm_state,3\)' crates/mrx-wm/src/main.rs; then fail 'inactive virtual desktop incorrectly marks client Iconic'; fi
grep -q 'Adopt already-viewable' crates/mrx-wm/src/main.rs || fail 'startup client adoption missing'
grep -q 'Event::UnmapNotify' crates/mrx-wm/src/main.rs || fail 'ICCCM unmap lifecycle handling missing'
grep -q 'randr_get_monitors' crates/mrx-wm/src/main.rs || fail 'RandR monitor discovery missing'
grep -q 'randr_select_input' crates/mrx-wm/src/main.rs || fail 'RandR hotplug notifications missing'
grep -q 'Event::RandrNotify' crates/mrx-wm/src/main.rs || fail 'RandR notification event handling missing'
grep -q 'monitor_for_geometry' crates/mrx-wm/src/main.rs || fail 'window-to-monitor assignment missing'
if grep -Fq 's.monitor.' crates/mrx-wm/src/main.rs; then fail 'legacy single-monitor state reference remains'; fi
grep -q 'window.monitor = fallback.name' crates/mrx-wm/src/main.rs || fail 'monitor-removal window migration missing'
if grep -Fq 'output:"screen-0"' shell/services/MRXBackend.qml || grep -Fq 'monitorName:"screen-0"' shell/services/MRXBackend.qml; then fail 'MRX QML adapter still hardcodes a synthetic monitor'; fi
grep -q '_NET_WM_STATE_FULLSCREEN' crates/mrx-wm/src/main.rs || fail 'EWMH fullscreen state missing'
grep -q 'set_wm_state' crates/mrx-wm/src/main.rs || fail 'ICCCM WM_STATE lifecycle helper missing'
grep -q 'take(65_537)' crates/mrx-wm/src/main.rs || fail 'IPC request size limit missing'
grep -q 'MAX_IPC_CLIENTS: usize = 32' crates/mrx-wm/src/main.rs || fail 'bounded IPC concurrency missing'
grep -q 'is_socket()' crates/mrx-wm/src/main.rs || fail 'IPC socket path safety check missing'
grep -q 'take(65_537)' crates/mrxctl/src/main.rs || fail 'mrxctl response size limit missing'
grep -q 'set_read_timeout' crates/mrxctl/src/main.rs || fail 'mrxctl socket timeout missing'
pass 'ICCCM WM_STATE/polite close, transient classification, EWMH workspace/close/active/fullscreen/client-stacking handling, startup adoption and bounded IPC are present'

old_identity="aure""lia"
if grep -RniF "$old_identity" --exclude-dir=.git .; then
  fail 'former project identity remains in source tree'
fi
pass 'former project identity is absent from project source'
