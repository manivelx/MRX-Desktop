# Validation record

## Latest successful full CI baseline (before IPC v3 protocol bump)

PASS — GitHub Actions [run #327](https://github.com/manivelx/MRX-Desktop/actions/runs/44761884015) passed on commit `422615cab39c298505d49e996ce80d927e165551` on 2026-11-20.

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo test --workspace` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo build --workspace` | PASS |
| `bash ./tests/static-check.sh` | PASS |
| `bash ./tests/installer-sandbox.sh` | PASS |
| `bash ./tests/xvfb-server-check.sh` | PASS |
| `bash ./tests/xvfb-smoke.sh` | PASS |

The compiled WM smoke test covers:
- IPC startup, snapshot and workspace switching.
- Two virtual RandR monitor objects and synthetic root-screen filtering.
- Independent monitor-local workspaces and workspace snapshot state.
- EWMH desktop metadata and dock partial-strut workarea reservation.
- Sticky dock behavior without keyboard-focus theft.
- Live setting persistence.
- ICCCM `WM_HINTS` urgency updates via a correctly typed X property.
- Client mapping, workspace visibility and close lifecycle.

## Validation environment

- GitHub-hosted Ubuntu 26.04 runner.
- Xvfb at 1920x1080x24.
- X11 test client `xmessage`.
- Rust stable with rustfmt and Clippy.
- X11 development libraries and Xvfb utilities installed by CI.

## Not validated by CI

- Complete Quickshell/QML runtime or end-to-end desktop session.
- Real physical multi-monitor hotplug, reconnect, EDID, mixed DPI or mixed resolutions.
- NVIDIA 470 / GeForce GT 740 proprietary driver, real GLX acceleration or animation frame pacing.
- Suspend/resume, power management and long-running session stability.
- Install/upgrade/removal in a disposable Arch/CachyOS VM; current installer test is a temporary-home sandbox.

## Local reproduction

On an Arch/CachyOS development machine with Rust and X11 test dependencies installed:

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
bash ./tests/static-check.sh
bash ./tests/installer-sandbox.sh
bash ./tests/xvfb-server-check.sh
bash ./tests/xvfb-smoke.sh
```

Use a disposable VM or nested X server for manual session tests. Keep the current desktop session and recovery path untouched.

## Interpretation

The exact commit named above passed the then-current automated CI suite; it predates the current branch's IPC v3 protocol bump. This is a useful software baseline, not a production-readiness claim. It does not establish Quickshell runtime compatibility, physical monitor behavior, NVIDIA proprietary driver support or animation performance.
