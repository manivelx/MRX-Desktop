# Feature status

This document separates design intent from implemented behavior. “Partial” means the code exists but is not complete or sufficiently tested for general use.

| Feature | Status | Notes |
|---|---|---|
| X11 connection | Partial | Initial connection and root-window ownership code exists. |
| Window mapping/focus | Partial | Initial unmap withdrawal handling added; focus input models, startup adoption and edge-case tests remain. |
| EWMH desktop metadata | Partial | Desktop names, viewport, workarea and geometry are published; stacking order, struts and full state semantics remain. |
| Workspaces | Partial | Basic workspace state and switching; no slide animation. |
| Master/stack layout | Partial | Shared geometry engine with per-monitor tiling and ICCCM min/max/base/increment/aspect constraints; small-workarea and real-client behavior still need broader validation. |
| Monocle layout | Initial | Shared geometry engine added; runtime testing pending. |
| Floating windows | Not complete | Dialog/transient behavior is not production-ready. |
| Fullscreen | Not complete | Full protocol and restoration behavior required. |
| Multi-monitor | Partial | RandR monitor discovery, hotplug refresh, and per-monitor tiling geometry exist; dock-aware work areas and independent per-monitor workspaces are not implemented. Physical multi-output validation remains outstanding. |
| IPC | Partial | Local Unix socket; protocol needs versioning and event streaming. |
| Settings | Partial | Initial TOML persistence; runtime validation/application needs completion. |
| Dashboard | Partial | Imported QML source and early adapter; not all pages are MRX-backed. |
| Media/audio/network | Upstream-dependent | Must audit each integration and remove unsupported assumptions. |
| Workspace animations | Not implemented | Requires dedicated renderer feasibility and hardware tests. |
| Installer | Experimental | Do not use on a daily-use machine. |

See [Phase 1 status](PHASE-1-STATUS.md) for the implementation increment and explicit acceptance blockers.
