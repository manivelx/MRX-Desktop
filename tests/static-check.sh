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
[[ -f crates/mrx-graphics-probe/src/main.rs ]] || fail 'graphics probe source missing'
[[ -f shell/scripts/lib/mrx-session.sh ]] || fail 'shared MRX shell session helper missing'
[[ -f tests/mrx-shell-backend.sh ]] || fail 'MRX shell backend test missing'
[[ -f scripts/mrx-diagnose.sh ]] || fail 'MRX startup diagnostics script missing'
[[ -f tests/diagnostics.sh ]] || fail 'MRX startup diagnostics test missing'
[[ -f scripts/package-release.sh ]] || fail 'release packager missing'
[[ -f tests/package-release.sh ]] || fail 'release archive test missing'
[[ -f tests/x11-test-client.c ]] || fail 'X11 test client source missing'
pass 'Rust workspace source layout exists'

bash -n install.sh uninstall.sh build.sh check-deps.sh scripts/package-release.sh tests/package-release.sh scripts/mrx-diagnose.sh tests/diagnostics.sh session/mrx-session session/mrx-wm-supervisor shell/install.sh shell/check-deps.sh scripts/install-deps-arch.sh scripts/hardware-report.sh tests/xvfb-server-check.sh tests/xvfb-smoke.sh tests/xephyr-smoke.sh shell/scripts/lib/mrx-session.sh shell/scripts/colors/switchwall.sh shell/scripts/videos/record.sh shell/scripts/system-appearance.sh tests/mrx-shell-backend.sh tests/installer-sandbox.sh
pass 'Shell scripts pass bash -n'

for gate in 'bash ./check-deps.sh' 'bash ./tests/mrx-shell-backend.sh' 'cargo fmt --all -- --check' 'cargo test --workspace' 'cargo clippy --workspace --all-targets -- -D warnings' 'cargo build --workspace' 'bash ./tests/static-check.sh' 'bash ./tests/installer-sandbox.sh' 'bash ./tests/xvfb-server-check.sh' 'bash ./tests/xvfb-smoke.sh' 'bash ./tests/xephyr-smoke.sh' 'cargo build --release --workspace'; do
  grep -Fq "$gate" build.sh || fail "build.sh is missing validation gate: $gate"
done
grep -Fq 'container: archlinux:latest' .github/workflows/ci.yml || fail 'CI does not validate the Arch Linux build environment'
grep -Fq 'name: Install Arch Linux toolchain and X11 test dependencies' .github/workflows/ci.yml || fail 'Arch Linux CI job omits its native dependency setup'
grep -Fq 'cargo build --release --workspace' .github/workflows/ci.yml || fail 'CI omits the release-mode workspace build'
grep -Fq 'bash ./tests/package-release.sh' build.sh || fail 'build.sh omits reproducible release archive tests'
grep -Fq 'bash ./tests/package-release.sh' .github/workflows/ci.yml || fail 'CI omits release archive tests'
grep -Fq 'bash ./tests/diagnostics.sh' build.sh || fail 'build.sh omits startup diagnostics tests'
grep -Fq 'bash ./tests/diagnostics.sh' .github/workflows/ci.yml || fail 'CI omits startup diagnostics tests'
grep -Fq 'mrx-diagnose --reason' session/mrx-session || fail 'session startup failure does not capture diagnostics'
grep -Fq 'Quickshell frontend failed to start' session/mrx-session || fail 'Quickshell startup failure does not capture diagnostics'
grep -Fq 'FAILURES >= 3' session/mrx-wm-supervisor || fail 'supervisor does not capture diagnostics after repeated WM crashes'
grep -Fq 'startup-diagnostics-*.txt' tests/diagnostics.sh || fail 'diagnostics test omits automatic session startup capture'
grep -Fq 'mrx-diagnose' install.sh || fail 'installer does not install the diagnostics command'
grep -Fq 'softprops/action-gh-release@v2' .github/workflows/release.yml || fail 'tagged release workflow does not publish GitHub releases'
grep -Fq 'mrx-graphics-probe' tests/xvfb-smoke.sh || fail 'X11 smoke test omits the Composite/Damage probe'
grep -Fq 'mrx-session.sh' shell/scripts/colors/switchwall.sh || fail 'wallpaper script omits the shared MRX/X11 session helper'
grep -Fq 'mrx-session.sh' shell/scripts/videos/record.sh || fail 'recording script omits the shared MRX/X11 session helper'
grep -Fq 'mrx_get_focused_monitor_geometry' shell/scripts/colors/switchwall.sh || fail 'MRX wallpaper script omits monitor-aware geometry'
grep -Fq 'mrx_is_x11_session "$COMPOSITOR"' shell/scripts/videos/record.sh || fail 'recording script omits the MRX X11 capture path'
grep -Fq 'GlobalStates.wallpaperSelectorOpen = !GlobalStates.wallpaperSelectorOpen' shell/services/LauncherSearch.qml || fail 'wallpaper search action still depends on compositor-only IPC'
grep -Fq '"$desktop" == *hyprland*' shell/scripts/system-appearance.sh || fail 'cursor script may invoke hyprctl outside Hyprland'
grep -Fq 'cc' check-deps.sh || fail 'dependency checker omits the Xlib test-client compiler'
grep -Fq 'xwininfo' check-deps.sh || fail 'dependency checker omits the secondary dock fixture lookup'
grep -Fq 'Xvfb Xephyr xdpyinfo xprop xwininfo xrandr python3 jq xdotool slop ffmpeg cc' check-deps.sh || fail 'dependency checker does not verify all required X11/Xephyr smoke tools'
grep -Fq 'xrandr --setmonitor MRX-LEFT' tests/xvfb-smoke.sh || fail 'Xvfb smoke test omits the first virtual RandR monitor'
grep -Fq 'xrandr --setmonitor MRX-RIGHT' tests/xvfb-smoke.sh || fail 'Xvfb smoke test omits the second virtual RandR monitor'
grep -Fq 'MONITORS_READY' tests/xvfb-smoke.sh || fail 'Xvfb smoke test does not assert monitor snapshot refresh'
grep -Fq 'independent monitor-local workspace state is exposed through IPC' tests/xvfb-smoke.sh || fail 'Xvfb smoke test does not exercise independent monitor workspaces'
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
grep -Fq '"$RUNTIME/x11-test-client" "MRX X11 test server test client"' tests/xvfb-smoke.sh || fail 'Xvfb smoke test does not launch the compiled minimal X11 client'
if grep -Eiq 'nvidia-470|nvidia-utils|linux-cachyos|grub-install|mkinitcpio' scripts/install-deps-arch.sh; then fail 'dependency installer must not mutate GPU driver/kernel/bootloader'; fi
pass 'session and QML adapter reference MRX services; test dependencies are declared; GPU and bootloader remain untouched'
grep -q 'WM_DELETE_WINDOW' crates/mrx-wm/src/main.rs || fail 'ICCCM polite close protocol missing'
grep -q 'WmHints::get' crates/mrx-wm/src/main.rs || fail 'ICCCM WM_HINTS input model missing'
grep -q 'WM_TAKE_FOCUS' crates/mrx-wm/src/main.rs || fail 'ICCCM WM_TAKE_FOCUS negotiation missing'
grep -q 'query_pointer(root)' crates/mrx-wm/src/main.rs || fail 'new X11 clients are not assigned using the pointer monitor'
grep -q 'focus_client' crates/mrx-wm/src/main.rs || fail 'central ICCCM focus helper missing'
grep -q 'focus_most_recent' crates/mrx-wm/src/main.rs || fail 'focus restoration after client removal missing'
grep -q 'WM_TRANSIENT_FOR' crates/mrx-wm/src/main.rs || fail 'transient-window classification missing'
grep -Fq 'WmSizeHints::get_normal_hints' crates/mrx-wm/src/main.rs || fail 'ICCCM WM_NORMAL_HINTS parsing missing'
grep -Fq 'fn constrain_rect' crates/mrx-wm/src/main.rs || fail 'ICCCM size constraints are not applied to tiled clients'
grep -Fq 'fn has_fixed_size' crates/mrx-wm/src/main.rs || fail 'fixed-size client detection missing'
grep -Fq 'size_hints.insert' crates/mrx-wm/src/main.rs || fail 'WM does not retain client size hints'
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
grep -Fq 'randr_get_monitors(root, false)' crates/mrx-wm/src/main.rs || fail 'RandR discovery must include inactive and virtual monitor objects'
grep -q 'randr_select_input' crates/mrx-wm/src/main.rs || fail 'RandR hotplug notifications missing'
grep -q 'Event::RandrNotify' crates/mrx-wm/src/main.rs || fail 'RandR notification event handling missing'
grep -q 'last_monitor_refresh.elapsed' crates/mrx-wm/src/main.rs || fail 'periodic RandR monitor reconciliation fallback missing'
grep -q 'fn refresh_monitor_state' crates/mrx-wm/src/main.rs || fail 'shared RandR monitor reconciliation routine missing'
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
grep -Fq 'pub struct RequestEnvelope' crates/mrx-core/src/lib.rs || fail 'versioned IPC request envelope missing'
grep -Fq 'pub const IPC_PROTOCOL_VERSION: u32 = 3' crates/mrx-core/src/lib.rs || fail 'IPC protocol version is not explicit'
grep -Fq "data['protocol_version'] == 3" tests/xvfb-smoke.sh || fail 'Xvfb smoke test does not assert the current IPC protocol version'
grep -Fq 'envelope.validate()' crates/mrx-wm/src/main.rs || fail 'WM does not reject unsupported IPC protocol versions'
grep -Fq 'response.request_id = Some(request_id)' crates/mrx-wm/src/main.rs || fail 'WM does not echo IPC request IDs'
grep -Fq 'response.request_id == Some(envelope.request_id)' crates/mrxctl/src/main.rs || fail 'mrxctl does not verify IPC response correlation'
pass 'ICCCM WM_STATE/polite close, transient classification, EWMH workspace/close/active/fullscreen/client-stacking handling, startup adoption and bounded IPC are present'

old_identity="aure""lia"
if grep -RniF "$old_identity" --exclude-dir=.git .; then
  fail 'former project identity remains in source tree'
fi
pass 'former project identity is absent from project source'

grep -Fq 'pub urgent: bool' crates/mrx-core/src/lib.rs || fail 'shared window snapshot omits ICCCM urgency'
grep -Fq 'read_urgent_hint' crates/mrx-wm/src/main.rs || fail 'WM does not read ICCCM urgency hints'
grep -Fq 'Some(Event::PropertyNotify(e))' crates/mrx-wm/src/main.rs || fail 'WM does not refresh urgency when WM_HINTS changes'
grep -Fq 'set-wm-hints' tests/xvfb-smoke.sh && grep -Fq 'XSetWMHints' tests/set-wm-hints.c || fail 'Xvfb smoke test does not exercise ICCCM urgency hint updates'
grep -Fq 'urgent: Boolean(w.urgent)' shell/services/MRXBackend.qml || fail 'Quickshell adapter discards window urgency'
grep -Fq 'hasUrgentWindows' shell/modules/ii/bar/ActiveWindow.qml || fail 'Quickshell active-window widget omits urgency indicator'

grep -Fq 'WmSizeHints::get_normal_hints' crates/mrx-wm/src/main.rs || fail 'WM does not parse WM_NORMAL_HINTS'
grep -Fq 'wm_normal_hints' crates/mrx-wm/src/main.rs || fail 'WM does not subscribe to WM_NORMAL_HINTS property changes'
grep -Fq 'constrain_rect' crates/mrx-wm/src/main.rs || fail 'tiled placement does not apply ICCCM size constraints'
grep -Fq 'normal_size_hints_apply_minimum_and_maximum_constraints' crates/mrx-wm/src/main.rs || fail 'WM_NORMAL_HINTS min/max regression test missing'
grep -Fq 'normal_size_hints_apply_resize_increments' crates/mrx-wm/src/main.rs || fail 'WM_NORMAL_HINTS resize-increment regression test missing'
grep -Fq 'normal_size_hints_apply_aspect_ratio_bounds' crates/mrx-wm/src/main.rs || fail 'WM_NORMAL_HINTS aspect-ratio regression test missing'
grep -Fq 's.size_hints.remove(&e.window)' crates/mrx-wm/src/main.rs || fail 'window size-hint cache is not cleared on client removal'

grep -Fq 'function switchWorkspaceOnMonitor' shell/services/WM.qml || fail 'monitor-specific workspace action missing'
grep -Fq 'SwitchWorkspaceOnMonitor' crates/mrx-core/src/lib.rs || fail 'monitor-scoped workspace IPC request missing'
grep -Fq 'Action::SwitchMonitor' crates/mrx-wm/src/main.rs || fail 'WM does not handle monitor-scoped workspace switches'
grep -Fq 'active_by_monitor' crates/mrx-wm/src/main.rs || fail 'WM has no independent active workspace state per monitor'
grep -Fq 'monitor: monitor.name.clone()' crates/mrx-wm/src/main.rs || fail 'workspace snapshots are not associated with their monitor'
grep -Fq 'switchWorkspaceOnMonitor(id, monitorName)' shell/services/MRXBackend.qml || fail 'MRX backend does not pass monitor identity to workspace switches'
grep -Fq 'MoveWindowToMonitor' crates/mrx-core/src/lib.rs || fail 'monitor-targeted window move IPC request missing'
grep -Fq 'Action::MoveMonitor' crates/mrx-wm/src/main.rs || fail 'WM does not handle monitor-targeted window moves'
grep -Fq '"move-monitor" => Request::MoveWindowToMonitor' crates/mrxctl/src/main.rs || fail 'mrxctl does not expose monitor-targeted window moves'
grep -Fq 'function moveWindowToMonitor' shell/services/MRXBackend.qml || fail 'MRX shell backend omits monitor-targeted window moves'
grep -Fq 'function monitorNameFor(name)' shell/services/MRXBackend.qml || fail 'MRX backend does not resolve Quickshell screen aliases'
grep -Fq 'Number(m.x) === Number(screen?.x)' shell/services/MRXBackend.qml || fail 'MRX backend lacks geometry-based monitor matching'
grep -Fq 'Boolean(w.minimized) || Number(w.workspace)' shell/services/MRXBackend.qml || fail 'MRX backend loses minimized/hidden window state'
grep -Fq 'maximizedHorz: Boolean(w.maximized_horz)' shell/services/MRXBackend.qml || fail 'MRX backend discards EWMH maximize state'
grep -Fq 'WM.monitorFor(screen)?.name ?? screen?.name' shell/modules/common/models/WorkspaceModel.qml || fail 'workspace model does not use resolved monitor identity'
grep -Fq 'function moveWindowToMonitor' shell/services/WM.qml || fail 'WM facade omits monitor-targeted window moves'
grep -Fq 'monitor\s*=\s*' shell/services/WM.qml || fail 'MRX command dispatcher cannot parse monitor-targeted moves'
grep -Fq 'IPC moves clients across monitors/workspaces and rejects unknown monitor targets' tests/xvfb-smoke.sh || fail 'Xvfb test omits cross-monitor window move coverage'
grep -Fq 'MoveWindowToMonitor' crates/mrx-core/src/lib.rs || fail 'monitor-targeted window move IPC request missing'
grep -Fq 'Action::MoveMonitor' crates/mrx-wm/src/main.rs || fail 'WM does not handle monitor-targeted window moves'
grep -Fq '"move-monitor" => Request::MoveWindowToMonitor' crates/mrxctl/src/main.rs || fail 'mrxctl does not expose monitor-targeted window moves'
grep -Fq 'function moveWindowToMonitor' shell/services/MRXBackend.qml || fail 'MRX shell backend omits monitor-targeted window moves'
grep -Fq 'function moveWindowToMonitor' shell/services/WM.qml || fail 'WM facade omits monitor-targeted window moves'
grep -Fq 'IPC moves clients across monitors/workspaces and rejects unknown monitor targets' tests/xvfb-smoke.sh || fail 'Xvfb test omits cross-monitor window move coverage'
grep -Fq 'WM.switchWorkspaceOnMonitor(wsModel.getWorkspaceIdAt(hoverIndex), wsModel.monitorName)' shell/modules/ii/bar/Workspaces.qml || fail 'workspace bar does not target its own monitor'
grep -Fq 'fn publish_workarea' crates/mrx-wm/src/main.rs || fail 'EWMH workarea property is not published'
grep -Fq 'publish_workarea' crates/mrx-wm/src/main.rs && grep -Fq 'net_workarea' crates/mrx-wm/src/main.rs || fail 'EWMH workarea is not initialized for desktops'
grep -Fq 'EWMH workarea is published for all nine desktops' tests/xvfb-smoke.sh || fail 'Xvfb smoke test omits EWMH workarea validation'

grep -Fq 'read_dock_strut' crates/mrx-wm/src/main.rs || fail 'WM does not parse EWMH dock struts'
grep -Fq 'monitor_workarea' crates/mrx-wm/src/main.rs || fail 'WM does not compute usable monitor workareas'
grep -Fq 'is_dock_window' crates/mrx-wm/src/main.rs || fail 'WM does not identify EWMH dock window types'
grep -Fq 'pub dock: bool' crates/mrx-core/src/lib.rs && grep -Fq 'is_dock_window' crates/mrx-wm/src/main.rs && grep -Fq 'let dock' crates/mrx-wm/src/main.rs || fail 'dock identity is not retained in window state'
grep -Fq 'w.monitor == monitor && !w.dock' crates/mrx-wm/src/main.rs || fail 'dock clients are hidden when switching workspaces'
grep -Fq 'filter(w => !w.dock)' shell/services/MRXBackend.qml || fail 'Quickshell exposes dock clients as ordinary workspace windows'
grep -Fq 'dock clients remain sticky and do not steal keyboard focus' tests/xvfb-smoke.sh || fail 'Xvfb smoke test omits dock focus and stickiness validation'
grep -Fq 'simultaneous top and vertical dock struts reserve intersecting edges' tests/xvfb-smoke.sh || fail 'Xvfb smoke test omits simultaneous vertical/top dock workarea coverage'
grep -Fq 'removing a dock restores the remaining top-only workarea' tests/xvfb-smoke.sh || fail 'Xvfb smoke test omits dock-removal workarea restoration'
grep -Fq 'name: "root-workarea"' crates/mrx-wm/src/main.rs || fail 'EWMH workarea is not computed against the root screen'
grep -Fq 'EWMH dock partial strut reduces usable workarea' tests/xvfb-smoke.sh || fail 'Xvfb smoke test omits dock strut reservation validation'
grep -Fq 'right_and_bottom_struts_use_root_edge_distances' crates/mrx-wm/src/main.rs || fail 'dock strut root-edge geometry tests missing'

grep -Fq 'pub const IPC_PROTOCOL_VERSION: u32 = 3' crates/mrx-core/src/lib.rs || fail 'IPC protocol version was not advanced for subscriptions'
grep -Fq '    Subscribe,' crates/mrx-core/src/lib.rs || fail 'IPC protocol has no subscription request'
grep -Fq 'fn stream_ipc_snapshots' crates/mrx-wm/src/main.rs || fail 'WM does not stream changed snapshots to subscribers'
grep -Fq '"subscribe"' crates/mrxctl/src/main.rs || fail 'mrxctl has no persistent subscription command'
grep -Fq 'mrxctl", "subscribe' shell/services/MRXBackend.qml || fail 'Quickshell does not consume the IPC event stream'
grep -Fq 'SplitParser' shell/services/MRXBackend.qml || fail 'Quickshell cannot parse newline-delimited subscription events'
grep -Fq 'persistent IPC subscription emits initial and changed snapshots' tests/xvfb-smoke.sh || fail 'Xvfb smoke test omits subscription event validation'
grep -Fq 'probe.read(&mut incoming)' crates/mrx-wm/src/main.rs || fail 'IPC subscriptions do not detect disconnected clients'

grep -Fq 'fn maximized_rect' crates/mrx-wm/src/main.rs || fail 'EWMH maximize geometry helper is missing'
grep -Fq 'net_wm_state_maximized_horz' crates/mrx-wm/src/main.rs || fail 'horizontal EWMH maximize state is missing'
grep -Fq 'net_wm_state_maximized_vert' crates/mrx-wm/src/main.rs || fail 'vertical EWMH maximize state is missing'
grep -Fq 'ewmh_maximize_axes_preserve_unrequested_geometry' crates/mrx-wm/src/main.rs || fail 'maximize axis geometry regression test is missing'
grep -Fq 'EWMH maximize/restore round-trip preserves original geometry' tests/xvfb-smoke.sh || fail 'Xvfb smoke test omits maximize/restore round-trip'
[[ -f tests/send-net-wm-state.c ]] || fail 'EWMH state ClientMessage test helper is missing'

grep -Fq 'pub minimized: bool' crates/mrx-core/src/lib.rs || fail 'IPC window state omits minimized status'
grep -Fq 'wm_change_state' crates/mrx-wm/src/main.rs || fail 'ICCCM WM_CHANGE_STATE minimize handling is missing'
grep -Fq 'net_wm_state_hidden' crates/mrx-wm/src/main.rs || fail 'EWMH hidden/minimize request handling is missing'
grep -Fq 'window.minimized' crates/mrx-wm/src/main.rs || fail 'WM does not preserve minimized clients in state'
grep -Fq 'EWMH minimize/restore retains the managed client and WM_STATE' tests/xvfb-smoke.sh || fail 'Xvfb smoke test omits minimize/restore lifecycle coverage'

grep -Fq 'net_wm_state_above' crates/mrx-wm/src/main.rs || fail 'EWMH ABOVE state is missing'
grep -Fq 'net_wm_state_below' crates/mrx-wm/src/main.rs || fail 'EWMH BELOW state is missing'
grep -Fq 'pub above: bool' crates/mrx-core/src/lib.rs && grep -Fq 'pub below: bool' crates/mrx-core/src/lib.rs || fail 'IPC window snapshot omits ABOVE/BELOW state'
grep -Fq 'EWMH ABOVE/BELOW state requests are mutually exclusive' tests/xvfb-smoke.sh || fail 'Xvfb smoke test omits ABOVE/BELOW stacking coverage'

grep -Fq 'fn read_atom_list' crates/mrx-wm/src/main.rs || fail 'startup EWMH atom-list reader is missing'
grep -Fq 'read_atom_list' crates/mrx-wm/src/main.rs || fail 'startup EWMH state adoption is missing'
grep -Fq 'existing_wm_state == 3' crates/mrx-wm/src/main.rs || fail 'startup ICCCM IconicState adoption is missing'
grep -Fq 'STARTUP_ADOPTED' tests/xvfb-smoke.sh || fail 'Xvfb startup adoption test is missing'

grep -Fq 'fn stacking_order' crates/mrx-wm/src/main.rs || fail 'managed EWMH stacking order is missing'
grep -Fq 'fn restack_managed_clients' crates/mrx-wm/src/main.rs || fail 'managed X11 restack reconciliation is missing'
grep -Fq 'managed_stacking_order_respects_protocol_layers_and_stability' crates/mrx-wm/src/main.rs || fail 'managed stacking layer unit test is missing'

grep -Fq 'pub transient_for: u32' crates/mrx-core/src/lib.rs || fail 'ICCCM transient parent is missing from window snapshots'
grep -Fq 'pub modal: bool' crates/mrx-core/src/lib.rs || fail 'EWMH modal state is missing from window snapshots'
grep -Fq 'fn modal_focus_target' crates/mrx-wm/src/main.rs || fail 'modal focus redirection is missing'
grep -Fq 'net_wm_state_modal' crates/mrx-wm/src/main.rs || fail 'EWMH modal state support is missing'
grep -Fq 'startup adopts ICCCM transient/modal dialogs and redirects parent focus' tests/xvfb-smoke.sh || fail 'Xvfb modal/transient acceptance test is missing'

grep -Fq 'pub struct OperationResult' crates/mrx-core/src/lib.rs || fail 'structured IPC operation results are missing'
grep -Fq 'GetOperation' crates/mrx-core/src/lib.rs || fail 'IPC operation polling request is missing'
grep -Fq 'fn enqueue_operation' crates/mrx-wm/src/main.rs || fail 'asynchronous operation queue is missing'
grep -Fq 'fn finish_operation' crates/mrx-wm/src/main.rs || fail 'operation completion tracking is missing'
grep -Fq 'operation <id>' crates/mrxctl/src/main.rs || fail 'mrxctl operation polling command is missing'
grep -Fq 'IPC operations expose queued/completed/failed states' tests/xvfb-smoke.sh || fail 'Xvfb asynchronous operation acceptance test is missing'

grep -Fq 'pub fn save_profile_atomic' crates/mrx-core/src/lib.rs || fail 'atomic settings profile export is missing'
grep -Fq 'pub fn load_profile' crates/mrx-core/src/lib.rs || fail 'validated settings profile import is missing'
grep -Fq 'settings_history' crates/mrx-wm/src/main.rs || fail 'bounded settings undo history is missing'
grep -Fq 'Action::UndoSettings' crates/mrx-wm/src/main.rs || fail 'settings undo action is missing'
grep -Fq 'Action::ResetSettings' crates/mrx-wm/src/main.rs || fail 'settings reset action is missing'
grep -Fq 'profile import' crates/mrxctl/src/main.rs || fail 'mrxctl profile commands are missing'
grep -Fq 'settings undo/reset, profile import/export and profile path validation' tests/xvfb-smoke.sh || fail 'Xvfb settings profile acceptance test is missing'

grep -Fq 'pub struct HealthReport' crates/mrx-core/src/lib.rs || fail 'IPC health report type is missing'
grep -Fq 'Request::Health' crates/mrx-wm/src/main.rs || fail 'IPC health request handler is missing'
grep -Fq 'Action::Shutdown' crates/mrx-wm/src/main.rs || fail 'orderly WM shutdown action is missing'
grep -Fq '"health" => Request::Health' crates/mrxctl/src/main.rs || fail 'mrxctl health command is missing'
grep -Fq '"shutdown" => Request::Shutdown' crates/mrxctl/src/main.rs || fail 'mrxctl shutdown command is missing'
grep -Fq 'IPC health reports process, uptime' tests/xvfb-smoke.sh || fail 'Xvfb health acceptance test is missing'
grep -Fq 'IPC shutdown exits cleanly' tests/xvfb-smoke.sh || fail 'Xvfb orderly shutdown acceptance test is missing'

bash -n session/mrx-wm-supervisor || fail 'WM supervisor shell syntax is invalid'
grep -Fq 'DELAY >= 30' session/mrx-wm-supervisor || fail 'WM supervisor restart backoff is not capped'
grep -Fq 'mrx-wm-supervisor &' session/mrx-session || fail 'session launcher does not use the WM supervisor'
grep -Fq 'mrx-wm-supervisor' install.sh || fail 'installer does not install the WM supervisor'
grep -Fq 'mrx-wm-supervisor' uninstall.sh || fail 'uninstaller does not remove the WM supervisor'
grep -Fq 'supervisor restarts crashed WM child' tests/xvfb-smoke.sh || fail 'Xvfb supervisor crash-recovery test is missing'

grep -Fq 'pub subscription_slow_clients_dropped' crates/mrx-core/src/lib.rs || fail 'subscription backpressure health metrics are missing'
grep -Fq 'pub sequence: Option<u64>' crates/mrx-core/src/lib.rs || fail 'snapshot event sequence numbers are missing'
grep -Fq 'fn record_subscription_write' crates/mrx-wm/src/main.rs || fail 'subscription write metrics recorder is missing'
grep -Fq 'slow_subscription_socket_write_records_backpressure_and_latency' crates/mrx-wm/src/main.rs || fail 'slow subscriber backpressure unit test is missing'
grep -Fq 'snapshot subscriptions report monotonic sequence' tests/xvfb-smoke.sh || fail 'Xvfb subscription sequence/metrics acceptance test is missing'
