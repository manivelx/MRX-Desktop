# Validation record

## Latest successful `main` validation

PASS — GitHub Actions [run #502](https://github.com/manivelx/MRX-Desktop/actions/runs/44834724872) passed on the merged default-branch commit `f55352e408e43613082d1e7bb1f916bb3b3b2f0e` on 2026-11-20.

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

## Nested Xephyr integration branch validation

PASS — GitHub Actions [PR #17 run](https://github.com/manivelx/MRX-Desktop/actions/runs/44911498714) passed on branch commit `7d11ddb864d067c05fdf59a7f424d9f68cdde377` on 2026-11-21.

The run passed formatting, workspace tests, Clippy with warnings denied, debug build, static checks, installer sandbox, Xvfb server probe, the compiled-WM Xvfb smoke suite and the complete compiled-WM smoke suite inside nested Xephyr.

The nested Xephyr run exposed and verified a fix for RandR servers that report an automatic root/output monitor alongside explicit RandR 1.5 monitor objects. MRX now prefers the explicit monitor objects when they exist, avoiding an overlapping synthetic full-screen monitor in the workspace topology.

The integration suite exercises:
- IPC startup, health, subscriptions, structured operation results and orderly shutdown.
- WM supervisor crash recovery and clean restart behavior.
- Virtual RandR monitor objects, monitor-local workspaces and monitor geometry refresh.
- EWMH desktop metadata, maximize/restore, minimize/restore, ABOVE/BELOW stacking and dock workareas.
- ICCCM urgency, pre-existing clients, transient/modal dialogs, focus redirection and client lifecycle.
- Settings persistence, undo/reset and named profile import/export.

This is nested-X software validation. It does not validate physical monitor hotplug, EDID behavior or GPU acceleration.

## Validation environment

- GitHub-hosted Ubuntu runner.
- Xvfb at 1920x1080x24.
- Nested Xephyr at 1920x1080, backed by a separate host Xvfb display.
- X11 test clients and RandR utilities.
- Rust stable with rustfmt and Clippy.
- X11 development libraries, Xvfb and Xephyr installed by CI.

## Not implemented / not validated by CI

- Complete Quickshell/QML runtime or end-to-end desktop session.
- Real physical multi-monitor hotplug, reconnect, EDID, mixed DPI or mixed resolutions.
- Dedicated overlapping/vertical/multiple dock geometry validation on physical Xorg sessions.
- NVIDIA 470 / GeForce GT 740 proprietary driver, real GLX acceleration or animation frame pacing.
- XComposite capture lifecycle, XDamage-driven redraw, renderer recovery and workspace animation performance.
- Suspend/resume, power management and long-running session stability.
- Install/upgrade/removal in a disposable Arch/CachyOS VM; current installer test is a temporary-home sandbox.

## Local reproduction

On an Arch/CachyOS development machine with Rust, Quickshell and X11 test dependencies installed:

```bash
./check-deps.sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
bash ./tests/static-check.sh
bash ./tests/installer-sandbox.sh
bash ./tests/xvfb-server-check.sh
bash ./tests/xvfb-smoke.sh
bash ./tests/xephyr-smoke.sh
cargo build --release --workspace
```

Or run the complete sequence with `./build.sh`. Use a disposable VM or nested X server for manual session tests. Keep the current desktop session and recovery path untouched.

## Interpretation

The main commit and nested-X development branch named above passed their recorded automated workflows. This does not establish physical hardware compatibility or full-session runtime behavior. MRX remains experimental until Quickshell, physical display paths, graphics fallback/recovery and Arch/CachyOS VM installation/rollback have been validated.
