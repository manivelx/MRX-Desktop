# Building from source

MRX currently targets Arch Linux and CachyOS. Dependencies are provisional until the QML imports and X11 extension requirements are fully audited.

## Toolchain

- Stable Rust toolchain with Cargo, rustfmt and Clippy.
- X11 development/runtime libraries.
- Quickshell and the Qt modules required by the imported QML source.
- Xvfb, xmessage, xprop, xdpyinfo and Python 3 for isolated integration tests.
- Optional Xephyr for nested manual testing.

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

This runs the same gates as CI, in order:

1. Rust formatting check.
2. Workspace unit and integration tests.
3. Clippy with warnings denied.
4. Debug workspace build.
5. Static shell/TOML/session-safety checks.
6. Xvfb server probe.
7. Compiled-WM Xvfb integration smoke test.
8. Release workspace build.

The Xvfb test is isolated and does not validate Quickshell rendering, physical multi-monitor hotplug, NVIDIA 470, GT 740 acceleration or animation performance.

Individual commands:

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
bash ./tests/static-check.sh
bash ./tests/xvfb-server-check.sh
bash ./tests/xvfb-smoke.sh
cargo build --release --workspace
```

The installer remains experimental. Do not run it on a daily-use system until Phase 5's ownership, rollback and VM validation gates are complete.
