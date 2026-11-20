# MRX Desktop

**A community-oriented, Rust-based X11 desktop environment in development.**

MRX combines a purpose-built Rust window manager and desktop services with a Quickshell/QML interface derived from the supplied end4-pC X11 port. The design goal is a cohesive, keyboard-driven desktop with configurable workspaces, a polished control center, media controls, notifications, and responsive live settings.

> [!WARNING]
> **Experimental development snapshot — not ready for daily use.** This repository is not a finished Hyprland replacement. Recent development has added RandR monitor discovery, independent per-monitor workspaces, a versioned IPC protocol, ICCCM urgency handling and initial EWMH dock workarea support. The Phase 1 baseline passed in [GitHub Actions run #300](https://github.com/manivelx/MRX-Desktop/actions/runs/44759250790); Phase 2 persistent IPC subscription tests passed in [run #315](https://github.com/manivelx/MRX-Desktop/actions/runs/44760870725). This does not establish physical hardware or full Quickshell-session compatibility. It has not passed a complete Quickshell session or physical hardware test. Workspace-slide animations are not implemented. Do not install it over your primary desktop or assume NVIDIA 470 compatibility.

## Project goals

- A standalone Rust X11 window manager, rather than a BSPWM wrapper.
- A single Rust-owned state and settings API shared by the window manager and shell.
- Live settings changes without restarting the shell for ordinary options.
- An end4-pC-inspired dashboard, left/right sidebars, quick toggles, media controls, notifications and OSD.
- Predictable window layouts, keyboard navigation, workspace rules and multi-monitor support.
- A separate session with reversible installation and explicit recovery paths.
- Clear documentation, tests and accessible contribution workflows for public development.

## Current status

| Area | Status |
|---|---|
| Rust workspace and crate layout | Scaffolded |
| X11 connection and WM ownership | Initial implementation |
| Basic workspaces and layouts | Partial |
| IPC and command-line client | Versioned protocol, request IDs, bounded framing, timeouts and response limits; event subscriptions remain |
| TOML settings persistence | Initial implementation |
| Quickshell/QML source | Imported and adapted from the supplied X11 port; not fully integrated |
| ICCCM/EWMH behavior | Partial; polite close, focus negotiation/restoration and core desktop messages tested in Xvfb |
| Floating, dialogs and fullscreen | Partial; transient detection added; fullscreen remains incomplete |
| Multi-monitor support | RandR discovery/reconciliation, per-monitor tiling and workspaces, monitor-specific IPC and initial dock workareas implemented; physical hardware validation remains |
| Live, event-driven UI updates | Incomplete; current bridge still needs work |
| Hyprland-like workspace animations | Not implemented |
| Full feature parity with end4-pC | Not implemented |
| NVIDIA 470 / GT 740 validation | Not tested |
| Production installer and upgrade path | Not ready |

See [Feature status](docs/FEATURE_STATUS.md), [Roadmap](ROADMAP.md), and [Validation](docs/VALIDATION.md).

## Architecture at a glance

```text
Quickshell / QML shell
        │ typed local IPC
        ▼
Rust desktop core ───── Linux services (D-Bus, PipeWire, NetworkManager, BlueZ)
        │
        ├── Rust X11 window manager
        ├── Settings and state service
        └── Workspace / animation coordination (renderer is future work)
        │
        ▼
X11 server ── XComposite / XDamage / RandR / XFixes / GLX (as validated)
```

The QML layer is presentation, not the authority for window-management policy. The Rust core should own runtime settings and publish state changes to the shell. Ordinary settings changes must update the relevant component without restarting Quickshell or the session.

## Repository layout

```text
crates/
  mrx-core/       Shared settings, state and IPC models
  mrx-wm/         Rust X11 window manager
  mrxctl/         Command-line IPC client
shell/                Quickshell/QML interface and upstream-derived assets
session/              Separate X11 session entry and launcher
config/               Default settings and optional compositor config
docs/                 Architecture, build, feature and validation notes
.github/              CI and issue templates
```

## Who can use it?

At this stage, the repository is most useful to Rust/Linux developers interested in helping build an X11 desktop environment. It is not currently an end-user installation recommendation. Contributions are welcome for documentation, protocol correctness, tests, accessibility, QML integration and hardware-independent components.

## Build prerequisites

The project targets Arch Linux and CachyOS first. Exact dependencies are still being validated. A starting point is:

```bash
sudo pacman -S --needed \
  rust cargo git base-devel \
  libxcb xorg-server-xvfb xorg-server-xephyr \
  xorg-xrandr xorg-xprop xorg-xwininfo xorg-xmessage xorg-fonts-misc \
  picom quickshell qt6-declarative \
  pipewire wireplumber networkmanager bluez
```

Build and run the available checks:

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo build --workspace
./tests/static-check.sh
```

GitHub Actions is the authoritative source for current validation. A green Rust build does not establish Quickshell runtime compatibility, physical multi-monitor behavior, NVIDIA 470 / GT 740 support or animation performance. Check [Validation](docs/VALIDATION.md) for the exact coverage and remaining hardware/runtime limits.

## Safe development testing

Use a nested X server such as Xephyr or a disposable virtual machine. Do not replace your current window manager or login session while testing. Example test display:

```bash
Xephyr :99 -screen 1280x800 -ac &
DISPLAY=:99 target/debug/mrx-wm
```

Run an X11 test client in another terminal with `DISPLAY=:99`. The command assumes the project has already compiled. See [Testing](docs/TESTING.md).

## Installation status

The included scripts are experimental developer scaffolding. **Do not run `install.sh` on a daily-use system yet.** The session and shell still require a complete integration and safety review. In particular, inspect every command and path before running scripts with elevated privileges. No script should modify another desktop session, change the SDDM default, replace a user's lock screen, or kill unrelated processes.

## Contributing

Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request. Small, reviewable changes with tests and clear status notes are preferred. Do not claim hardware compatibility without recording the hardware, driver, session, test steps and results.

## Licensing and upstream attribution

- The Rust project files are licensed under MIT; see [LICENSE](LICENSE).
- The imported Quickshell/QML source retains its upstream license in `shell/LICENSE` and `docs/END4-LICENSE`.
- Components and assets may have additional licenses. Review [Third-party notices](THIRD_PARTY_NOTICES.md) before redistributing modified bundles or binary releases.
- MRX is an independent community project and is not affiliated with or endorsed by the end4-pC authors, Hyprland, Quickshell, Arch Linux or CachyOS.

## Project status and communication

This is an early public development snapshot. Please use GitHub Issues for reproducible bugs and Discussions for design questions (if Discussions are enabled in the repository). Be respectful, include logs with private data removed, and never upload passwords, tokens, private keys or unredacted personal information.
