# Validation record

## Latest CI result

The development branch has changed substantially since the historical baseline below. Commit-specific validation is being rerun after the latest RandR, per-monitor workspace, dock and urgency changes. Check the [GitHub Actions history](https://github.com/manivelx/MRX-Desktop/actions) for the exact latest head and result. Do not treat a cancelled, superseded or failed run as a passing validation.

The workflow covers Rust formatting, workspace tests, Clippy with warnings denied, build, static checks, isolated installer regression tests, Xvfb server startup and the compiled WM smoke test.

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