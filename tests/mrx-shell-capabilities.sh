#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }
pass() { printf 'PASS: %s\n' "$*"; }

grep -Fq 'enabled: WM.compositor !== "mrx"' shell/modules/ii/sidebarRight/quickToggles/classicStyle/NightLight.qml || fail 'classic MRX night-light toggle is not disabled'
grep -Fq 'Night Light is not available in MRX yet' shell/modules/ii/sidebarRight/quickToggles/classicStyle/NightLight.qml || fail 'classic night-light toggle has no unsupported-state tooltip'
grep -Fq 'available: WM.compositor !== "mrx"' shell/modules/common/models/quickToggles/NightLightToggle.qml || fail 'Android MRX night-light model is not marked unavailable'
grep -Fq 'Night Light is not available in MRX yet' shell/modules/common/models/quickToggles/NightLightToggle.qml || fail 'Android night-light model has no unsupported-state tooltip'

grep -Fq 't.type !== "gameMode" && t.type !== "antiFlashbang"' shell/modules/ii/sidebarRight/quickToggles/AndroidQuickPanel.qml || fail 'configured Hyprland-only Android toggles remain visible outside Hyprland'
grep -Fq 't !== "gameMode" && t !== "antiFlashbang"' shell/modules/ii/sidebarRight/quickToggles/AndroidQuickPanel.qml || fail 'Hyprland-only Android toggles remain addable outside Hyprland'

grep -Fq 'visible: WM.compositor === "mrx"' shell/modules/ii/sidebarRight/nightLight/NightLightDialog.qml || fail 'MRX night-light unsupported notice is missing'
grep -Fq 'enabled: WM.compositor !== "mrx"' shell/modules/ii/sidebarRight/nightLight/NightLightDialog.qml || fail 'MRX night-light controls are not disabled'
grep -Fq 'visible: WM.compositor === "hyprland"' shell/modules/ii/sidebarRight/nightLight/NightLightDialog.qml || fail 'Hyprland-only anti-flashbang controls are not gated'
grep -Fq 'text: Translation.tr("Brightness")' shell/modules/ii/sidebarRight/nightLight/NightLightDialog.qml || fail 'supported X11 hardware brightness control was removed'

grep -Fq 'root.currentWindowGaps = Number(appearance.gaps ?? 8)' shell/modules/ii/sidebarRight/quickToggles/classicStyle/GameMode.qml || fail 'MRX game mode does not preserve the previous window gap'
grep -Fq 'root.currentBorderWidth = Number(appearance.border_width ?? 2)' shell/modules/ii/sidebarRight/quickToggles/classicStyle/GameMode.qml || fail 'MRX game mode does not preserve the previous border width'
grep -Fq 'root.savedWindowGaps = root.currentWindowGaps' shell/modules/ii/sidebarRight/quickToggles/classicStyle/GameMode.qml || fail 'MRX game mode does not snapshot the original gap'
grep -Fq 'root.savedBorderWidth = root.currentBorderWidth' shell/modules/ii/sidebarRight/quickToggles/classicStyle/GameMode.qml || fail 'MRX game mode does not snapshot the original border width'
grep -Fq 'if (!root.mrxSnapshotLoaded || !root.mrxRestoreStateLoaded) return' shell/modules/ii/sidebarRight/quickToggles/classicStyle/GameMode.qml || fail 'MRX game mode does not wait for both startup state sources'

grep -Fq 'root.currentWindowGaps === 0 && root.currentBorderWidth === 1' shell/modules/ii/sidebarRight/quickToggles/classicStyle/GameMode.qml || fail 'MRX game mode initial state is inverted or misdetected'
grep -Fq 'String(root.savedWindowGaps)' shell/modules/ii/sidebarRight/quickToggles/classicStyle/GameMode.qml || fail 'MRX game mode does not restore the previous gap'
grep -Fq 'String(root.savedBorderWidth)' shell/modules/ii/sidebarRight/quickToggles/classicStyle/GameMode.qml || fail 'MRX game mode does not restore the previous border width'
grep -Fq 'property string mrxRestorePath:' shell/modules/ii/sidebarRight/quickToggles/classicStyle/GameMode.qml || fail 'MRX game mode restore state is not persisted'
grep -Fq 'command: ["cat", root.mrxRestorePath]' shell/modules/ii/sidebarRight/quickToggles/classicStyle/GameMode.qml || fail 'MRX game mode does not reload its restore state'
grep -Fq 'mrxctl set appearance.border_width 1' shell/modules/ii/sidebarRight/quickToggles/classicStyle/GameMode.qml || fail 'MRX game mode does not apply its temporary border setting'
grep -Fq 'rm -f "$1"' shell/modules/ii/sidebarRight/quickToggles/classicStyle/GameMode.qml || fail 'MRX game mode does not clear its restore state after reverting'

grep -Fq 'active: WM.compositor === "hyprland"' shell/services/CompositorGlobalShortcut.qml || fail 'global shortcuts are not scoped to Hyprland'
grep -Fq 'global shell shortcuts managed by MRX' shell/modules/ii/settings/pages/MRXConfig.qml || fail 'MRX settings page does not disclose global shortcut limitation'
grep -Fq 'Hyprland-only shader and compositor controls are hidden' shell/modules/ii/settings/pages/MRXConfig.qml || fail 'MRX settings page does not disclose compositor-only control boundary'

echo "PASS: MRX shell capabilities disable unsupported night-light and Hyprland-only controls while retaining supported X11 brightness"
