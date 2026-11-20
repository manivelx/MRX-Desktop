# Phase 1 — Rust X11 window manager status

Release target: `0.1.0-alpha.12`  
Status: **IN PROGRESS — experimental, not a daily-driver release**

## Implemented

- Rust X11 WM ownership/event loop, shared state snapshot and typed local IPC.
- ICCCM polite close, WM_STATE updates, WM_TAKE_FOCUS negotiation, transient detection, live WM_HINTS urgency, WM_NORMAL_HINTS min/max/base/increment/aspect constraints, fixed-size floating policy and focus restoration after unmap/destruction.
- Startup adoption of viewable non-override-redirect clients; floating and tiled ConfigureRequest policy.
- EWMH desktop metadata, client list/stacking, close/active/current desktop/window desktop/fullscreen requests and initial dock strut/workarea handling.
- Master/stack, columns and monocle layout geometry; per-monitor workareas for tiled layout.
- RandR 1.5 monitor discovery, atom names, primary-monitor selection, active-CRTC fallback, notification refresh and 250ms reconciliation. Synthetic root-screen rectangles are discarded when actual output monitors are available.
- Independent monitor-local workspaces, visibility, per-monitor snapshots and monitor-targeted IPC. Quickshell workspace bars route to the matching monitor.
- Window monitor identity, per-monitor tiling, client migration after monitor removal and floating-window clamping.
- IPC protocol v3 with request IDs/correlated responses, event subscription path, bounded socket framing/timeouts and connection cleanup; settings persistence and atomic writes.
- Isolated installer/reinstaller/uninstaller sandbox coverage. CI exercises formatting, tests, Clippy, build, static guards, installer sandbox and Xvfb server/WM smoke tests.

## Latest CI validation

The latest recorded successful baseline is [run #327](https://github.com/manivelx/MRX-Desktop/actions/runs/44761884015) on commit `422615cab39c298505d49e996ce80d927e165551`. It predates the current IPC v3 protocol bump and latest README/docs edits. The current branch's CI must pass before the current tip is described as validated.

## Remaining Phase 1 acceptance work

- Complete EWMH state semantics: initial state adoption, maximize/restore, above/below and full minimized/hidden behavior.
- Improve ICCCM corner-case coverage: very small workareas, aspect/increment interactions, focus/withdrawal and transient/dialog behavior.
- Validate overlapping/multiple/vertical struts, mixed-DPI/mixed-resolution geometry and physical hotplug/disconnect/reconnect on actual multi-output hardware.
- Validate Xephyr/nested Xorg session behavior and complete QML runtime interactions.

## Phase 1 exit boundary

Phase 1 is **not complete** while those protocol and runtime criteria remain. A successful CI/Xvfb run does not prove real monitor hardware behavior, Quickshell session stability or NVIDIA proprietary driver support.

## Hardware and runtime validation not performed

- Physical multi-monitor hotplug/EDID behavior.
- NVIDIA 470 / GeForce GT 740 proprietary driver, GLX acceleration and animation frame pacing.
- Full Quickshell/QML desktop session on CachyOS/Arch.
- Power-management, suspend/resume and long-session stability.

Do not mark Phase 1 complete or recommend daily use until these acceptance tests have been performed.
