# Phase 1 — Rust X11 window manager status

Release target: `0.1.0-alpha.12`  
Status: **IN PROGRESS — experimental, not a daily-driver release**

## Implemented

- Rust X11 WM ownership and event loop with a shared state snapshot and typed local IPC.
- Bounded IPC input framing, socket read/write timeouts, bounded concurrent clients, private fallback runtime directory, safe socket-path handling, and atomic settings persistence.
- `mrxctl` uses bounded response reads and socket timeouts.
- Workspace switching and EWMH desktop metadata, including `_NET_CLIENT_LIST`, `_NET_CLIENT_LIST_STACKING`, desktop names, viewport, workarea and geometry.
- ICCCM polite `WM_DELETE_WINDOW` close, `WM_STATE` updates, `WM_TAKE_FOCUS` negotiation, transient-window detection, and focus restoration after client unmap/destruction.
- Startup adoption of already-viewable non-override-redirect clients.
- EWMH handling for close, active-window, current-desktop, per-window desktop and fullscreen state requests.
- Floating and tiled client ConfigureRequest policy; master/stack, columns and monocle layout calculations.
- RandR 1.5 monitor discovery with atom-derived monitor names, geometry and primary-monitor selection, plus fallback to root-screen geometry.
- RandR screen/CRTC/output/resource notifications refresh monitor state.
- Windows retain monitor identity in the shared snapshot; tiled clients are arranged per monitor. On monitor removal, affected clients migrate to the remaining focused/primary monitor; floating windows are clamped into its bounds.
- Quickshell MRX adapter consumes the reported monitor list and window monitor identity instead of hardcoding `screen-0`.
- GitHub Actions validates formatting, workspace tests, Clippy, build, static checks, Xvfb server startup and the compiled WM smoke test.

## Latest CI validation

GitHub Actions [run #105](https://github.com/manivelx/MRX-Desktop/actions/runs/44737503616) passed on commit `5190f3bacd66b51e9aa4c88daa66b8ae8d1ccdc6`.

The compiled WM Xvfb smoke test passed IPC startup, EWMH metadata, workspace switching, setting persistence, RandR snapshot geometry, real X11 client mapping, monitor assignment and ICCCM close.

## Remaining work before Phase 1 can be considered complete

- Implement and test full `WM_NORMAL_HINTS` size-hint handling, urgency (`WM_HINTS` urgency), and remaining ICCCM lifecycle edge cases.
- Finish EWMH state semantics beyond the current subset: initial client state adoption, maximize/restore, above/below and complete minimized/hidden behavior.
- Add dock/panel strut parsing and compute usable workareas rather than using full monitor rectangles.
- Per-monitor workspace ownership and independent per-monitor workspace switching are not implemented; workspaces remain global across the X screen.
- Physical multi-output hotplug, disconnect/reconnect and floating-window migration need an integration test on a multi-output X server or real hardware. Xvfb validates only its virtual single-screen configuration.
- Validate transient dialogs, startup adoption, unmap/destroy races, focus policy, fullscreen restore and EWMH client stacking in more comprehensive Xephyr/Xorg scenarios.
- IPC event subscriptions and orderly shutdown remain open; the shell currently uses periodic snapshot refresh.

## Validation boundary

Rust formatting, tests, Clippy, build, static checks and the Xvfb WM smoke test pass in GitHub Actions. This does not validate Quickshell/QML runtime behavior, a complete MRX session, physical multi-monitor hotplug, the proprietary NVIDIA 470 driver, GT 740 compatibility, GLX acceleration or animation performance.

Do not mark Phase 1 complete or recommend daily use until the remaining protocol work and runtime/hardware validation have been completed.