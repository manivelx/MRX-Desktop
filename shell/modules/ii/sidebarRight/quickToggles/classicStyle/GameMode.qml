import qs.modules.common
import qs.modules.common.widgets
import qs.services
import Quickshell
import Quickshell.Io

QuickToggleButton {
    id: root
    buttonIcon: "gamepad"
    toggled: toggled
    property int savedWindowGaps: 8
    property int savedBorderWidth: 2
    property bool mrxSettingsLoaded: false
    property bool mrxRestoreStateLoaded: false
    property bool mrxRestoreStatePresent: false
    property bool mrxSnapshotLoaded: false
    property string mrxRestorePath: `${Quickshell.env("XDG_CONFIG_HOME") ?? (Quickshell.env("HOME") + "/.config")}/mrx/game-mode-restore.json`
    enabled: WM.compositor !== "mrx" || root.mrxSettingsLoaded

    onClicked: {
        root.toggled = !root.toggled
        if (WM.compositor === "mrx") {
            if (root.toggled) {
                Quickshell.execDetached(["bash", "-c",
                    `set -e; mkdir -p "$(dirname -- "$1")"; printf '%s' '{"gaps":${root.savedWindowGaps},"border_width":${root.savedBorderWidth}}' > "$1"; mrxctl set appearance.gaps 0; mrxctl set appearance.border_width 1`,
                    "mrx-game-mode", root.mrxRestorePath])
            } else {
                Quickshell.execDetached(["bash", "-c",
                    'set -e; mrxctl set appearance.gaps "$2"; mrxctl set appearance.border_width "$3"; rm -f "$1"',
                    "mrx-game-mode", root.mrxRestorePath, String(root.savedWindowGaps), String(root.savedBorderWidth)])
            }
        } else if (root.toggled) {
            Quickshell.execDetached(["bash", "-c", `hyprctl --batch "keyword animations:enabled 0; keyword decoration:shadow:enabled 0; keyword decoration:blur:enabled 0; keyword general:gaps_in 0; keyword general:gaps_out 0; keyword general:border_size 1; keyword decoration:rounding 0; keyword general:allow_tearing 1"`])
        } else {
            Quickshell.execDetached(["hyprctl", "reload"])
        }
    }
    Process {
        id: readSavedState
        running: WM.compositor === "mrx"
        command: ["cat", root.mrxRestorePath]
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    const saved = JSON.parse(text)
                    root.savedWindowGaps = Number(saved.gaps ?? 8)
                    root.savedBorderWidth = Number(saved.border_width ?? 2)
                    root.mrxRestoreStatePresent = true
                } catch (e) {
                    root.mrxRestoreStatePresent = false
                }
                root.mrxRestoreStateLoaded = true
                root.mrxSettingsLoaded = root.mrxSnapshotLoaded
            }
        }
    }
    Process {
        id: fetchActiveState
        running: true
        command: WM.compositor === "mrx" ? ["mrxctl", "snapshot"] : (WM.isX11
            ? ["bash", "-c", `test "$(bspc config window_gap)" -ne 0`]
            : ["bash", "-c", `test "$(hyprctl getoption animations:enabled -j | jq ".int")" -ne 0`])
        stdout: StdioCollector {
            onStreamFinished: {
                if (WM.compositor === "mrx") {
                    try {
                        const appearance = JSON.parse(text).settings.appearance ?? ({})
                        if (!root.mrxRestoreStatePresent) {
                            root.savedWindowGaps = Number(appearance.gaps ?? 8)
                            root.savedBorderWidth = Number(appearance.border_width ?? 2)
                        }
                        root.toggled = root.mrxRestoreStatePresent && Number(appearance.gaps ?? 8) === 0 && Number(appearance.border_width ?? 2) === 1
                        root.mrxSnapshotLoaded = true
                        root.mrxSettingsLoaded = root.mrxRestoreStateLoaded
                    } catch (e) {
                        console.warn("[MRX] game-mode state unavailable", e)
                    }
                }
            }
        }
        onExited: (exitCode, exitStatus) => {
            if (WM.compositor !== "mrx") root.toggled = exitCode !== 0
        }
    }
    StyledToolTip {
        text: Translation.tr("Game mode")
    }
}