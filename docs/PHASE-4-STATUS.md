# Phase 4 — Quickshell integration status

## Implemented and CI-covered

The initial X11 service/fallback audit is recorded in [SHELL_AUDIT.md](SHELL_AUDIT.md). It explicitly distinguishes implemented X11 paths from Hyprland-only features and external fallbacks.

Latest CI evidence: [PR #20 run](https://github.com/manivelx/MRX-Desktop/actions/runs/44914024803), commit `73cff5cdd7435f16c7781a106f84bbb2c4ee0a06`.

- MRXBackend obtains state from \`mrxctl\` and consumes the IPC snapshot stream with reconnect/fallback behavior.
- Wallpaper-selector search toggles Quickshell state directly; it no longer sends a Hyprland-only compositor IPC command.
- Shared \`mrx-session.sh\` identifies MRX as an X11 session and provides mrxctl monitor names, active-monitor selection, focused-monitor geometry and root display geometry.
- Static wallpaper placement uses the focused MRX monitor and X11 cursor coordinates; video wallpapers are explicitly disabled with a user-facing notice until an X11 video backend exists.
- Screen recording uses ffmpeg \`x11grab\` and \`slop\` for MRX/BSPWM, and retains \`wf-recorder\`/slurp for Wayland sessions.
- System cursor changes invoke \`hyprctl setcursor\` only for Hyprland sessions.
- Hide Hyprland-only anti-flashbang shader and brightness-adjustment controls outside Hyprland; disable MRX night-light controls with an explicit unsupported notice while retaining the supported X11 hardware-brightness control.
- Filter Hyprland-only anti-flashbang quick toggles from non-Hyprland Android panels and mark MRX night-light quick toggles unavailable with explanatory tooltips.
- Add an MRX settings capability summary for implemented WM controls and current unsupported features; add source-level regression coverage in both Ubuntu and Arch CI.
- Correct MRX Game Mode state detection and persist/restore the user's pre-toggle gap/border settings across Quickshell reloads instead of assuming fixed defaults.
- Shell helper tests cover MRX, BSPWM, Niri and Hyprland detection, MRX monitor enumeration, focused monitor geometry and the no-hyprctl X11 path.
- Add an Arch CI live-session smoke test that installs the project into a temporary home, starts Xvfb and a private DBus session, launches the Rust WM and Quickshell frontend, then verifies IPC health and frontend process survival. The same test is optional in the developer build when runtime dependencies are installed.
- Add `docs/SHELL_AUDIT.md` covering global shortcuts, focus grabs, lock-screen fallback, Picom, tray, idle inhibition, notifications, wallpaper, recording and the automated validation boundary.

## Still outstanding

- Complete the per-file/per-control audit of all imported QML services, shell commands and compositor-specific APIs; the new audit is a baseline, not a claim that every control has been tested.
- Add complete disabled/unsupported states for every control without an MRX implementation.
- [x] Implement monitor-targeted window moves through versioned MRX IPC, mrxctl and the Quickshell backend; exercise forward/reverse moves and unknown-monitor errors in Xvfb. PASS — [PR #22 CI run #591](https://github.com/manivelx/MRX-Desktop/actions/runs/44917525244) passed on the exact PR head in Ubuntu and Arch Linux.
- Validate wallpaper selector, recording UI, settings pages, overview, lock integration and all interactive controls in a complete Quickshell session.
- Add an X11 focus-dismissal implementation and a documented application keybinding layer.
- Consolidate visual styling, typography, responsive layouts, focus order and accessibility.
- Test Quickshell reconnect, shell crash/restart, screen hotplug and long-running session behavior on CachyOS-LTS.

## Validation boundary

The shell helper tests use mocked mrxctl/X11 tools and are not a live Quickshell session test. The current changes do not modify SDDM, the existing Orbital theme, the existing Quickshell lock screen or the user's desktop session.
