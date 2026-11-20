pragma Singleton
pragma ComponentBehavior: Bound
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland

Singleton {
    id: root
    readonly property string compositor: detectCompositor()
    readonly property bool isX11: compositor === "bspwm" || compositor === "mrx"
    property var backend: null

    function detectCompositor() {
        const desktop = (Quickshell.env("XDG_CURRENT_DESKTOP") ?? "").toLowerCase()
        const session = (Quickshell.env("XDG_SESSION_DESKTOP") ?? "").toLowerCase()
        const combined = desktop + " " + session
        if (combined.includes("mrx")) return "mrx"
        if (combined.includes("bspwm") || Quickshell.env("BSPWM_SOCKET")) return "bspwm"
        if (combined.includes("hyprland")) return "hyprland"
        if (combined.includes("niri")) return "niri"
        if (combined.includes("sway")) return "sway"
        return "unknown"
    }

    Component { id: hyprlandComp; HyprlandBackend {} }
    Component { id: niriComp; NiriBackend {} }
    Component { id: bspwmComp; BspwmBackend {} }
    Component { id: mrxComp; MRXBackend {} }

    Component.onCompleted: {
        switch (root.compositor) {
        case "hyprland": backend = hyprlandComp.createObject(root); break
        case "niri": backend = niriComp.createObject(root); break
        case "bspwm": backend = bspwmComp.createObject(root); break
        case "mrx": backend = mrxComp.createObject(root); break
        default: console.error("[WM] Unsupported window manager:", root.compositor)
        }
    }

    readonly property var windowList: backend?.windowList ?? []
    readonly property var workspaces: backend?.workspaces ?? []
    readonly property var workspaceById: backend?.workspaceById ?? ({})
    readonly property var activeWorkspace: backend?.activeWorkspace ?? null
    readonly property var monitors: backend?.monitors ?? []
    readonly property var focusedMonitor: backend?.focusedMonitor ?? null
    readonly property var windowByAddress: backend?.windowByAddress ?? ({})
    readonly property var addresses: backend?.addresses ?? []
    readonly property var settings: backend?.settings ?? ({})
    readonly property var focusedWindow: windowList.find(w => w.focused || w.activated) ?? null

    function focusWindow(id) { backend?.focusWindow(id) }
    function closeWindow(id) { backend?.closeWindow(id) }
    function switchWorkspace(id) { backend?.switchWorkspace(id) }
    function switchWorkspaceOnMonitor(id, monitorName) {
        if (root.compositor === "bspwm") backend?.switchWorkspace(id, monitorName)
        else backend?.switchWorkspace(id)
    }
    function switchWorkspaceRelative(direction) { backend?.switchWorkspaceRelative(direction) }
    function moveWindowToWorkspace(id, wsId) { backend?.moveWindowToWorkspace(id, wsId) }
    function monitorFor(screen) { return backend?.monitorFor(screen) ?? null }
    function activeWorkspaceForMonitor(name) { return backend?.activeWorkspaceForMonitor(name) ?? null }
    function biggestWindowForWorkspace(id) { return backend?.biggestWindowForWorkspace(id) ?? null }
    function fullscreenOnMonitor(name) { return backend?.fullscreenOnMonitor(name) ?? false }
    function monitorGeometry(screen) { return backend?.monitorGeometry(screen) ?? { x: 0, y: 0, scale: 1 } }

    function setConfig(key, value) {
        if (root.compositor === "mrx") {
            const map = { window_gap: "appearance.gaps", border_width: "appearance.border_width", focus_follows_pointer: "window.focus_follows_pointer", split_ratio: "window.master_ratio", layout: "window.layout" }
            const target = map[String(key)]
            if (target) Quickshell.execDetached(["mrxctl", "set", target, JSON.stringify(value)])
            return
        }
        if (root.compositor !== "bspwm") return
        Quickshell.execDetached(["bspc", "config", String(key), String(value)])
    }

    // Compatibility dispatcher for the original end4-pC QML actions.
    function dispatch(command) {
        if (root.compositor === "hyprland") { Hyprland.dispatch(command); return }
        const s = String(command)
        if (root.compositor === "niri") {
            if (/workspace\s+r?\+1/.test(s)) { root.switchWorkspaceRelative("next"); return }
            if (/workspace\s+r?-1/.test(s)) { root.switchWorkspaceRelative("previous"); return }
            let nm = s.match(/window\s*=\s*"address:([^"]+)"/)
            if (nm && /close/.test(s)) { root.closeWindow(nm[1]); return }
            if (nm && /(moveToWorkspace|window\.move)/.test(s)) {
                const ws = s.match(/workspace\s*=\s*(-?\d+)/)
                if (ws) root.moveWindowToWorkspace(nm[1], Number(ws[1]))
                return
            }
            if (nm && /focus|activate/.test(s)) { root.focusWindow(nm[1]); return }
            nm = s.match(/workspace\s*=\s*(-?\d+)/)
            if (nm) { root.switchWorkspace(Number(nm[1])); return }
            return
        }
        if (root.compositor === "mrx") {
            if (/workspace\s+r?\+1/.test(s)) { root.switchWorkspaceRelative("next"); return }
            if (/workspace\s+r?-1/.test(s)) { root.switchWorkspaceRelative("previous"); return }
            let am = s.match(/address:([0-9]+)/)
            if (am && /close/.test(s)) { root.closeWindow(am[1]); return }
            if (am && /focus|activate/.test(s)) { root.focusWindow(am[1]); return }
            if (am && /(moveToWorkspace|window\.move)/.test(s)) { const target = s.match(/workspace\s*=\s*(-?\d+)/); if (target) root.moveWindowToWorkspace(am[1], Number(target[1])); return }
            am = s.match(/workspace\s*=\s*(-?\d+)/)
            if (am) root.switchWorkspace(Number(am[1]))
            return
        }
        if (root.compositor !== "bspwm") return
        if (s.includes("quickshell:wallpaperSelectorToggle")) {
            Quickshell.execDetached(["qs", "-c", "mrx", "ipc", "call", "end4", "toggleWallpaperSelector"])
            return
        }
        if (/workspace\s+r?\+1/.test(s)) { root.switchWorkspaceRelative("next"); return }
        if (/workspace\s+r?-1/.test(s)) { root.switchWorkspaceRelative("previous"); return }
        let m = s.match(/window\s*=\s*"address:([^"]+)"/)
        const address = m?.[1]
        if (address && /close/.test(s)) { root.closeWindow(address); return }
        if (address && /(moveToWorkspace|window\.move)/.test(s)) {
            const ws = s.match(/workspace\s*=\s*(-?\d+)/)
            if (ws) root.moveWindowToWorkspace(address, Number(ws[1]))
            return
        }
        if (address && /window\.swap/.test(s)) {
            const second = s.match(/address:([0-9a-fx]+)/g)
            if (second?.length > 1) Quickshell.execDetached(["bspc", "node", second[0].replace("address:", ""), "-s", second[1].replace("address:", "")])
            return
        }
        if (address && /focus|activate/.test(s)) { root.focusWindow(address); return }
        m = s.match(/workspace\s*=\s*(-?\d+)/)
        if (m) { root.switchWorkspace(Number(m[1])); return }
        // Hyprland cursor warp settings have no BSPWM equivalent; safely ignore.
    }
}
