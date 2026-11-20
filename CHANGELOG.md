# Changelog

## 0.1.0-alpha.11

- Publish `_NET_CLIENT_LIST_STACKING` using the X server's bottom-to-top root-child order, while retaining a separate MRX client map-order list for `_NET_CLIENT_LIST`.
- Track initial client mapping order and refresh both EWMH client lists on map, unmap, and destroy.
- Adopt already-viewable non-override-redirect top-level windows at WM startup, set their ICCCM `WM_STATE`, attach property notifications, and publish the initial active client.
- Extend static and Xvfb smoke-test assertions for the client-stacking property.
- This is not full RandR multi-monitor support; the current layout engine still arranges against one monitor rectangle.


## 0.1.0-alpha.9 — IPC isolation and runtime-path hardening

- Handle local IPC clients concurrently with a fixed maximum of 32 active client workers.
- Reject oversized and incomplete IPC frames; return an error for empty requests.
- Refuse to remove an existing IPC path unless it is a Unix socket.
- Keep the fallback runtime directory private without changing permissions on an existing XDG_RUNTIME_DIR.
- Align Cargo workspace version metadata with the alpha.9 source archive.
- Expand static checks for IPC concurrency and socket-path safety.
- Rust compilation and Quickshell runtime validation remain pending because the build tools are unavailable in this environment.

## 0.1.0-alpha.8 — EWMH metadata and client lifecycle increment

- Publish `_NET_DESKTOP_NAMES`, `_NET_WORKAREA`, `_NET_DESKTOP_VIEWPORT`, and `_NET_DESKTOP_GEOMETRY` from the configured workspace count and active X screen dimensions.
- Advertise implemented desktop metadata and hidden/fullscreen state atoms in `_NET_SUPPORTED`.
- Handle `_NET_WM_DESKTOP` requests for regular workspaces and synchronize visibility, ICCCM normal state, and EWMH hidden state.
- Track root `_NET_ACTIVE_WINDOW` on managed-window map and accepted focus/activation requests; clear it when changing workspaces.
- Add initial `UnmapNotify` withdrawal handling while excluding clients hidden on inactive workspaces.
- Extend static checks and the Xvfb smoke test for EWMH metadata and polite close behavior.
- Correct the MRX QML adapter's hidden-window flag and resolve workspace names from the snapshot.
- Rust compilation and the WM smoke test remain unverified in the authoring container because Cargo/rustc are unavailable.


All notable changes to MRX Desktop are documented here.

## 0.1.0-alpha.3 — X11 test environment and dependency hardening

- Added an Arch/CachyOS dependency installer for Rust, Quickshell/Qt, X11, Xvfb, Xephyr and end4-pC runtime helpers.
- Added isolated Xvfb server and MRX WM smoke-test scripts.
- Added a read-only hardware/driver report and NVIDIA Kepler/X11 compatibility notes.
- Documented the local Unix-socket IPC path; no TCP ports are required.
- Retired the old BSPWM session launch files and converted shell-level installer commands into wrappers for the MRX root scripts.
- Added a validation record distinguishing virtual-X11 checks from Rust, Quickshell and real NVIDIA hardware tests.

## Unreleased — next engineering milestone

- Initial public repository packaging.
- Rust workspace with core, X11 window-manager and CLI crates.
- Initial X11 workspace/window-management foundation and local IPC.
- Initial TOML settings persistence.
- Imported end4-pC X11 Quickshell/QML source and an early MRX adapter.
- Experimental isolated session files and developer scripts.
- Added contributor, security, testing, architecture and licensing documentation.

### Known gaps

- No verified complete build or runtime session.
- No dedicated workspace-slide renderer.
- Incomplete ICCCM/EWMH behavior, floating/transient/fullscreen handling and monitor support.
- QML integration still contains upstream compositor-specific assumptions and requires a systematic audit.
- Installer is not ready for daily-use systems.

## 0.1.0-alpha.2 — foundation hardening

- Added an explicit IPC snapshot protocol version.
- Made settings files tolerant of omitted fields so older user settings can inherit safe defaults.
- Added central settings validation for workspace count, animation duration, opacity, layout, master ratio and accent color.
- Changed settings persistence to validate first and write through a same-directory temporary file before atomic rename.
- Made the Rust settings path honor `MRX_CONFIG_HOME`, keeping the session's settings isolated.
- Changed setting updates to validate and persist a candidate copy before replacing live state.
- Rejected invalid workspace move targets rather than allowing an invalid desktop index.
- Added the missing static validation script referenced by the build workflow.

This is still an experimental foundation update. It is not a stable release and has not been compiled or exercised against a live X server in this environment.


### Validation for alpha.3

- Shell syntax checks: passed.
- TOML parsing: passed.
- Repository static checks: passed.
- Xvfb 1920x1080 X11 client test: passed.
- Picom XRender and GLX backends launched in Xvfb; GLX vsync was unavailable in the virtual server.
- Rust compilation, Quickshell runtime and NVIDIA GT 740 testing: not run in this environment.


## 0.1.0-alpha.4 — Phase 1 WM foundation increment

- Added a pure Rust layout geometry module for master/stack, columns and monocle layouts.
- Added unit tests for layout edge cases and non-zero geometry guarantees.
- Connected the WM arrange operation to the shared geometry module.
- Added an explicit Phase 1 acceptance report documenting protocol and runtime blockers.
- This release is not a completed or production-ready window manager.

## MRX alpha.5 identity pass

- Renamed project-owned identifiers, crates, executables, session files, IPC/config paths and documentation to MRX.
- Added a regression check preventing the former product identifier from returning.
- Phase 0 source-level identity/static audit recorded in `docs/PHASE-0-STATUS.md`.
- Phase 1 remains incomplete; see `docs/PHASE-1-STATUS.md` for protocol blockers.


## 0.1.0-alpha.6 — ICCCM/EWMH protocol increment

- Added ICCCM `WM_DELETE_WINDOW` polite-close requests with force-kill fallback only for clients that do not advertise the protocol.
- Identified `WM_TRANSIENT_FOR` windows and excluded them from tiled layout.
- Constrained ConfigureRequest handling for tiled windows while permitting floating-window geometry requests.
- Added initial EWMH root ClientMessage handling for `_NET_CLOSE_WINDOW`, `_NET_ACTIVE_WINDOW`, and `_NET_CURRENT_DESKTOP`.
- Added GitHub Actions CI for Rust formatting, tests, Clippy, build, static checks and Xvfb probes.
- Updated Phase 0/1 status documents: Phase 1 remains incomplete and is not a daily-driver release.
- Rust compilation and Quickshell/NVIDIA runtime validation remain outstanding because the build toolchain and target hardware are unavailable in this environment.

## 0.1.0-alpha.8 — EWMH metadata and client lifecycle increment

- Publish `_NET_DESKTOP_NAMES`, `_NET_WORKAREA`, `_NET_DESKTOP_VIEWPORT`, and `_NET_DESKTOP_GEOMETRY` from the configured workspace count and active X screen dimensions.
- Advertise the implemented `_NET_WM_STATE_HIDDEN` and desktop metadata atoms in `_NET_SUPPORTED`.
- Handle client `_NET_WM_DESKTOP` requests for non-sticky desktops and synchronize workspace visibility, `WM_STATE`, and hidden state.
- Track root `_NET_ACTIVE_WINDOW` on managed-window map and accepted activation/focus requests.
- Add initial `UnmapNotify` handling for client withdrawal while avoiding removal of clients hidden on inactive workspaces.
- Extend the Xvfb smoke test to inspect EWMH root metadata and exercise ICCCM close requests against xterm.

Validation boundary: shell/TOML/static checks and the standalone Xvfb server probe can run in the current build container. The Rust WM smoke test requires Cargo-built binaries and has not been run here.


## 0.1.0-alpha.10

- Added ICCCM focus negotiation using `WM_HINTS` InputHint and `WM_TAKE_FOCUS`.
- Preserved the existing IPC focus command and EWMH active-window request while routing focus through the ICCCM helper.
- Updated Phase 1 implementation and validation status.
- This revision is source-reviewed only; Rust build and Xvfb WM integration remain unverified until CI executes.


## 0.1.0-alpha.12 — 2026-11-20

- Fixed startup ordering so EWMH client-list properties populated during startup adoption are not immediately erased before the WM event loop begins.
- Kept ICCCM `WM_STATE` at `NormalState` when a client is moved to an inactive virtual desktop; EWMH hidden state represents desktop visibility.
- Added regression checks for startup property preservation and desktop-hidden ICCCM state.


## 0.1.0-alpha.12 — 2026-11-20

- Fixed startup ordering so EWMH client-list properties populated during startup adoption are not immediately erased before the WM event loop begins.
- Kept ICCCM `WM_STATE` at `NormalState` when a client is moved to an inactive virtual desktop; EWMH hidden state represents desktop visibility.
- Prevented the imported Hyprland-specific autostart helper from running in an MRX session.
- Added regression checks for startup property preservation, desktop-hidden ICCCM state, and MRX autostart isolation.
