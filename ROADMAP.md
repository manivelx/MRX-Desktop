# MRX Desktop roadmap

Items are marked complete only when implementation and relevant automated validation exist. Physical hardware and end-to-end session checks are recorded separately.

## Phase 0 — Repository foundation

- [x] Publish source tree, license notices and contributor documentation.
- [x] Define project scope and record known gaps.
- [x] Add CI for formatting, tests, lint, build, static checks, installer sandbox and Xvfb.

## Phase 1 — Reliable X11 window manager

- [x] Rust X11 connection, event loop and local IPC.
- [x] ICCCM close/focus basics, startup adoption, urgency and common size hints.
- [x] EWMH desktop metadata and core client messages.
- [x] RandR 1.5 monitor discovery, event updates and reconciliation.
- [x] Per-monitor tiling, independent workspaces and monitor-aware shell workspace routing.
- [x] Move managed windows between named monitors and monitor-local workspaces through MRX IPC, with Xvfb acceptance coverage.
- [x] Initial EWMH dock strut/workarea handling.
- [x] Automated Xvfb virtual-monitor, workspace, dock, urgency and lifecycle coverage.
- [x] Run the complete compiled-WM integration suite inside nested Xephyr; prefer explicit RandR monitor objects over overlapping automatic fallbacks.
- [x] Validate simultaneous top/vertical docks, same-edge maximum reservations, dock-removal workarea restoration and mixed-resolution monitor strut intersection in Xvfb/Xephyr and unit tests.
- [x] Implement horizontal/vertical EWMH maximize and restore with original-geometry preservation and Xvfb coverage.
- [x] Implement EWMH hidden/minimize and ICCCM WM_CHANGE_STATE with managed-client retention and Xvfb coverage.
- [x] Implement mutually exclusive EWMH ABOVE/BELOW state requests with Xvfb snapshot coverage.
- [x] Adopt initial EWMH fullscreen/maximize/hidden/ABOVE/BELOW state, ICCCM IconicState and per-window desktop assignment at WM startup; validate pre-existing clients in Xvfb.
- [x] Reconcile managed-client stacking by BELOW/normal/ABOVE/dock/fullscreen layer, including state removal and map/unmap/destroy/configure transitions.
- [x] Add ICCCM transient-parent tracking, EWMH modal state adoption and parent-focus redirection for nested modal dialogs; validate startup and IPC focus behavior in Xvfb.
- [ ] Complete full state restoration semantics and broader dialog focus/withdrawal edge cases.
- [ ] Validate complex ICCCM hints/dialog/focus edge cases in Xephyr/Xorg.
- [ ] Validate remaining overlapping/multi-panel dock layouts, mixed-DPI behavior and hotplug/disconnect/reconnect on physical Xorg hardware.
- [ ] Validate physical monitor hotplug, EDID, reconnect and migration behavior.

## Phase 2 — Desktop core and IPC

- [x] Version IPC messages (v3) and correlate request/response IDs.
- [x] Bound IPC framing, timeouts and concurrent service clients.
- [x] Implement persistent push-based snapshot subscriptions with change-only events and bounded subscriber disconnect cleanup.
- [x] Standardize structured asynchronous operation results, stable operation IDs, bounded result retention, polling and categorized failures.
- [x] Validate typed settings on a candidate copy and persist atomically before replacing live settings.
- [x] Add bounded settings undo history, atomic reset, and validated atomic profile import/export with Xvfb acceptance coverage.
- [x] Add IPC process health reporting and orderly shutdown with root-property/socket cleanup and Xvfb acceptance tests.
- [x] Add a session WM supervisor with health-gated startup, capped exponential crash-restart backoff, clean-shutdown detection and Xvfb crash-recovery coverage.
- [x] Add structured asynchronous operation results.
- [x] Add monotonic snapshot event sequence numbers, subscriber lifecycle/traffic metrics, slow-client timeout/drop accounting and high-rate Xvfb subscription tests.

## Phase 3 — Graphics and animations

- [x] Prototype isolated XComposite named-pixmap capture and XDamage event tracking on Xvfb/Xephyr; production WM redirection remains disabled until a renderer owns the compositor role.
- [x] Validate off-screen XRender composition of captured pixmaps on Xvfb/Xephyr; managed-client/root overlay rendering remains a separate production milestone.
- [ ] Build/validate renderer with a no-animation fallback.
- [ ] Add workspace transitions with correct input/focus handoff.
- [ ] Verify GLX/Present behavior on supported hardware.
- [ ] Measure frame pacing, CPU/GPU use, memory and recovery from renderer failure.

## Phase 4 — Quickshell integration and visual system

- [ ] Audit every imported QML service, shell command and compositor-specific API.
- [ ] Route all supported window/workspace/settings actions through MRX APIs.
- [x] Add explicit MRX X11 paths for static wallpaper application, monitor-aware wallpaper placement and ffmpeg/x11grab screen recording.
- [x] Route wallpaper-selector search through Quickshell state instead of Hyprland-only compositor IPC.
- [x] Add explicit disabled/unsupported states for MRX night-light and Hyprland-only shader quick toggles; continue the full imported-QML control audit.
- [ ] Consolidate theme, typography, spacing, responsive layouts and accessibility.
- [ ] Validate every QML page and interactive control in a complete session (Arch CI now has a Quickshell/Xvfb startup + IPC smoke gate; full visual/control coverage remains).
- [x] Replace normal Quickshell snapshot polling with the IPC event stream and retain reconnect/fallback snapshots.

## Phase 5 — Session, installer and release engineering

- [x] Add reversible user-scoped install/uninstall scaffolding with isolated regression tests.
- [x] Add Ubuntu and native Arch Linux container CI for formatting, tests, Clippy, debug/release builds, installer sandbox and Xvfb/Xephyr integration coverage.
- [ ] Test installation, upgrade, rollback and removal in disposable Arch/CachyOS VMs.
- [ ] Validate coexistence with existing BSPWM/SDDM/lock-screen configuration.
- [ ] Add recovery and diagnostics for failed session startup.
- [x] Add reproducible Linux release archives, SHA-256 checksums and tag-triggered GitHub Release automation (first version tag still pending).
- [x] Add privacy-conscious startup diagnostics and automatic report capture when the MRX WM fails its startup health check.
- [ ] Complete hardware compatibility matrix before making support claims.

## Release gates

MRX remains experimental until all of the following are true:

1. CI is green on the exact release commit.
2. The WM passes nested-X and real-client integration tests.
3. The Quickshell session is validated end-to-end.
4. Installer/upgrade/rollback are tested in disposable VMs.
5. Hardware claims have recorded results for each supported GPU/output path.

A virtual Xvfb monitor test is not a substitute for physical multi-monitor validation. A successful Rust build is not evidence that the full desktop session is stable.
