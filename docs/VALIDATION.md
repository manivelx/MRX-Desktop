# Validation record

## Latest successful `main` validation

PASS — GitHub Actions [run #403](https://github.com/manivelx/MRX-Desktop/actions/runs/44765939899) passed on the merged default-branch commit `579dacb6f099726621583fa034b88d9d9df40b28` on 2026-11-20.

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

## Phase 2 IPC subscription branch validation

PASS — GitHub Actions [run #323](https://github.com/manivelx/MRX-Desktop/actions/runs/44761662970) validated the Phase 2 branch head `65118681ba2200da143e6ffe5f600bfaff801857` on 2026-11-20. This is branch-level evidence; it is distinct from the successful `main` baseline above.

The run passed formatting, workspace tests, Clippy with warnings denied, build, static checks, installer sandbox, Xvfb server probe and the compiled-WM Xvfb smoke test. The smoke test starts `mrxctl subscribe`, receives an initial snapshot, changes the active workspace and verifies a change-only snapshot over the same persistent connection. A read probe also detects client disconnects so abandoned subscribers release bounded IPC worker slots.

The Quickshell adapter consumes the newline-delimited stream with `SplitParser`, reconnects after process exit and retains a slow one-shot snapshot fallback while the stream is unavailable. The WM checks state deltas every 100 ms; this removes normal shell polling but is not yet an internal event-broadcast architecture.

## Validation environment

- GitHub-hosted Ubuntu 26.04 runner.
- Xvfb at 1920x1080x24.
- X11 test client `xmessage`.
- Rust stable with rustfmt and Clippy.
- X11 development libraries and Xvfb utilities installed by CI.

## Not implemented / not validated by CI

- Structured asynchronous mutation results, subscriber backpressure metrics and high-rate event-ordering tests remain outstanding. The subscription implementation uses a 100 ms state-delta check rather than an internal event-broadcast architecture.

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

The exact `main` commit named above passed the full automated workflow. This still does not establish physical hardware compatibility or full-session runtime behavior. This is a useful software baseline, not a production-readiness claim. It does not establish Quickshell runtime compatibility, physical monitor behavior, NVIDIA proprietary driver support or animation performance.
