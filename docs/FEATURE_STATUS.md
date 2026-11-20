# Feature status

This document separates design intent from implemented behavior. “Partial” means the code exists but is not complete or sufficiently tested for general use.

| Feature | Status | Notes |
|---|---|---|
| X11 connection | Partial | Initial connection and root-window ownership code exists. |
| Window mapping/focus | Partial | Startup adoption, unmap/destroy cleanup, focus restoration, transient handling and urgency snapshots exist; input focus models and edge-case runtime coverage remain. |
| EWMH desktop metadata | Partial | Desktop names, viewport, workarea, geometry, client lists and stacking are published; dock struts have initial parsing and workarea support; full state semantics remain. |
| Workspaces | Partial | Independent monitor-local workspace state, monitor-specific IPC and Quickshell workspace routing are implemented; slide animation is not implemented. |
| Master/stack layout | Partial | Shared geometry engine with per-monitor tiling and ICCCM min/max/base/increment/aspect constraints; small-workarea and real-client behavior still need broader validation. |
| Monocle layout | Initial | Shared geometry engine added; runtime testing pending. |
| Floating windows | Partial | Transient/fixed-size clients float; complete dialog modality, stacking and edge-case behavior need validation. |
| Fullscreen | Partial | Basic EWMH fullscreen state is implemented; maximize/restore, initial state adoption and complete restoration semantics remain. |
| Multi-monitor | Partial | RandR discovery/reconciliation, per-monitor tiling and workspaces, monitor-specific IPC, Quickshell routing and initial EWMH dock struts/workareas are implemented. Physical multi-output and panel validation remain outstanding. |
| IPC | Partial | Protocol v3 adds a persistent Unix-socket subscription that emits change-only snapshots and releases workers on disconnect; Quickshell consumes it via SplitParser with reconnect/fallback. The WM checks state deltas every 100 ms. Structured async mutation results and subscriber backpressure metrics remain. |
| Settings | Partial | Typed settings are validated on a candidate copy and atomically persisted before replacing live state. Undo/reset/profile import-export remain. |
| Dashboard | Partial | Imported QML source and early adapter; not all pages are MRX-backed. |
| Media/audio/network | Upstream-dependent | Must audit each integration and remove unsupported assumptions. |
| Workspace animations | Not implemented | Requires dedicated renderer feasibility and hardware tests. |
| Installer | Experimental | Isolated install/reinstall/uninstall regression tests exist; full VM/session coexistence and release signing remain. Do not use on a daily-use machine. |

See [Phase 1 status](PHASE-1-STATUS.md) for the implementation increment and explicit acceptance blockers.
