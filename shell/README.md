# MRX Quickshell frontend

This directory contains the end4-pC-derived Quickshell/QML frontend used by
MRX Desktop. The active session is **MRX Rust WM + X11**, not BSPWM.

## Runtime integration

- `services/WM.qml` detects `XDG_CURRENT_DESKTOP=MRX` and creates
  `MRXBackend.qml`.
- `MRXBackend.qml` obtains desktop state through the `mrxctl` client.
- The client talks to the Rust WM over a per-user Unix-domain socket; there are
  no network TCP ports to configure.
- The active shell configuration is installed to
  `${XDG_CONFIG_HOME:-~/.config}/quickshell/mrx` and started with `qs -c mrx`.
- The frontend requests X11 panels through Quickshell's XCB/X11 platform path.

## Important compatibility notes

The upstream QML source includes compatibility branches for Hyprland, Niri and
BSPWM. Those branches are retained where useful for upstream source maintenance,
but MRX must use the `mrx` backend at runtime. Any control that has no
MRX implementation must be hidden or explicitly disabled; it must not fall
through to `hyprctl` or `bspc`.

Some upstream widgets still need a complete command audit. The presence of a
QML page or control is not proof that its action is implemented by the Rust WM.
See the root `ROADMAP.md` and `docs/TESTING-MATRIX.md` for feature status.

## Build and install

Use the repository root scripts only:

```bash
./install.sh --install-deps
cargo build --release --workspace
./install.sh
```

Do not run an upstream BSPWM installer. MRX does not require BSPWM or SXHKD
as its window manager or keybinding daemon. Its lock-screen launcher is reused
without modifying `~/.local/share/quickshell-lockscreen`.
