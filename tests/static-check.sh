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

bash -n install.sh uninstall.sh build.sh check-deps.sh session/mrx-session shell/install.sh shell/check-deps.sh scripts/install-deps-arch.sh scripts/hardware-report.sh tests/xvfb-server-check.sh tests/xvfb-smoke.sh
pass 'Shell scripts pass bash -n'

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

# The MRX shell must use its own backend rather than launch BSPWM as WM.
grep -q 'mrx-wm' session/mrx-session || fail 'session does not launch MRX WM'
grep -q 'mrxctl' shell/services/MRXBackend.qml || fail 'QML adapter does not use MRX IPC client'
grep -q 'xorg-server-xvfb' scripts/install-deps-arch.sh || fail 'Arch dependency installer omits Xvfb'
grep -q 'xorg-server-xephyr' scripts/install-deps-arch.sh || fail 'Arch dependency installer omits Xephyr'
if grep -Eiq 'nvidia-470|nvidia-utils|linux-cachyos|grub-install|mkinitcpio' scripts/install-deps-arch.sh; then fail 'dependency installer must not mutate GPU driver/kernel/bootloader'; fi
pass 'session and QML adapter reference MRX services; test dependencies are declared; GPU and bootloader remain untouched'
grep -q 'WM_DELETE_WINDOW' crates/mrx-wm/src/main.rs || fail 'ICCCM polite close protocol missing'
grep -q 'WmHints::get' crates/mrx-wm/src/main.rs || fail 'ICCCM WM_HINTS input model missing'
grep -q 'WM_TAKE_FOCUS' crates/mrx-wm/src/main.rs || fail 'ICCCM WM_TAKE_FOCUS negotiation missing'
grep -q 'focus_client' crates/mrx-wm/src/main.rs || fail 'central ICCCM focus helper missing'
grep -q 'WM_TRANSIENT_FOR' crates/mrx-wm/src/main.rs || fail 'transient-window classification missing'
grep -q 'e.type_==net_close' crates/mrx-wm/src/main.rs || fail 'EWMH close client message missing'
grep -q 'e.type_==net_active' crates/mrx-wm/src/main.rs || fail 'EWMH active-window client message missing'
grep -q 'e.type_==current' crates/mrx-wm/src/main.rs || fail 'EWMH desktop client message missing'
grep -q 'e.type_==desktop' crates/mrx-wm/src/main.rs || fail 'EWMH per-window desktop client message missing'
grep -q '_NET_DESKTOP_NAMES' crates/mrx-wm/src/main.rs || fail 'EWMH desktop names missing'
grep -q '_NET_WORKAREA' crates/mrx-wm/src/main.rs || fail 'EWMH workarea metadata missing'
grep -q '_NET_CLIENT_LIST_STACKING' crates/mrx-wm/src/main.rs || fail 'EWMH client stacking list missing'
grep -q 'query_tree(root)' crates/mrx-wm/src/main.rs || fail 'EWMH stacking order query missing'
grep -q 'Client lists were already published after startup adoption' crates/mrx-wm/src/main.rs || fail 'startup client-list preservation comment missing'
if grep -Fq 'root,clientlist,AtomEnum::WINDOW,&[]' crates/mrx-wm/src/main.rs; then fail 'startup path clears EWMH client list after adoption'; fi
if grep -Fq 'set_wm_state(&conn,target,wm_state,3)' crates/mrx-wm/src/main.rs; then fail 'inactive virtual desktop incorrectly marks client Iconic'; fi
grep -Fq 'WM.compositor !== "mrx" && Config.options.hyprland.autostartApps.enable' shell/shell.qml || fail 'MRX session may invoke Hyprland autostart helper'
if grep -Eq 'root,clientlist,AtomEnum::WINDOW,&\[\]' crates/mrx-wm/src/main.rs; then fail 'startup path clears EWMH client list after adoption'; fi
if grep -Eq 'set_wm_state\(&conn,target,wm_state,3\)' crates/mrx-wm/src/main.rs; then fail 'inactive virtual desktop incorrectly marks client Iconic'; fi
grep -q 'Adopt already-viewable' crates/mrx-wm/src/main.rs || fail 'startup client adoption missing'
grep -q 'Event::UnmapNotify' crates/mrx-wm/src/main.rs || fail 'ICCCM unmap lifecycle handling missing'
grep -q '_NET_WM_STATE_FULLSCREEN' crates/mrx-wm/src/main.rs || fail 'EWMH fullscreen state missing'
grep -q 'set_wm_state' crates/mrx-wm/src/main.rs || fail 'ICCCM WM_STATE lifecycle helper missing'
grep -q 'take(65_537)' crates/mrx-wm/src/main.rs || fail 'IPC request size limit missing'
grep -q 'MAX_IPC_CLIENTS: usize = 32' crates/mrx-wm/src/main.rs || fail 'bounded IPC concurrency missing'
grep -q 'is_socket()' crates/mrx-wm/src/main.rs || fail 'IPC socket path safety check missing'
pass 'ICCCM WM_STATE/polite close, transient classification, EWMH workspace/close/active/fullscreen/client-stacking handling, startup adoption and bounded IPC are present'

old_identity="aure""lia"
if grep -RniF "$old_identity" --exclude-dir=.git .; then
  fail 'former project identity remains in source tree'
fi
pass 'former project identity is absent from project source'
