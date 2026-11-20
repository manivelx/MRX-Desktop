# Phase 1 — Rust X11 window manager status

Release target: `0.1.0-alpha.12`  
Status: **IN PROGRESS — not a daily-driver release**

## Implemented in this revision

- Fixed an initialization regression where `_NET_CLIENT_LIST` and `_NET_CLIENT_LIST_STACKING` were published after startup adoption and then immediately cleared before entering the event loop.
- Kept ICCCM `WM_STATE=NormalState` for clients hidden on another virtual desktop; `_NET_WM_STATE_HIDDEN` carries virtual-desktop visibility.

- Fixed startup EWMH client-list initialization: `_NET_CLIENT_LIST` and `_NET_CLIENT_LIST_STACKING` are published after startup adoption and are no longer immediately overwritten with empty arrays before the event loop starts.
- Kept ICCCM `WM_STATE=NormalState` for clients hidden on another virtual desktop; `_NET_WM_STATE_HIDDEN` carries the desktop visibility state.

- IPC accepts concurrent local clients using a bounded 32-client limit; slow clients no longer block all other clients.
- IPC rejects oversized/non-newline-terminated requests, refuses to unlink non-socket paths, and uses mode 0600 for the socket. A private fallback runtime directory is mode 0700; an existing XDG_RUNTIME_DIR is not chmodded.

- Shared, deterministic master/stack, columns and monocle geometry calculations.
- RandR 1.5 monitor discovery with stable atom-derived names, geometry and primary-monitor selection; falls back to the X screen geometry when RandR monitor objects are unavailable.
- RandR screen/CRTC/output/resource change notifications refresh the monitor snapshot; the Quickshell adapter now maps windows to reported monitor rectangles instead of hardcoding `screen-0`.
- Windows retain a monitor identity in the shared snapshot, tiled clients are arranged independently inside each monitor's geometry, and windows assigned to a disconnected monitor migrate to the remaining focused/primary monitor. Floating windows are clamped into the fallback monitor bounds.
- ICCCM `WM_DELETE_WINDOW` is now requested through `WM_PROTOCOLS`; `KillClient` is reserved for clients that do not advertise polite close support.
- ICCCM focus negotiation now reads `WM_HINTS` InputHint and sends `WM_TAKE_FOCUS` only when the client advertises that protocol.
- Transient windows are identified through `WM_TRANSIENT_FOR` and excluded from tiling as floating windows.
- Tiled clients no longer have arbitrary ConfigureRequest geometry applied; floating clients can request geometry.
- EWMH root-message handling for `_NET_CLOSE_WINDOW`, `_NET_ACTIVE_WINDOW`, `_NET_CURRENT_DESKTOP`, and `_NET_WM_STATE_FULLSCREEN` add/remove/toggle requests.
- ICCCM `WM_STATE` remains Normal for managed clients hidden on another workspace; Withdrawn is set when a visible client unmaps itself. EWMH hidden state represents workspace visibility.
- EWMH `_NET_WM_STATE_HIDDEN` is updated alongside workspace visibility.
- Publish desktop names, viewport, workarea and desktop geometry metadata.
- Publish `_NET_CLIENT_LIST_STACKING` using the server root-child stacking order, and retain a separate MRX client map-order list for `_NET_CLIENT_LIST`.
- Adopt already-viewable non-override-redirect root children on WM startup, initialize ICCCM Normal state, subscribe to property changes, and initialize the active-window property.
- Handle `_NET_WM_DESKTOP` requests for regular workspaces and update `_NET_ACTIVE_WINDOW` on accepted focus changes.
- Initial `UnmapNotify` withdrawal handling, with hidden-workspace unmaps excluded.
- Fullscreen transitions preserve the current client geometry and restore it when fullscreen is removed.
- IPC requests now have a 64 KiB input cap and read/write timeouts.
- GitHub Actions CI added for formatting, tests, Clippy, build and Xvfb smoke checks.

## Still blocking Phase 1 completion

- Complete `WM_STATE` lifecycle (Withdrawn/Normal/Iconic), urgency handling, normal size hints, startup adoption and robust unmap handling. `WM_HINTS` InputHint and `WM_TAKE_FOCUS` negotiation are implemented but still require integration tests. The current unmap handling is an initial implementation and needs integration testing for synthetic and reparenting edge cases.
- Remaining `_NET_WM_STATE` semantics: initial client state adoption, maximize/restore, above/below, and full hidden/minimized behavior. Fullscreen support is implemented as an initial subset.
- Startup adoption can only approximate original map order from current stacking; full historical `_NET_CLIENT_LIST` ordering needs persistent tracking during a running WM. Client-list stacking is now published from the X server tree, but must be integration-tested. Workarea still assumes the full screen and does not yet subtract dock/panel struts.
- Full ICCCM focus policy (`WM_HINTS` input model and `WM_TAKE_FOCUS`), focus restoration, urgency, stacking requests and keyboard grab error handling.
- Dock/panel struts and work-area calculation.
- Per-monitor workspace ownership and independent per-monitor workspace switching remain unimplemented; workspaces are still global across the X screen. Monitor hotplug/migration code is present but still needs a multi-output integration test, including an actual disconnect/reconnect.
- IPC now has a 64 KiB line cap, read/write timeouts, bounded concurrent clients and safer socket-path handling; multiple requests per connection, event subscriptions and orderly shutdown remain open.
- Compilation, formatting, Clippy and runtime integration must pass on CI; the Xvfb smoke test is being used to validate window mapping and ICCCM close before Phase 1 can be considered complete.
- Xephyr integration scenarios for normal map/unmap/destroy, transient dialog, focus, polite close, fullscreen restore, EWMH and RandR hotplug.

## Current validation boundary

The local container has Xvfb but does not have `cargo`, `rustc`, Quickshell or an NVIDIA device/driver. The Xvfb probe validates the virtual X server only. It does not validate the Rust WM, QML runtime, proprietary NVIDIA 470 driver, GT 740, or animation performance. The expanded WM smoke test is present but remains unexecuted until a Rust build is available.

Do not mark Phase 1 complete until the CI build and integration matrix pass and the missing protocol items above are implemented.
