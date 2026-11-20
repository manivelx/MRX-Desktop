# Architecture and implementation contracts

## Process model

`mrx-wm` owns the X11 `SubstructureRedirect` selection and is the only window-manager process. It publishes root EWMH metadata and manages client/workspace state. `mrxctl` is a stateless Unix-socket client. Quickshell is a presentation process and must never own window policy.

## IPC

One JSON request per line over `$XDG_RUNTIME_DIR/mrx.sock`; socket mode 0600. Requests are tagged by `command`. Replies are JSON. Future versioning should add `protocol_version`, request IDs, event subscriptions and structured error codes. Never expose a TCP listener.

## Settings

Runtime settings are owned by the Rust core. QML asks the core to validate and apply a setting; the core atomically persists TOML. Settings must not trigger a whole-shell restart. Unsupported settings must return an explicit error, not silently succeed.

## X11 policy

Use x11rb and explicit protocol extension features. Implement ICCCM/EWMH incrementally and test with common X11 applications. The WM must not assume that X11 client windows are all normal toplevels. Treat docks, dialogs, splash screens, override-redirect windows, transient windows and fullscreen clients separately.

## Animation boundary

Workspace animation is a separate renderer project. The WM publishes workspace transitions and geometry; renderer captures/retains window content via XComposite and damage tracking, and presents frames using a validated GLX/Present path. Do not add an unverified Picom animation patch as a dependency. GT 740/NVIDIA 470 testing is mandatory before enabling the renderer by default.

## Session safety

MRX owns only its own binaries, session entry, `~/.config/mrx`, and `~/.config/quickshell/mrx`. It must not kill processes by broad name, change SDDM defaults, replace the existing lock screen, or mutate the user's BSPWM/end4 sessions.
