pragma Singleton
import qs.modules.common
import qs.services
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

/**
 * A nice wrapper for date and time strings.
 */
Singleton {
    id: root

    property bool inhibit: false

    onInhibitChanged: {
        if (WM.isX11) x11Inhibitor.running = root.inhibit;
        else idleInhibitor.enabled = root.inhibit;
    }

    Connections {
        target: Persistent
        function onReadyChanged() {
            root.inhibit = Persistent.states.idle.inhibit;
        }
    }

    function toggleInhibit(active = null) {
        if (active !== null) {
            root.inhibit = active;
        } else {
            root.inhibit = !root.inhibit;
        }
        Persistent.states.idle.inhibit = root.inhibit;
    }

    Process {
        id: x11Inhibitor
        running: WM.isX11 && root.inhibit
        command: ["systemd-inhibit", "--what=idle:sleep", "--who=mrx", "--why=User requested keep-awake", "--mode=block", "sleep", "infinity"]
    }

    IdleInhibitor {
        id: idleInhibitor
        enabled: !WM.isX11 && root.inhibit
        window: PanelWindow {
            // Inhibitor requires a "visible" surface
            // Actually not lol
            implicitWidth: 0
            implicitHeight: 0
            color: "transparent"
            // Just in case...
            anchors {
                right: true
                bottom: true
            }
            // Make it not interactable
            mask: Region {
                item: null
            }
        }
    }
}