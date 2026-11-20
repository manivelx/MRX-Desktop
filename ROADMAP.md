# Roadmap

This roadmap is directional. Items are not complete until code, tests and validation evidence are merged.

## Phase 0 — Public foundation

- [x] Publish source tree and upstream license files.
- [x] Document current limitations and validation gaps.
- [ ] Establish maintainer/release process and repository settings.
- [ ] Add CI that passes on a clean supported runner.

## Phase 1 — Reliable X11 window manager

- [ ] Complete ICCCM/EWMH essentials and startup client discovery.
- [ ] Correctly handle dialogs, transients, docks, override-redirect windows and urgency.
- [x] Implement common ICCCM size hints, fixed-size floating detection, fullscreen and focus policies.
- [ ] Validate edge-case size-hint/aspect combinations and complete remaining ICCCM lifecycle semantics.
- [x] Add RandR monitor discovery/change handling and per-monitor tiling foundation.
- [ ] Add dock-aware multi-monitor work areas and independent per-monitor workspaces.
- [ ] Add deterministic Xvfb/Xephyr integration tests.

## Phase 2 — Desktop core and live settings

- [ ] Version the IPC protocol and add request IDs and structured errors.
- [ ] Replace polling with event subscriptions.
- [ ] Make settings typed, validated and transactionally applied.
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
