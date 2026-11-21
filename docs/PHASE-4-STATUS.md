# Phase 4 — Quickshell integration status

## Implemented and CI-covered

Latest CI evidence: [PR #20 run](https://github.com/manivelx/MRX-Desktop/actions/runs/44914024803), commit `73cff5cdd7435f16c7781a106f84bbb2c4ee0a06`.

- MRXBackend obtains state from \`mrxctl\` and consumes the IPC snapshot stream with reconnect/fallback behavior.
- Wallpaper-selector search toggles Quickshell state directly; it no longer sends a Hyprland-only compositor IPC command.
- Shared \`mrx-session.sh\` identifies MRX as an X11 session and provides mrxctl monitor names, active-monitor selection, focused-monitor geometry and root display geometry.
- Static wallpaper placement uses the focused MRX monitor and X11 cursor coordinates; video wallpapers are explicitly disabled with a user-facing notice until an X11 video backend exists.
- Screen recording uses ffmpeg \`x11grab\` and \`slop\` for MRX/BSPWM, and retains \`wf-recorder\`/slurp for Wayland sessions.
- System cursor changes invoke \`hyprctl setcursor\` only for Hyprland sessions.
- Shell helper tests cover MRX, BSPWM, Niri and Hyprland detection, MRX monitor enumeration, focused monitor geometry and the no-hyprctl X11 path.

## Still outstanding

- Audit all imported QML services, shell commands and compositor-specific APIs.
- Add complete disabled/unsupported states for every control without an MRX implementation.
- [x] Implement monitor-targeted window moves through versioned MRX IPC, mrxctl and the Quickshell backend; exercise forward/reverse moves and unknown-monitor errors in Xvfb. (CI validation pending)
- Validate wallpaper selector, recording UI, settings pages, overview, lock integration and all interactive controls in a complete Quickshell session.
- Consolidate visual styling, typography, responsive layouts, focus order and accessibility.
- Test Quickshell reconnect, shell crash/restart, screen hotplug and long-running session behavior on CachyOS-LTS.

## Validation boundary

The shell helper tests use mocked mrxctl/X11 tools and are not a live Quickshell session test. The current changes do not modify SDDM, the existing Orbital theme, the existing Quickshell lock screen or the user's desktop session.
