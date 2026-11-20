# Feature status

This table distinguishes implemented code from complete, validated desktop behavior. “Partial” means real implementation exists but acceptance coverage or full protocol semantics are still outstanding.

| Feature | Status | Notes |
|---|---|---|
| X11 connection and WM ownership | Partial | Rust X11 connection, root selection and event loop are implemented; nested-session and long-session behavior still need validation. |
| Window mapping and focus | Partial | Startup adoption, unmap/destroy cleanup, focus restoration, transient classification, focus negotiation and urgency snapshots are implemented; remaining focus/withdrawal edge cases need broader X server tests. |
| ICCCM size hints | Partial | Min/max/base/increment/aspect constraints and fixed-size floating policy exist; pathological combinations/small workareas need more tests. |
| EWMH desktop metadata | Partial | Desktop names, viewport, geometry, workarea, client lists and stacking are published; complete state semantics remain. |
| EWMH window state | Partial | Close, active/current desktop, per-window desktop, fullscreen, horizontal/vertical maximize with geometry restoration, hidden/minimize lifecycle and mutually exclusive ABOVE/BELOW state requests are implemented. Startup adoption now preserves initial fullscreen/maximize/hidden/ABOVE/BELOW state, ICCCM IconicState and valid per-window desktop assignment; managed-client stacking is reconciled by BELOW/normal/ABOVE/dock/fullscreen layers; exact restoration across WM replacement and complex transitions remain. |
| Workspaces and layouts | Partial | Master/stack, columns, monocle and independent monitor-local workspace state are implemented; animation and real-client edge coverage remain. |
| Floating and dialogs | Partial | Transient parents are tracked, transient/fixed-size clients float, EWMH modal state is adopted, and focus requests are redirected through visible modal descendants. Full withdrawal, modality and nested-dialog edge cases need more validation. |
| Multi-monitor | Partial | RandR discovery/reconciliation, per-monitor tiling/workspaces, monitor-aware IPC and QML routing, hotplug state refresh and initial dock workareas are implemented. Physical multi-output, EDID and mixed-resolution behavior remain untested. |
| Dock panels | Partial | Partial and legacy struts influence workareas; overlapping, vertical and multi-panel geometry needs physical/Xephyr coverage. |
| IPC | Partial | Protocol v3 includes a persistent Unix-socket subscription that emits change-only snapshots and releases subscriber workers on disconnect; Quickshell consumes the stream via SplitParser with reconnect and slow snapshot fallback. The WM checks state deltas every 100 ms; structured async mutation results and backpressure metrics remain. |
| Settings | Partial | Typed settings are validated on a candidate copy and atomically persisted before replacing live state. Undo/reset/profile import-export remain. |
| Shell integration | Partial | MRX adapter maps supported window/workspace APIs; not every imported QML service or action has a verified MRX implementation. |
| Graphics/animations | Not implemented | XComposite/Damage capture, renderer, transitions and frame pacing are future work. |
| Installer | Experimental | Isolated temporary-home install/reinstall/backup/uninstall tests pass in CI; VM/session coexistence, upgrade policy and signed release artifacts remain. |

See [Phase 1 status](PHASE-1-STATUS.md), [Roadmap](../ROADMAP.md) and [Validation](VALIDATION.md) for acceptance boundaries.
