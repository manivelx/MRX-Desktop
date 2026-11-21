# Building from source

MRX currently targets Arch Linux and CachyOS. Dependencies are provisional until the QML imports and X11 extension requirements are fully audited.

## Toolchain

- Stable Rust toolchain with Cargo, rustfmt and Clippy.
- X11 development/runtime libraries.
- Quickshell and the Qt modules required by the imported QML source.
- Xvfb, Xephyr, xmessage, xwininfo, xprop, xdpyinfo, xrandr and Python 3 for automated integration tests.

On Arch/CachyOS, install the project dependency set with:

```bash
./install.sh --install-deps
```

The dependency installer must not install or modify the NVIDIA driver, kernel, bootloader, SDDM theme, BSPWM session or existing lock screen.

## Full validation and release build

Run:

```bash
./build.sh
```

This runs the complete local validation suite and release build, in order:

1. Verify required Rust, Quickshell, X11 and nested-X test dependencies.
2. Format Rust sources, then verify formatting is clean.
2. Workspace unit and integration tests.
3. Clippy with warnings denied.
4. Debug workspace build.
6. Static shell/TOML/session-safety checks.
7. Installer/reinstaller/uninstaller sandbox.
8. Xvfb server probe.
9. Compiled-WM Xvfb integration smoke test.
10. Complete compiled-WM smoke suite inside nested Xephyr.
11. Release workspace build.

The Xvfb and Xephyr tests are isolated software-server tests. They cover virtual dock topology and workarea changes but do not validate Quickshell rendering, physical multi-monitor hotplug, NVIDIA 470, GT 740 acceleration or animation performance.

Individual commands:

```bash
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

The installer remains experimental. Do not run it on a daily-use system until Phase 5's ownership, rollback and VM validation gates are complete.
