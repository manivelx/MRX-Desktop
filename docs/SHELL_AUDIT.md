# Quickshell and X11 compatibility audit

## Scope

This audit covers the supplied end4-pC-derived Quickshell shell when launched by the MRX X11 session. A component being importable or a Quickshell process remaining alive does not prove that its controls behave correctly on X11.

Status labels:
- **X11 path exists** — the project has an explicit X11 implementation.
- **External fallback** — MRX delegates to a standard X11/session utility.
- **Wayland/Hyprland-only** — the current implementation depends on a Wayland or Hyprland API.
- **Not validated** — the code path exists, but it still needs a real interactive test.

## Feature audit

| Feature | Current implementation | X11 status | Remaining work |
|---|---|---|---|
| Window/workspace state | `MRXBackend` reads the authoritative state and actions through `mrxctl`/MRX IPC. | **X11 path exists** | Exercise every action in a complete session, including error/reconnect cases. |
| Global shortcuts | `CompositorGlobalShortcut.qml` activates the Quickshell global-shortcut component only for Hyprland. | **Wayland/Hyprland-only** | Add an X11 shortcut layer. The lowest-risk first step is optional `sxhkd`; a single MRX TOML keybinding source can be considered later. |
| Focus grabs and popup dismissal | Shared focus-grab code uses `HyprlandFocusGrab`. X11 `PanelWindow` components cannot use the removed, unsupported `onActiveChanged` handler. | **Wayland/Hyprland-only / incomplete on X11** | Add an X11 focus/active-window dismissal path and test Escape, outside-click and focus transfer for each popup. |
| Lock screen | The imported `LockScreen` component uses Quickshell's Wayland lock integration. The MRX session separately starts `xss-lock` with the user's existing `~/.local/share/quickshell-lockscreen/lock.sh` when that executable exists. | **External fallback** | Test lock, unlock and suspend/resume on Xorg. Do not replace or edit the user's existing lock-screen files. |
| Compositing and effects | `mrx-session` starts Picom if it is installed and not already running. The installer preserves the user's Picom configuration. | **External fallback** | Validate Picom's GLX backend on the GT 740/470xx driver; test an XRender or no-compositor fallback. Do not build a custom compositor before this is stable. |
| System tray | The shell uses `Quickshell.Services.SystemTray`. | **Not validated** | Verify StatusNotifier items and legacy XEmbed applications on X11; add a bridge only if real applications require it. |
| Idle inhibition | The shell has an X11-compatible `systemd-inhibit` path for keep-awake behavior. | **X11 path exists** | Test inhibit/uninhibit, DPMS, screen blanking and lock timing. |
| Notifications | Quickshell's notification service is D-Bus based. | **Expected to work; not validated** | Test notification delivery, actions, persistence and daemon coexistence on X11. |
| Wallpaper | MRX uses its X11 static-wallpaper path and monitor/cursor geometry. Video wallpapers are explicitly unsupported. | **X11 path exists** | Test multiple monitors, hotplug and wallpaper restoration after shell restart. |
| Screen recording | MRX uses FFmpeg `x11grab` and `slop` for X11 sessions. | **X11 path exists** | Test capture region, audio inclusion, stop/cancel behavior and output files. |
| Session/autostart | `mrx-session` launches the WM supervisor, Quickshell, optional Picom and the existing X11 lock launcher without rewriting SDDM or the user's lock-screen configuration. | **X11 path exists** | Test login, logout, restart, failure recovery and coexistence with the existing BSPWM/SDDM setup. |

## Automated validation boundary

The Arch CI Quickshell smoke test:

1. Installs MRX into a temporary HOME/XDG tree.
2. Starts a disposable Xvfb server and private D-Bus session.
3. Launches the actual MRX session.
4. Verifies MRX IPC health and that Quickshell remains in the session process tree.
5. Stops only that session's descendant processes and removes the temporary files.

This proves configuration startup and IPC connectivity. It does **not** prove visible rendering, interactive popup behavior, tray compatibility, lock/unlock, GLX acceleration, multi-monitor behavior or physical GT 740 compatibility.

## Accepted implementation direction

- Use Picom first for shadows, transparency and rounded corners; keep it optional.
- Use `sxhkd` as the initial X11 global-keybinding bridge rather than adding a second large keybinding engine to the Rust WM immediately.
- Keep the existing MRX IPC event stream. The normal Quickshell snapshot path is already push/event driven; a 100 ms timeout in subscriber lifecycle handling is not the same as polling the entire desktop state.
- Keep a no-compositor fallback and defer custom composition/workspace animations until renderer ownership, recovery and hardware performance are proven.
- Preserve explicit unsupported states for controls that still call Hyprland-only APIs.
