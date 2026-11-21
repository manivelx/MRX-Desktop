# MRX Desktop manual validation checklist

## Test safety

- [ ] Keep the current BSPWM/SDDM session and a working TTY recovery path.
- [ ] First test in nested Xephyr; use a disposable Arch/CachyOS-LTS VM for full-session tests.
- [ ] Do not overwrite the existing Orbital theme, SDDM configuration or `~/.local/share/quickshell-lockscreen`.
- [ ] Do not change the NVIDIA 470xx driver, kernel or bootloader as part of MRX testing.

## Record the environment

| Field | Value |
|---|---|
| Date/time | |
| Distro and release | |
| Kernel | |
| Session type (X11) | |
| GPU and driver | |
| Quickshell version | |
| Picom version/backend | |
| Monitor model, resolution and refresh | |
| MRX commit | |

## Build and installation

- [ ] `./check-deps.sh` reports all required dependencies.
- [ ] `./build.sh` completes without errors.
- [ ] `bash ./tests/xvfb-smoke.sh` passes.
- [ ] `bash ./tests/xephyr-smoke.sh` passes.
- [ ] `bash ./tests/quickshell-smoke.sh` passes when Quickshell/Xvfb dependencies are installed.
- [ ] Install into a disposable user/VM and verify the session entry.
- [ ] Reinstall/upgrade and verify backups are preserved.
- [ ] Uninstall and verify pre-existing binaries, session entries, Picom configuration and user settings are restored.

## Window manager and input

- [ ] Login and logout repeatedly without a black screen or orphaned processes.
- [ ] Open, focus, move, resize, minimize, restore, maximize and close normal windows.
- [ ] Test transient dialogs, modal dialogs, urgency and fullscreen.
- [ ] Test all nine workspaces and monitor-local workspace behavior.
- [ ] Test keyboard navigation, Escape handling and popup dismissal.
- [ ] Record missing global application shortcuts; do not assume Hyprland shortcuts work on X11.
- [ ] Test mouse focus and window dragging/resizing where implemented.

## Quickshell pages and services

- [ ] Bar, workspace indicator and focused-monitor display.
- [ ] Overview and window selection.
- [ ] Settings pages and each interactive control.
- [ ] Left/right sidebars and desktop menu.
- [ ] Wallpaper selector, static wallpaper application and monitor placement.
- [ ] System tray with StatusNotifier applications and legacy XEmbed applications.
- [ ] Notifications, notification actions and daemon coexistence.
- [ ] Audio, media, network, brightness and battery widgets.
- [ ] Clipboard, screenshot and screen-recording controls.
- [ ] Unsupported Wayland/Hyprland controls are visibly disabled or explain why they are unavailable.
- [ ] Test shell restart and verify MRX IPC reconnects.

## Lock, idle and session behavior

- [ ] Verify `xss-lock` starts the existing user lock launcher without modifying it.
- [ ] Lock and unlock repeatedly; verify no input bypass or stuck black screen.
- [ ] Test suspend/resume and lock-on-resume.
- [ ] Test idle inhibition and release of inhibition.
- [ ] Test DPMS/screen blanking and wake-up.
- [ ] Verify logout/shutdown does not leave Quickshell, Picom or WM processes behind.

## Graphics and display hardware

- [ ] Record `glxinfo -B` output and Picom diagnostics.
- [ ] Test Picom GLX backend with the GT 740 and NVIDIA 470xx driver.
- [ ] Test Picom XRender fallback if GLX is unstable.
- [ ] Test the session with Picom disabled; window management and shell interaction must remain usable.
- [ ] Check shadows, transparency, rounded corners and tearing.
- [ ] Test one monitor, two monitors, mixed resolutions and refresh rates.
- [ ] Hotplug/disconnect/reconnect monitors and verify workspace/window migration.
- [ ] Verify EDID, preferred mode, scaling and mixed-DPI behavior.
- [ ] Measure idle CPU, memory, GPU use and frame pacing during normal interaction.

## Result log

| Test area | Pass / Fail / Blocked | Evidence or issue |
|---|---|---|
| Build and install | | |
| WM and IPC | | |
| Quickshell rendering | | |
| Quickshell controls | | |
| Global shortcuts | | |
| Tray and notifications | | |
| Lock and idle | | |
| Picom GLX / XRender | | |
| No-compositor fallback | | |
| Multi-monitor and hotplug | | |
| Suspend/resume | | |
| Upgrade/rollback/uninstall | | |

Do not mark a hardware or release gate complete until the test was performed on the stated physical device or disposable VM and the result is recorded.
