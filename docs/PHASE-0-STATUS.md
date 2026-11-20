# Phase 0 — source audit and project identity

Release snapshot: `0.1.0-alpha.12`  
Status: **source rename and static audit completed; runtime compatibility audit remains open**

## Completed in this pass

- Renamed the previous project identity to **MRX** across project-owned source, manifests, session descriptors, documentation, installer messages, IPC socket/config paths, and executable/crate names.
- Renamed Rust crates and directories to `mrx-core`, `mrx-wm`, and `mrxctl`.
- Renamed session launcher/desktop entry to `mrx-session` and `mrx.desktop`.
- Confirmed the previous product identifier is absent from the extracted project tree.
- Verified the MRX session selects the MRX backend and that the MRX backend calls `mrxctl` rather than BSPWM/Hyprland for the MRX session's WM operations.
- Preserved upstream end4 license files and third-party notices.
- Static checks confirm shell syntax, TOML syntax, protected SDDM/lock-screen paths, no GPU/kernel/bootloader mutation in the dependency installer, ICCCM/EWMH source hooks, bounded IPC framing, bounded concurrent IPC clients, and refusal to replace a non-socket IPC path.
- IPC handling now isolates slow clients in bounded per-client workers; fallback runtime directories are private, and the existing XDG_RUNTIME_DIR permissions are left unchanged.
- Xvfb server probe passes at 1920x1080x24 and advertises Composite and GLX. This validates only the virtual X server, not MRX or GPU acceleration.
- MRX session startup now skips the imported Hyprland-specific autostart helper.

## Still open

- Rust compilation, formatting, tests and Clippy cannot be verified in this container because `cargo`/`rustc` are unavailable and package network access is blocked.
- Quickshell/QML imports and every QML module's API compatibility still require `qmllint`/runtime validation against the target Quickshell version. A source scan still finds 22 legacy `hyprctl`/`bspc` command references in upstream shell modules; some are conditionally disabled for MRX, but every user-visible path must be audited and gated before Phase 0 can be signed off.
- The imported shell retains upstream Hyprland, Niri and BSPWM backend implementations for their respective upstream sessions. They are not the selected backend in an MRX session, but MRX-visible pages still contain some conditional legacy commands and require a feature-by-feature runtime audit before claiming complete isolation.
- License compatibility and redistribution obligations need a human legal review before public redistribution.

Phase 0 is therefore complete only for the **identity/source-level pass**, not as a full release-readiness sign-off.
