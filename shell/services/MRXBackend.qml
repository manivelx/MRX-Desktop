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
    property var settings: ({})
    property bool streamHealthy: false

    function refresh() { if (!snapshot.running) snapshot.running = true }
    function apply(raw) {
        if (!raw || !Array.isArray(raw.workspaces)) return
        root.settings = raw.settings ?? ({})
        const monitorData = (raw.monitors ?? []).map(m => ({id:m.name,name:m.name,x:m.x,y:m.y,width:m.width,height:m.height,scale:1,focused:m.focused,active_workspace:m.active_workspace ?? 1}))
        const primaryMonitor = monitorData.find(m => m.focused) ?? monitorData[0] ?? null
        root.monitors = monitorData
        root.workspaces = raw.workspaces.map(w => {
            const output = w.monitor ?? primaryMonitor?.name ?? ""
            const monitor = monitorData.find(m => m.name === output)
            return ({id:w.id, idx:w.id, name:w.name, output, is_active:w.active, focused:Boolean(w.active && monitor?.focused), windows:w.windows, monitorFocused:Boolean(monitor?.focused)})
        })
        // Workspace IDs are local to a monitor; never key this map by ID alone.
        const byWs = {}
        for (const w of root.workspaces) {
            const key = `${w.output}:${w.id}`
            byWs[key] = w
        }
        // Preserve the legacy numeric lookup as the focused monitor view.
        if (primaryMonitor) {
            for (const w of root.workspaces.filter(item => item.output === primaryMonitor.name))
                byWs[w.id] = w
        }
        root.workspaceById = byWs
        root.activeWorkspace = root.workspaces.find(w => w.output === primaryMonitor?.name && w.is_active) ?? root.workspaces[0] ?? null
        root.windowList = (raw.windows ?? []).filter(w => !w.dock).map(w => {
            const cx = w.x + w.width / 2, cy = w.y + w.height / 2
            const monitor = monitorData.find(m => m.name === w.monitor) ?? monitorData.find(m => cx >= m.x && cx < m.x + m.width && cy >= m.y && cy < m.y + m.height) ?? primaryMonitor
            return ({id:String(w.id), address:String(w.id), xid:w.id, title:w.title, appId:w.class, class:w.class, instance:w.class, workspaceId:w.workspace, workspaceName:(root.workspaceById[`${monitor?.name ?? ""}:${w.workspace}`]?.name ?? String(w.workspace)), workspace:{id:w.workspace,name:(root.workspaceById[`${monitor?.name ?? ""}:${w.workspace}`]?.name ?? String(w.workspace))}, monitorName:monitor?.name ?? "", focused:w.focused, activated:w.focused, width:w.width, height:w.height, x:w.x, y:w.y, size:[w.width,w.height], at:[w.x,w.y], state:w.fullscreen?"fullscreen":(w.floating?"floating":"tiled"), fullscreen:w.fullscreen, floating:w.floating, urgent:w.urgent ?? false, hidden:Number(w.workspace)!==Number(monitor?.active_workspace ?? raw.active_workspace)})
        })
        const byAddr={}; for (const w of root.windowList) byAddr[w.address]=w
        root.windowByAddress=byAddr; root.addresses=root.windowList.map(w=>w.address)
        root.focusedMonitor=primaryMonitor ? {...primaryMonitor, activeWorkspace:root.activeWorkspace} : null
    }
    function run(args) { action.command = ["mrxctl", ...args.map(String)]; action.running = true }
    function switchWorkspace(id) { run(["workspace", id]) }
    function switchWorkspaceOnMonitor(id, monitorName) { run(["workspace", id, monitorName]) }
    function switchWorkspaceRelative(direction) { run(["workspace", direction]) }
    function focusWindow(id) { run(["focus", Number(id)]) }
    function closeWindow(id) { run(["close", Number(id)]) }
    function moveWindowToWorkspace(id, ws) { run(["move", Number(id), Number(ws)]) }
    function monitorFor(screen) { return root.monitors.find(m=>m.name===screen?.name)??root.focusedMonitor }
    function activeWorkspaceForMonitor(name) { return root.workspaces.find(w => w.output === name && w.is_active) ?? root.activeWorkspace }
    function biggestWindowForWorkspace(id, monitorName) { const a=root.windowList.filter(w=>w.workspaceId===Number(id) && (!monitorName || w.monitorName===monitorName));return a.sort((x,y)=>y.width*y.height-x.width*x.height)[0]??null }
    function fullscreenOnMonitor(name) { const active=root.activeWorkspaceForMonitor(name);return root.windowList.some(w=>w.fullscreen&&w.monitorName===name&&w.workspaceId===active?.id) }
    function monitorGeometry(screen) { const m=monitorFor(screen);return m?{x:m.x,y:m.y,scale:1}:{x:0,y:0,scale:1} }
    Process {
        id: snapshot
        command: ["mrxctl", "snapshot"]
        stdout: StdioCollector {
            onStreamFinished: {
                try { root.apply(JSON.parse(text)) }
                catch (e) { console.warn("[MRX] snapshot failed", e) }
            }
        }
    }
    Process {
        id: subscription
        command: ["mrxctl", "subscribe"]
        running: true
        stdout: SplitParser {
            onRead: data => {
                try {
                    root.apply(JSON.parse(data))
                    root.streamHealthy = true
                } catch (e) {
                    console.warn("[MRX] subscription event failed", e)
                }
            }
        }
        onRunningChanged: {
            if (!running) {
                root.streamHealthy = false
                reconnect.running = true
            }
        }
    }
    Timer {
        id: reconnect
        interval: 1500
        repeat: false
        onTriggered: subscription.running = true
    }
    Process { id: action; onExited: root.refresh() }
    // Keep a slow one-shot fallback while the event stream is unavailable.
    Timer {
        interval: 1500
        running: !root.streamHealthy
        repeat: true
        onTriggered: root.refresh()
    }
    Component.onCompleted: root.refresh()
}
