pragma ComponentBehavior: Bound
import QtQuick
import Quickshell
import Quickshell.Io

Scope {
    id: root
    property var windowList: []
    property var workspaces: []
    property var workspaceById: ({})
    property var activeWorkspace: null
    property var monitors: []
    property var focusedMonitor: null
    property var windowByAddress: ({})
    property var addresses: []
    property string activeWindowNodeId: ""
    property string activeWindowXid: ""

    function hexId(id) {
        if (id === undefined || id === null) return ""
        const n = Number(id)
        return "0x" + n.toString(16)
    }

    function visitNode(node, desktop, output) {
        if (!node || typeof node !== "object") return
        if (node.client && !node.vacant) {
            const c = node.client
            const rect = node.rectangle ?? c.tiledRectangle ?? c.floatingRectangle ?? ({})
            const id = hexId(node.id)
            output.push({
                id: id,
                address: id,
                title: c.name ?? c.title ?? c.className ?? c.instanceName ?? "",
                appId: c.className ?? c.instanceName ?? "",
                class: c.className ?? "",
                instance: c.instanceName ?? "",
                xid: c.window ?? c.xid ?? null,
                workspaceId: desktop.idx,
                workspaceName: desktop.name,
                workspace: { id: desktop.idx, name: desktop.name },
                monitorName: desktop.output,
                focused: !!desktop.is_active && !!desktop.monitorFocused && Number(node.id) === Number(desktop.focusedNodeId),
                activated: !!desktop.is_active && !!desktop.monitorFocused && Number(node.id) === Number(desktop.focusedNodeId),
                width: rect.width ?? 0,
                height: rect.height ?? 0,
                x: rect.x ?? 0,
                y: rect.y ?? 0,
                size: [rect.width ?? 0, rect.height ?? 0],
                at: [rect.x ?? 0, rect.y ?? 0],
                state: (c.fullscreen === true || c.state === "fullscreen") ? "fullscreen" : (c.state ?? (c.floating ? "floating" : "tiled")),
                fullscreen: c.fullscreen === true || c.state === "fullscreen",
                urgent: c.urgent ?? false,
                hidden: node.hidden ?? c.hidden ?? false,
                floating: c.state === "floating" || c.floating === true
            })
        }
        visitNode(node.firstChild, desktop, output)
        visitNode(node.secondChild, desktop, output)
    }

    function applySnapshot(raw) {
        if (!raw || !Array.isArray(raw.monitors)) return
        let nextMonitors = []
        let nextWorkspaces = []
        let nextWindows = []
        let byWorkspace = {}
        let byAddress = {}
        let focusedWs = null
        const focusedMonitorId = raw.focusedMonitorId === undefined ? null : Number(raw.focusedMonitorId)

        for (const mon of raw.monitors) {
            const monName = mon.name ?? String(mon.id)
            const r = mon.rectangle ?? ({})
            const monitor = {
                id: String(mon.id), name: monName,
                x: r.x ?? 0, y: r.y ?? 0,
                width: r.width ?? 0, height: r.height ?? 0,
                scale: 1, focused: (focusedMonitorId !== null && Number(mon.id) === focusedMonitorId) || mon.focused === true,
                activeWorkspace: null
            }
            const desktops = Array.isArray(mon.desktops) ? mon.desktops : []
            for (let i = 0; i < desktops.length; i++) {
                const d = desktops[i]
                const dName = String(d.name ?? (i + 1))
                const parsed = Number.parseInt(dName, 10)
                const idx = Number.isFinite(parsed) && parsed > 0 ? parsed : i + 1
                const ws = {
                    id: idx, idx: idx, name: dName,
                    output: monName,
                    is_active: !!d.focused,
                    focused: !!d.focused,
                    monitorFocused: monitor.focused,
                    windows: 0,
                    focusedNodeId: d.focusedNodeId ?? d.focusHistory?.[0] ?? mon.focusedNodeId ?? 0,
                    root: d.root ?? null
                }
                ws.is_active = (d.focused === true) || (mon.focusedDesktopId !== undefined && String(mon.focusedDesktopId) === String(d.id))
                ws.focused = ws.is_active
                ws.monitorFocused = monitor.focused
                nextWorkspaces.push(ws)
                byWorkspace[idx] = ws
                if (ws.is_active) {
                    monitor.activeWorkspace = { id: idx, name: dName, idx: idx }
                    focusedWs = ws
                }
                const before = nextWindows.length
                visitNode(d.root, ws, nextWindows)
                ws.windows = nextWindows.length - before
            }
            nextMonitors.push(monitor)
        }
        for (const win of nextWindows) byAddress[win.address] = win
        root.monitors = nextMonitors
        root.workspaces = nextWorkspaces
        root.workspaceById = byWorkspace
        root.windowList = nextWindows
        root.windowByAddress = byAddress
        root.addresses = nextWindows.map(w => w.address)
        root.focusedMonitor = nextMonitors.find(m => m.focused) ?? nextMonitors[0] ?? null
        root.activeWorkspace = nextWorkspaces.find(w => w.output === root.focusedMonitor?.name && w.is_active) ?? focusedWs
        const activeWindow = nextWindows.find(w => w.focused)
        root.activeWindowNodeId = activeWindow?.id ?? ""
        root.activeWindowXid = activeWindow?.xid ? String(activeWindow.xid) : ""
        if (root.activeWindowXid) {
            activeTitleProc.command = ["xprop", "-id", root.activeWindowXid, "_NET_WM_NAME", "WM_NAME"]
            activeTitleProc.running = true
        }
    }

    function refresh() {
        if (!snapshot.running) snapshot.running = true
    }

    function switchWorkspaceRelative(direction) {
        action.command = ["bspc", "desktop", "-f", direction === "next" ? "next.local" : "prev.local"]
        action.running = true
    }
    function switchWorkspace(id, monitorName = "") {
        const focusedOutput = monitorName || root.focusedMonitor?.name
        const ws = root.workspaces.find(w => w.output === focusedOutput && (Number(w.id) === Number(id) || w.name === String(id)))
            ?? root.workspaces.find(w => Number(w.id) === Number(id) || w.name === String(id))
        if (!ws) return
        action.command = ["bspc", "desktop", "-f", ws.name]
        action.running = true
    }
    function focusWindow(id) {
        action.command = ["bspc", "node", String(id), "-f"]
        action.running = true
    }
    function closeWindow(id) {
        action.command = ["bspc", "node", String(id), "-c"]
        action.running = true
    }
    function moveWindowToWorkspace(id, wsId) {
        const source = root.windowList.find(w => w.id === String(id) || w.address === String(id))
        const ws = root.workspaces.find(w => w.output === (source?.monitorName ?? root.focusedMonitor?.name)
            && (Number(w.id) === Number(wsId) || w.name === String(wsId)))
            ?? root.workspaces.find(w => Number(w.id) === Number(wsId) || w.name === String(wsId))
        if (!ws) return
        action.command = ["bspc", "node", String(id), "-d", ws.name]
        action.running = true
    }
    function monitorFor(screen) {
        return root.monitors.find(m => m.name === screen?.name) ?? null
    }
    function activeWorkspaceForMonitor(monitorName) {
        return root.workspaces.find(w => w.output === monitorName && w.is_active) ?? null
    }
    function biggestWindowForWorkspace(wsId) {
        const focusedOutput = root.focusedMonitor?.name
        const wins = root.windowList.filter(w => Number(w.workspaceId) === Number(wsId) && (!focusedOutput || w.monitorName === focusedOutput))
        if (!wins.length) return null
        return wins.reduce((a, b) => (a.width * a.height >= b.width * b.height ? a : b))
    }
    function fullscreenOnMonitor(monitorName) {
        const ws = root.activeWorkspaceForMonitor(monitorName)
        return !!ws && root.windowList.some(w => w.workspaceId === ws.id && w.state === "fullscreen")
    }
    function monitorGeometry(screen) {
        const m = root.monitorFor(screen)
        return m ? { x: m.x, y: m.y, scale: 1 } : { x: 0, y: 0, scale: 1 }
    }

    Process {
        id: snapshot
        command: ["bspc", "wm", "-d"]
        stdout: StdioCollector {
            onStreamFinished: {
                try { root.applySnapshot(JSON.parse(text)) }
                catch (e) { console.warn("[BSPWM] Could not parse bspc wm -d:", e) }
            }
        }
    }
    Process {
        id: activeTitleProc
        stdout: StdioCollector {
            onStreamFinished: {
                const match = text.match(/(?:_NET_WM_NAME|WM_NAME)\([^)]*\) = "([^"]*)"/)
                if (!match || !root.activeWindowNodeId) return
                const updated = root.windowList.map(w => {
                    if (w.id !== root.activeWindowNodeId) return w
                    const copy = Object.assign({}, w)
                    copy.title = match[1]
                    return copy
                })
                root.windowList = updated
                const byAddress = {}
                for (const win of updated) byAddress[win.address] = win
                root.windowByAddress = byAddress
            }
        }
    }
    Process {
        id: action
        onExited: root.refresh()
    }
    Process {
        id: events
        command: ["bspc", "subscribe"]
        stdout: SplitParser {
            splitMarker: "\n"
            onRead: line => refreshDebounce.restart()
        }
        onExited: eventRestart.restart()
    }
    Timer { id: refreshDebounce; interval: 90; onTriggered: root.refresh() }
    Timer { id: eventRestart; interval: 900; onTriggered: events.running = true }
    Timer { interval: 1800; running: true; repeat: true; onTriggered: root.refresh() }
    Component.onCompleted: { root.refresh(); events.running = true }
}
