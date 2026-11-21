# Phase 5 — Session, installer and release engineering

## Implemented and CI-covered

- Reversible user-scoped install/uninstall scaffolding with isolated temporary-home regression tests.
- Ubuntu CI and a native \`archlinux:latest\` container job both run Rust formatting, workspace tests, Clippy, debug and release builds, static checks, shell helper tests, installer sandbox, Xvfb and nested Xephyr integration.
- Reproducible x86_64/aarch64 Linux binary archives include the Rust binaries, MRX session, Quickshell shell, config, installer, scripts, docs and license notices; tar ordering/timestamps/ownership and gzip headers are normalized, with SHA-256 verification and deterministic rebuild tests.
- A version-tagged GitHub Release workflow checks the tag against the Cargo workspace version and uploads the archive and checksum. No public release is published until a matching version tag is pushed.
- Arch CI installs the native Arch Rust toolchain and X11/Xephyr packages through pacman, validating the Linux package names and build environment independently of Ubuntu.

## Still outstanding

- Test installation, upgrade, rollback and removal in disposable Arch and CachyOS-LTS VMs.
- Validate session registration and coexistence with existing BSPWM, SDDM, Orbital theme and Quickshell lock-screen configuration.
- Add startup recovery, diagnostics and safe rollback after failed session startup.
- Produce reproducible release artifacts, checksums, signatures and release notes.
- Complete the real-hardware compatibility matrix, including the legacy NVIDIA 470/GT 740 path.

## Validation boundary

The Arch CI job is a container build/integration test, not a booted VM. It does not validate SDDM, logind session startup, kernel modules, GPU acceleration, physical displays or desktop-session coexistence. The user's current CachyOS installation and desktop configuration remain untouched until a disposable CachyOS-LTS VM or explicitly designated test installation is available.
