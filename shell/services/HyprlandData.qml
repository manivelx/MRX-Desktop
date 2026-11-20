pragma Singleton
pragma ComponentBehavior: Bound

import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Hyprland
import qs.services

/**
 * Provides access to some Hyprland data not available in Quickshell.Hyprland.
 */
Singleton {
    id: root
    property var _hyprWindowList: []
    property var _hyprAddresses: []
    property var _hyprWindowByAddress: ({})
    property var _hyprWorkspaces: []
    property var _hyprWorkspaceIds: []
    property var _hyprWorkspaceById: ({})
    property var _hyprActiveWorkspace: null
    property var _hyprMonitors: []
    property var _hyprLayers: ({})
    readonly property var windowList: WM.isX11 ? WM.windowList : _hyprWindowList
    readonly property var addresses: WM.isX11 ? WM.addresses : _hyprAddresses
    readonly property var windowByAddress: WM.isX11 ? WM.windowByAddress : _hyprWindowByAddress
    readonly property var workspaces: WM.isX11 ? WM.workspaces : _hyprWorkspaces
    readonly property var workspaceIds: WM.isX11 ? WM.workspaces.map(w => w.id) : _hyprWorkspaceIds
    readonly property var workspaceById: WM.isX11 ? WM.workspaceById : _hyprWorkspaceById
    readonly property var activeWorkspace: WM.isX11 ? WM.activeWorkspace : _hyprActiveWorkspace
    readonly property var monitors: WM.isX11 ? WM.monitors : _hyprMonitors
    readonly property var layers: WM.isX11 ? ({}) : _hyprLayers

    // Convenient stuff

    function toplevelsForWorkspace(workspace) {
        if (WM.isX11) return [];
        return ToplevelManager.toplevels.values.filter(toplevel => {
            const address = `0x${toplevel.HyprlandToplevel?.address}`;
            var win = HyprlandData.windowByAddress[address];
            return win?.workspace?.id === workspace;
        })
    }

    function hyprlandClientsForWorkspace(workspace) {
        return root.windowList.filter(win => WM.isX11 ? win.workspaceId === workspace : win.workspace?.id === workspace);
    }

    function clientForToplevel(toplevel) {
        if (WM.isX11 || !toplevel || !toplevel.HyprlandToplevel) {
            return null;
        }
        const address = `0x${toplevel?.HyprlandToplevel?.address}`;
        return root.windowByAddress[address];
    }

    // Internals

    property bool _pendingClients: false
    property bool _pendingMonitors: false
    property bool _pendingLayers: false
    property bool _pendingWorkspaces: false

    Timer {
        id: eventDebounceTimer
        interval: 60
        repeat: false
        onTriggered: {
            if (WM.compositor !== "hyprland") return;
            if (root._pendingClients) {
                getClients.running = false;
                getClients.running = true;
                root._pendingClients = false;
            }
            if (root._pendingMonitors) {
                getMonitors.running = false;
                getMonitors.running = true;
                root._pendingMonitors = false;
            }
            if (root._pendingLayers) {
                getLayers.running = false;
                getLayers.running = true;
                root._pendingLayers = false;
            }
            if (root._pendingWorkspaces) {
                getWorkspaces.running = false;
                getWorkspaces.running = true;
                getActiveWorkspace.running = false;
                getActiveWorkspace.running = true;
                root._pendingWorkspaces = false;
            }
        }
    }

    function queueUpdate(clients = false, workspaces = false, monitors = false, layers = false) {
        if (WM.compositor !== "hyprland") return;
        if (clients) root._pendingClients = true;
        if (workspaces) root._pendingWorkspaces = true;
        if (monitors) root._pendingMonitors = true;
        if (layers) root._pendingLayers = true;
        eventDebounceTimer.restart();
    }

    function updateWindowList() {
        queueUpdate(true, false, false, false);
    }

    function updateLayers() {
        queueUpdate(false, false, false, true);
    }

    function updateMonitors() {
        queueUpdate(false, false, true, false);
    }

    function updateWorkspaces() {
        queueUpdate(false, true, false, false);
    }

    function updateAll() {
        queueUpdate(true, true, true, true);
    }

    function biggestWindowForWorkspace(workspaceId) {
        if (WM.isX11) return WM.biggestWindowForWorkspace(workspaceId);
        const windowsInThisWorkspace = HyprlandData.windowList.filter(w => w.workspace.id == workspaceId);
        return windowsInThisWorkspace.reduce((maxWin, win) => {
            const maxArea = (maxWin?.size?.[0] ?? 0) * (maxWin?.size?.[1] ?? 0);
            const winArea = (win?.size?.[0] ?? 0) * (win?.size?.[1] ?? 0);
            return winArea > maxArea ? win : maxWin;
        }, null);
    }

    Component.onCompleted: {
        if (WM.compositor === "hyprland") {
            getClients.running = true;
            getMonitors.running = true;
            getLayers.running = true;
            getWorkspaces.running = true;
            getActiveWorkspace.running = true;
        }
    }

    Connections {
        target: Hyprland
        enabled: WM.compositor === "hyprland"

        function onRawEvent(event) {
            const name = event.name;
            if (["openlayer", "closelayer", "screencast", "submap", "activelayout"].includes(name)) return;

            if (name.startsWith("workspace") || name.startsWith("createworkspace") || name.startsWith("destroyworkspace") || name.startsWith("moveworkspace") || name === "renameworkspace") {
                root.queueUpdate(false, true, false, false);
            } else if (name.startsWith("activespecial")) {
                root.queueUpdate(false, true, true, false);
            } else if (name.startsWith("openwindow") || name.startsWith("closewindow") || name.startsWith("movewindow")) {
                root.queueUpdate(true, true, false, false);
            } else if (name.startsWith("window") || name.startsWith("activewindow") || name === "fullscreen" || name === "changefloatingmode" || name === "pin" || name === "urgent" || name === "minimize") {
                root.queueUpdate(true, false, false, false);
            } else if (name.startsWith("monitor") || name === "focusedmon") {
                root.queueUpdate(false, true, true, false);
            } else {
                root.queueUpdate(true, true, false, false);
            }
        }
    }

    Process {
        id: getClients
        command: ["hyprctl", "clients", "-j"]
        stdout: StdioCollector {
            id: clientsCollector
            onStreamFinished: {
                try {
                    root._hyprWindowList = JSON.parse(clientsCollector.text);
                    let tempWinByAddress = {};
                    for (var i = 0; i < root.windowList.length; ++i) {
                        var win = root.windowList[i];
                        tempWinByAddress[win.address] = win;
                    }
                    root._hyprWindowByAddress = tempWinByAddress;
                    root._hyprAddresses = root._hyprWindowList.map(win => win.address);
                } catch (e) {
                    console.error("[HyprlandData] Failed to parse clients:", e);
                }
            }
        }
    }

    Process {
        id: getMonitors
        command: ["hyprctl", "monitors", "-j"]
        stdout: StdioCollector {
            id: monitorsCollector
            onStreamFinished: {
                try {
                    root._hyprMonitors = JSON.parse(monitorsCollector.text);
                } catch (e) {
                    console.error("[HyprlandData] Failed to parse monitors:", e);
                }
            }
        }
    }

    Process {
        id: getLayers
        command: ["hyprctl", "layers", "-j"]
        stdout: StdioCollector {
            id: layersCollector
            onStreamFinished: {
                try {
                    root._hyprLayers = JSON.parse(layersCollector.text);
                } catch (e) {
                    console.error("[HyprlandData] Failed to parse layers:", e);
                }
            }
        }
    }

    Process {
        id: getWorkspaces
        command: ["hyprctl", "workspaces", "-j"]
        stdout: StdioCollector {
            id: workspacesCollector
            onStreamFinished: {
                try {
                    var rawWorkspaces = JSON.parse(workspacesCollector.text);
                    root._hyprWorkspaces = rawWorkspaces.filter(ws => ws.id >= 1 && ws.id <= 100);
                    let tempWorkspaceById = {};
                    for (var i = 0; i < root._hyprWorkspaces.length; ++i) {
                        var ws = root._hyprWorkspaces[i];
                        tempWorkspaceById[ws.id] = ws;
                    }
                    root._hyprWorkspaceById = tempWorkspaceById;
                    root._hyprWorkspaceIds = root._hyprWorkspaces.map(ws => ws.id);
                } catch (e) {
                    console.error("[HyprlandData] Failed to parse workspaces:", e);
                }
            }
        }
    }

    Process {
        id: getActiveWorkspace
        command: ["hyprctl", "activeworkspace", "-j"]
        stdout: StdioCollector {
            id: activeWorkspaceCollector
            onStreamFinished: {
                try {
                    root._hyprActiveWorkspace = JSON.parse(activeWorkspaceCollector.text);
                } catch (e) {
                    console.error("[HyprlandData] Failed to parse active workspace:", e);
                }
            }
        }
    }
}