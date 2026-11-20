# Architecture and implementation contracts

## Process model

`mrx-wm` owns the X11 `SubstructureRedirect` selection and is the only window-manager process. It publishes root EWMH metadata and manages client/workspace state. `mrxctl` is a stateless Unix-domain socket client. Quickshell is a presentation process and must not own window-management policy.

## IPC protocol (v3)

- Socket: `$XDG_RUNTIME_DIR/mrx.sock` (0600). When `XDG_RUNTIME_DIR` is unset, MRX uses a private per-user fallback runtime directory.
- Transport: newline-delimited JSON over a local Unix socket; request/reply commands use one connection, while `subscribe` keeps a connection open for snapshot events. No TCP listener.
- Each request carries `protocol_version` and a client-side `request_id`. The server rejects incompatible versions and echoes the ID; `mrxctl` verifies both before accepting a response or event.
- The server bounds request framing, socket read/write timeouts and concurrent client workers. It refuses to replace a non-socket at the IPC path.
- `Request::Subscribe` streams an initial snapshot and change-only snapshots. `mrxctl subscribe` validates the event envelope and emits newline-delimited snapshots for Quickshell's `SplitParser`.
- The WM currently checks for snapshot changes every 100 ms rather than publishing from an internal state-change bus. Disconnected clients are detected and release their worker slot; Quickshell reconnects and uses a slow one-shot snapshot fallback while the stream is unavailable.
- Structured asynchronous mutation results, subscriber backpressure metrics and high-rate event-ordering tests remain outstanding.

## Settings

Runtime settings are owned by the Rust window manager. The `set` request parses a typed JSON value for a supported key, applies it to a candidate settings copy, validates the candidate, atomically writes TOML and only then swaps it into runtime state. Invalid types, unknown keys or out-of-range values are rejected without replacing the current configuration. Full multi-setting transactions, rollback UI and profile import/export are not implemented.

## X11 policy

The WM uses `x11rb` and explicit protocol extension features. It handles common ICCCM/EWMH behaviors incrementally and is tested against Xvfb and simple X11 clients. Clients are not all ordinary toplevels: startup adoption skips override-redirect windows; transient/fixed-size clients float; dock clients are sticky, excluded from ordinary workspace tiling and may reserve monitor workarea through partial/legacy struts.

Implemented protocol areas include polite `WM_DELETE_WINDOW`, `WM_TAKE_FOCUS` negotiation, `WM_STATE`, `WM_HINTS` urgency, common `WM_NORMAL_HINTS`, desktop metadata, client lists/stacking, current desktop, per-window desktop, close/active requests and basic fullscreen state. Maximized/restored geometry, ABOVE/BELOW, full initial-state adoption, hidden/minimize lifecycle and advanced dialog modality still need work.

## Monitor model

RandR 1.5 monitor objects are preferred, with active-CRTC fallback. The WM refreshes on RandR notifications and reconciles periodically for servers that do not report monitor-object changes reliably. Workspaces are local to a monitor; identical workspace numbers on different outputs are distinct state instances. The EWMH root `_NET_WORKAREA` remains a root-screen compatibility view; monitor-specific workareas are used internally for tiling.

Xvfb virtual-monitor coverage does not replace physical EDID, hotplug, mixed-resolution or mixed-DPI tests.

## Graphics and animations

Workspace animation is a future renderer project. The renderer must capture/retain window content with XComposite and track damage with XDamage before presenting frames through a validated GLX/Present path. No animation renderer is shipped as a validated production feature. GT 740/NVIDIA 470 testing is mandatory before enabling one by default.

## Session safety

MRX owns only its own binaries, session entry and MRX-specific configuration/shell directories. Its installer is experimental and user-scoped. It must not broadly kill processes, change SDDM defaults, replace the existing lock screen, or mutate the user's BSPWM/end4 sessions.
