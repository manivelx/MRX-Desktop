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
    enabled: WM.compositor !== "mrx" || root.mrxSettingsLoaded

    onClicked: {
        root.toggled = !root.toggled
        if (WM.compositor === "mrx") {
            if (root.toggled) {
                Quickshell.execDetached(["mrxctl", "set", "appearance.gaps", "0"]); Quickshell.execDetached(["mrxctl", "set", "appearance.border_width", "1"])
            } else {
                Quickshell.execDetached(["mrxctl", "set", "appearance.gaps", String(root.savedWindowGaps)])
                Quickshell.execDetached(["mrxctl", "set", "appearance.border_width", String(root.savedBorderWidth)])
            }
        } else if (root.toggled) {
            Quickshell.execDetached(["bash", "-c", `hyprctl --batch "keyword animations:enabled 0; keyword decoration:shadow:enabled 0; keyword decoration:blur:enabled 0; keyword general:gaps_in 0; keyword general:gaps_out 0; keyword general:border_size 1; keyword decoration:rounding 0; keyword general:allow_tearing 1"`])
        } else {
            Quickshell.execDetached(["hyprctl", "reload"])
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
                        root.savedWindowGaps = Number(appearance.gaps ?? 8)
                        root.savedBorderWidth = Number(appearance.border_width ?? 2)
                        root.toggled = root.savedWindowGaps === 0 && root.savedBorderWidth === 1
                        root.mrxSettingsLoaded = true
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