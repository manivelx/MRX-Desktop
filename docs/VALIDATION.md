# Validation record

## Latest CI result

PASS — GitHub Actions [run #323](https://github.com/manivelx/MRX-Desktop/actions/runs/44761662970) validated the Phase 2 IPC subscription branch head `65118681ba2200da143e6ffe5f600bfaff801857` on 2026-11-20.

The run passed formatting, workspace tests, Clippy with warnings denied, build, static checks, installer sandbox, Xvfb server probe and the compiled WM Xvfb smoke test. The smoke test starts `mrxctl subscribe`, receives an initial snapshot, changes the active workspace and verifies a change-only snapshot arrives over the same persistent connection. The WM now also probes the subscription socket for disconnects so abandoned subscribers release their bounded IPC worker slots.

The Quickshell adapter consumes the stream with `SplitParser`, reconnects after process exit and retains a slow one-shot snapshot fallback while the stream is unavailable. The WM checks snapshot changes at 100 ms intervals; this removes normal shell polling but is not yet an internal event-broadcast architecture.

## Validation environment and limits

- CI runner: GitHub-hosted Ubuntu 26.04.
- Virtual display: Xvfb 1920x1080x24.
- Test client: `xmessage`.
- The separate Xvfb server probe verifies X server startup and extension advertisement only. WM integration coverage is provided by `tests/xvfb-smoke.sh`.
- The local authoring container does not have Rust/Cargo, Quickshell, or GPU device nodes; compilation and WM integration were therefore validated in GitHub Actions, not locally.
- No physical multi-monitor hotplug/disconnect test has been run.
- Quickshell/QML runtime and full desktop-session behavior have not been tested.
- NVIDIA 470 / GeForce GT 740, real GLX acceleration, animation performance, and the user's CachyOS hardware have not been tested.

## Interpretation

The current Rust workspace compiles, passes its unit tests and Clippy, and passes the isolated Xvfb WM smoke test on the CI runner. This is meaningful baseline validation, not a claim that the desktop environment is production-ready.

The Xvfb run does not validate Quickshell rendering, physical multi-monitor hotplug, the proprietary NVIDIA driver, GPU acceleration, or animation frame pacing. Do not advertise the project as crash-free or GT 740/NVIDIA-tested.

## Reproduction

On an Arch/CachyOS development machine with Rust and the X11 test dependencies installed:

```bash
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
bash ./tests/static-check.sh
bash ./tests/xvfb-server-check.sh
bash ./tests/xvfb-smoke.sh
```

Use a disposable VM or nested X server for manual session tests. Keep the existing desktop session and recovery path untouched.