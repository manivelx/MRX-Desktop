# Validation record

## Latest successful \`main\` validation

PASS — GitHub Actions [run #518](https://github.com/manivelx/MRX-Desktop/actions/runs/44912544584) passed on the merged default-branch commit \`ebbbce695428a284c75e1a8d341a36a7edef487e\` on 2026-11-21.

| Check | Result |
|---|---|
| \`cargo fmt --all -- --check\` | PASS |
| \`cargo test --workspace\` | PASS |
| \`cargo clippy --workspace --all-targets -- -D warnings\` | PASS |
| \`cargo build --workspace\` | PASS |
| \`bash ./tests/static-check.sh\` | PASS |
| \`bash ./tests/installer-sandbox.sh\` | PASS |
| \`bash ./tests/xvfb-server-check.sh\` | PASS |
| \`bash ./tests/xvfb-smoke.sh\` | PASS |
| \`bash ./tests/xephyr-smoke.sh\` | PASS |

## Quickshell MRX X11 action validation

PASS — GitHub Actions [PR #20 run](https://github.com/manivelx/MRX-Desktop/actions/runs/44914024803) passed on branch commit `73cff5cdd7435f16c7781a106f84bbb2c4ee0a06`.

The run passed formatting, workspace tests, Clippy, build, static shell/QML checks, the mocked MRX shell backend test, installer sandbox and both Xvfb/Xephyr integration suites.

The mocked helper test verifies:
- MRX, BSPWM, Niri and Hyprland session detection.
- MRX monitor names, focused monitor and monitor geometry through `mrxctl snapshot`.
- X11 root geometry via xdotool.
- X11 session classification without invoking `hyprctl`.

The shell scripts now use the MRX/X11 path for static wallpaper placement and screen recording. Video wallpapers are explicitly unsupported in the current X11 backend. This does not replace a live Quickshell test: visual rendering, interactive controls, actual recording, wallpaper application and full session stability remain to be tested on CachyOS-LTS.

## XComposite and XDamage prototype branch validation

PASS — GitHub Actions [PR #19 run](https://github.com/manivelx/MRX-Desktop/actions/runs/44912799891) passed on branch commit \`b63a00f4ed1e4c9558922dca809ec3bf845438f8\` on 2026-11-21.

The run passed formatting, workspace tests, Clippy with warnings denied, debug build, static checks, installer sandbox, Xvfb server probe, Xvfb integration smoke and the complete nested Xephyr integration smoke.

The standalone \`mrx-graphics-probe\` produced these results on both Xvfb and nested Xephyr:

- XComposite 0.4 successfully redirected one disposable test window and named a 64×64 off-screen pixmap.
- XDamage 1.1 delivered a non-empty damage event after a deterministic window update.
- The probe released the Damage object, named pixmap, redirection and temporary window.

This verifies the protocol path only. The production WM does not redirect managed client windows, and the test does not validate GLX, GPU acceleration, frame pacing or a user-visible compositor.

## Nested Xephyr and dock geometry validation

PASS — GitHub Actions [PR #18 run](https://github.com/manivelx/MRX-Desktop/actions/runs/44912263933) passed on branch commit \`f908e9bd9eb66448340f1b6bc643aa073cbeba55\`.

The run passed formatting, workspace tests, Clippy with warnings denied, debug build, static checks, installer sandbox, Xvfb server probe, the compiled-WM Xvfb smoke suite and the complete compiled-WM smoke suite inside nested Xephyr.

The suite exercises simultaneous top/vertical dock struts, same-edge maximum reservations and workarea restoration after dock removal. Mixed-resolution monitor strut intersection is covered by Rust unit tests. Xephyr also exposed and verified a fix for RandR servers that report an automatic root/output monitor alongside explicit RandR 1.5 monitor objects.

## Integration coverage

- IPC startup, health, subscriptions, structured operation results and orderly shutdown.
- WM supervisor crash recovery and clean restart behavior.
- Virtual RandR monitor objects, monitor-local workspaces and monitor geometry refresh.
- EWMH desktop metadata, maximize/restore, minimize/restore, ABOVE/BELOW stacking and dock workareas.
- ICCCM urgency, pre-existing clients, transient/modal dialogs, focus redirection and client lifecycle.
- Settings persistence, undo/reset and named profile import/export.
- Isolated XComposite named-pixmap capture and XDamage event delivery.

## Validation environment

- GitHub-hosted Ubuntu runner.
- Xvfb at 1920x1080x24.
- Nested Xephyr at 1920x1080, backed by a separate host Xvfb display.
- X11 test clients, xwininfo and RandR utilities.
- Rust stable with rustfmt and Clippy.
- X11 development libraries, Xvfb and Xephyr installed by CI.

## Not implemented / not validated by CI

- Complete Quickshell/QML runtime or end-to-end desktop session.
- Real physical multi-monitor hotplug, reconnect, EDID, mixed DPI or mixed resolutions.
- Physical overlapping/multiple dock geometry and mixed-DPI behavior on actual Xorg sessions.
- NVIDIA 470 / GeForce GT 740 proprietary driver, real GLX acceleration or animation frame pacing.
- Production managed-client XComposite capture lifecycle, XDamage-driven redraw, renderer recovery and workspace animation performance.
- Suspend/resume, power management and long-running session stability.
- Install/upgrade/removal in a disposable Arch/CachyOS VM; current installer test is a temporary-home sandbox.

## Local reproduction

On an Arch/CachyOS development machine with Rust, Quickshell and X11 test dependencies installed:

\`\`\`bash
./check-deps.sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
bash ./tests/static-check.sh
bash ./tests/mrx-shell-backend.sh
bash ./tests/installer-sandbox.sh
bash ./tests/xvfb-server-check.sh
bash ./tests/xvfb-smoke.sh
bash ./tests/xephyr-smoke.sh
cargo build --release --workspace
\`\`\`

Or run the complete sequence with \`./build.sh\`. Use a disposable VM or nested X server for manual session tests. Keep the current desktop session and recovery path untouched.

## Interpretation

The main commit and prototype branches named above passed their recorded automated workflows. This does not establish physical hardware compatibility or full-session runtime behavior. MRX remains experimental until Quickshell, physical display paths, graphics fallback/recovery and Arch/CachyOS VM installation/rollback have been validated.
