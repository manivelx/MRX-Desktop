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
- [x] Initial EWMH dock strut/workarea handling.
- [x] Automated Xvfb virtual-monitor, workspace, dock, urgency and lifecycle coverage.
- [ ] Complete EWMH maximize/restore, above/below and minimized/hidden state lifecycle.
- [ ] Validate complex ICCCM hints/dialog/focus edge cases in Xephyr/Xorg.
- [ ] Validate multiple/overlapping/vertical docks and mixed-resolution layouts.
- [ ] Validate physical monitor hotplug, EDID, reconnect and migration behavior.

## Phase 2 — Desktop core and IPC

- [x] Version IPC messages and correlate request/response IDs.
- [x] Bound IPC framing, timeouts and concurrent service clients.
- [x] Add push-based event subscriptions and subscriber disconnect cleanup.
- [ ] Standardize structured asynchronous operation results and error categories.
- [ ] Finish typed settings validation, atomic apply/rollback and profile import/export.
- [ ] Add process health reporting, supervised restart and orderly shutdown tests.
- [ ] Validate event ordering, backpressure and high-rate update behavior.

## Phase 3 — Graphics and animations

- [ ] Prototype XComposite capture and XDamage tracking.
- [ ] Build/validate renderer with a no-animation fallback.
- [ ] Add workspace transitions with correct input/focus handoff.
- [ ] Verify GLX/Present behavior on supported hardware.
- [ ] Measure frame pacing, CPU/GPU use, memory and recovery from renderer failure.

## Phase 4 — Quickshell integration and visual system

- [ ] Audit every imported QML service, shell command and compositor-specific API.
- [ ] Route supported window/workspace/settings actions through MRX APIs.
- [ ] Add honest disabled/unsupported states for unavailable features.
- [ ] Consolidate theme, typography, spacing, responsive layouts and accessibility.
- [ ] Validate each QML page and interactive control in a complete session.
- [ ] Replace snapshot polling with subscription-driven updates where supported.

## Phase 5 — Session, installer and release engineering

- [x] Add reversible user-scoped install/uninstall scaffolding with isolated regression tests.
- [ ] Test installation, upgrade, rollback and removal in disposable Arch/CachyOS VMs.
- [ ] Validate coexistence with existing BSPWM/SDDM/lock-screen configuration.
- [ ] Add recovery and diagnostics for failed session startup.
- [ ] Publish reproducible release artifacts, checksums and release notes.
- [ ] Complete hardware compatibility matrix before making support claims.

## Release gates

MRX remains experimental until all of the following are true:

1. CI is green on the exact release commit.
2. The WM passes nested-X and real-client integration tests.
3. The Quickshell session is validated end-to-end.
4. Installer/upgrade/rollback are tested in disposable VMs.
5. Hardware claims have recorded results for each supported GPU/output path.

A virtual Xvfb monitor test is not a substitute for physical multi-monitor validation. A successful Rust build is not evidence that the full desktop session is stable.
