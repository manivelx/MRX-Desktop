# Validation record

## Latest CI result

PASS — GitHub Actions [run #315](https://github.com/manivelx/MRX-Desktop/actions/runs/44760870725) validated the Phase 2 event-subscription branch head `743e58dd22fa4cf98ba3ade752ebff58a57cfd9b` on 2026-11-20.

The run passed formatting, workspace tests, Clippy with warnings denied, build, static checks, installer sandbox, Xvfb server probe and the compiled WM Xvfb smoke test. In addition to the Phase 1 coverage, the smoke test starts `mrxctl subscribe`, receives the initial snapshot, switches workspaces, and verifies a second change-only snapshot arrives over the same persistent connection. The Quickshell adapter now consumes this stream using `SplitParser` and retains a slow snapshot fallback while the stream is unavailable.

The WM currently checks serialized snapshot changes at 100 ms intervals and emits only when the state changes; this removes the shell's normal 500 ms polling loop but is not yet a fully event-broadcast implementation inside the WM.

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