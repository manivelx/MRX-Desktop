# Validation record

## Environment used for this engineering pass

- Container OS: Debian GNU/Linux 13 (trixie)
- Container kernel: `6.18.57 #1 SMP Tue Oct 20 2026`
- Virtual display: Xvfb 1920x1080x24
- Available X11 utilities: Xvfb, xdpyinfo, xprop, xwininfo, xterm
- Python-Xlib available for a basic X11 client round trip
- No Rust compiler/Cargo found in the container
- No Quickshell executable found in the container
- No `/dev/dri` device nodes or NVIDIA device nodes exposed to the container
- Package network access unavailable; `apt-get update` timed out

The container kernel version is not the user's CachyOS kernel. It must not be
used as evidence of compatibility with the user's installed NVIDIA driver.

## Tests actually run

| Test | Result | Notes |
|---|---|---|
| `bash -n` on repository scripts | PASS | Shell syntax only |
| TOML parse of workspace and default settings | PASS | Python `tomllib` |
| `tests/static-check.sh` | PASS | Repository invariants and static checks |
| Xvfb 1920x1080 startup | PASS | Virtual framebuffer only |
| Python-Xlib create/map/query/destroy | PASS | Basic X11 round trip |
| X11 Composite extension advertisement | PASS | Xvfb server extension only |
| X11 GLX extension advertisement | PASS | Xvfb advertises GLX; this is not NVIDIA hardware acceleration |
| Picom XRender backend startup | PASS | Started in Xvfb; no GPU acceleration implied |
| Picom GLX backend startup | PASS | Started in Xvfb; GLX vsync unavailable in this virtual server |
| Rust formatting/tests/build | NOT RUN | Rust/Cargo unavailable and package network blocked |
| MRX WM Xvfb integration test | NOT RUN | Requires compiled Rust binaries |
| Quickshell/QML runtime | NOT RUN | Quickshell unavailable |
| NVIDIA 470 / GT 740 | NOT TESTED | No GPU devices or driver exposed |
| Real GLX/Picom frame pacing | NOT TESTED | Requires target GPU and real Xorg |

## Interpretation

The Xvfb test demonstrates that this container can host a basic X11 server and
client. It does not compile MRX, launch the Rust WM, validate Qt/QML, emulate
the NVIDIA proprietary driver, or measure GPU animation performance.

The Rust source remains unverified until CI or an Arch/CachyOS machine runs:

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
./tests/xvfb-smoke.sh
```

Do not advertise this snapshot as crash-free or NVIDIA-tested before those
checks pass and the results are attached to a release.


## Alpha.8 validation boundary

The alpha.8 source pass runs `tests/static-check.sh` and `tests/xvfb-server-check.sh` in the authoring container. Both pass. The static checks now also guard desktop metadata, per-window desktop requests and initial unmap handling. The Xvfb check validates the X server and extensions only; it does not launch the Rust WM. This container has no `cargo`, `rustc`, `rustfmt`, `qmllint`, or NVIDIA device, so Rust compilation, unit tests, Clippy, QML validation, Xephyr WM integration, and proprietary NVIDIA 470 validation remain unverified. CI is configured to run formatting, Rust tests, Clippy, build, static checks, Xvfb server probe, and the built-WM Xvfb smoke test.
