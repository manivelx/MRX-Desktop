# Building from source

MRX currently targets Arch Linux and CachyOS. Dependencies are provisional until the QML imports and X11 extension requirements are fully audited.

## Toolchain

- Stable Rust toolchain with Cargo.
- X11 development/runtime libraries.
- Quickshell and the Qt modules required by the imported QML source.
- Optional Xephyr/Xvfb for isolated testing.

## Commands

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo build --workspace
```

For a release build:

```bash
cargo build --release --workspace
```

The current installer expects release binaries, but it is experimental and should not be run on a daily-use system. See the root README and `docs/VALIDATION.md`.
