# Roadmap

This roadmap is directional. Items are not complete until code, tests and validation evidence are merged.

## Phase 0 — Public foundation

- [x] Publish source tree and upstream license files.
- [x] Document current limitations and validation gaps.
- [ ] Establish maintainer/release process and repository settings.
- [x] Add CI that passes on a clean supported runner.

## Phase 1 — Reliable X11 window manager

- [ ] Complete ICCCM/EWMH essentials and startup client discovery.
- [ ] Correctly handle dialogs, transients, docks, override-redirect windows and urgency.
- [x] Implement common ICCCM size hints, fixed-size floating detection, fullscreen and focus policies.
- [ ] Validate edge-case size-hint/aspect combinations and complete remaining ICCCM lifecycle semantics.
- [x] Add RandR monitor discovery/change handling and per-monitor tiling foundation.
- [x] Add initial EWMH dock strut parsing and per-monitor usable workareas.
- [ ] Validate multiple/overlapping docks, vertical panels and physical mixed-resolution outputs.
- [x] Implement independent monitor-local workspaces, per-monitor snapshots and workspace-bar routing.
- [ ] Validate independent workspaces on physical multi-output hardware.
- [x] Add deterministic Xvfb integration tests for the compiled WM.\n- [ ] Add Xephyr nested-server coverage.

## Phase 2 — Desktop core and live settings

- [x] Version the IPC protocol, add request IDs, correlate responses and reject incompatible clients.
- [x] Add persistent push-based snapshot subscriptions with change-only events.\n- [ ] Add structured asynchronous operation results for queued mutations.
- [x] Replace normal Quickshell snapshot polling with the IPC event stream; retain a slow reconnect/fallback snapshot path.
- [x] Make settings typed, validated and transactionally applied (candidate copy, validation and atomic persistence before runtime replacement).
- [ ] Add undo/reset/profile import-export where appropriate.
- [ ] Add service health reporting and crash recovery.

## Phase 3 — Graphics and animations

- [ ] Prototype XComposite window capture and damage tracking.
- [ ] Validate GLX/Present rendering on supported hardware.
- [ ] Build workspace transitions with correct input/focus handoff.
- [ ] Measure frame pacing, CPU/GPU usage and memory.
- [ ] Keep a graceful no-animation fallback.

## Phase 4 — Shell integration and visual system

- [ ] Audit every QML service and compositor-specific command.
- [ ] Replace supported Hyprland/BSPWM calls with the MRX API.
- [ ] Provide explicit disabled states for unsupported features.
- [ ] Unify layout, spacing, typography, controls and responsive behavior.
- [ ] Test all settings from the UI without shell restarts.

## Phase 5 — Session and release engineering

- [ ] Make install/uninstall idempotent and ownership-scoped.
- [ ] Test in a disposable VM and nested X server.
- [ ] Verify coexistence with existing sessions and display managers.
- [ ] Publish signed/checksummed artifacts and release notes.
- [ ] Validate hardware matrix before making compatibility claims.
