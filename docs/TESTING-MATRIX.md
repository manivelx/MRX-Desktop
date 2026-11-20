# MRX validation matrix

| Layer | Environment | What it can prove | What it cannot prove |
|---|---|---|---|
| Rust unit tests | Cargo test | Settings, serialization, pure logic | X11 behavior or QML rendering |
| Static checks | Shell/TOML/QML source checks | Basic syntax and repository invariants | Runtime correctness |
| Xvfb smoke test | 1920x1080 virtual X server | X11 connection, WM startup, IPC, workspace state | NVIDIA/GLX acceleration or display-driver stability |
| Xephyr test | Nested X server on target PC | Client mapping, focus, shortcuts and shell placement | Exact full-screen compositor performance |
| Quickshell test | Actual X11 session | QML imports, shell windows, widgets and D-Bus services | Driver performance without GPU measurements |
| GT 740 test | Xorg + NVIDIA 470.256.02 | GLX vendor, compositor stability, frame pacing | Other GPU/kernel combinations |

A release must not claim a test passed unless its command ran and its result was
recorded. `not run` is not equivalent to `pass`.

## Local commands

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
./tests/xvfb-smoke.sh
```

The Xvfb test needs `xorg-server-xvfb`, `xorg-xdpyinfo`, `xorg-xprop`, `xterm` and Python 3.
For a nested interactive test, install `xorg-server-xephyr` and use:

```bash
Xephyr :99 -screen 1920x1080 -ac &
DISPLAY=:99 target/debug/mrx-wm
```

Do not start MRX's WM on the user's primary display while another WM owns
`SubstructureRedirect`.
