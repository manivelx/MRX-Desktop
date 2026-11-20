# Contributing to MRX Desktop

Thanks for your interest. MRX is experimental; contributions that improve correctness, documentation and testability are especially valuable.

## Before starting

1. Read `README.md`, `docs/ARCHITECTURE.md`, `docs/FEATURE_STATUS.md` and `docs/VALIDATION.md`.
2. Search existing issues before starting a large change.
3. For major design changes, open an issue first so the architecture can be discussed.
4. Do not assume a feature listed in end4-pC or Hyprland is implemented here.

## Development principles

- Rust owns window-management policy and authoritative runtime state.
- QML is a client of the Rust API; avoid duplicating window-management logic in QML.
- Ordinary settings changes should not restart the shell or the whole session.
- Unsupported operations must return a visible, structured error.
- Never silently fall back to invoking `bspc` or Hyprland commands in the MRX session.
- Keep session installation isolated and reversible.
- Avoid broad `pkill`, destructive cleanup, and unbounded filesystem operations.
- Prefer small modules, typed interfaces, explicit error handling and tests.

## Local checks

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
./tests/static-check.sh
```

If a check cannot run on your system, say so in the pull request. Do not report a test as passing unless it was actually executed.

## Pull requests

A good pull request should include:

- The problem and the reason for the change.
- The design and any compatibility implications.
- Tests added or run, including exact commands.
- Screenshots or recordings for visible UI changes, captured from the actual running build.
- Any known limitations or follow-up work.

Keep unrelated formatting or refactoring out of feature pull requests where possible.

## QML and upstream-derived code

Preserve upstream copyright notices and license files. Clearly distinguish new MRX code from imported or modified upstream code. Do not remove attribution headers. Check whether a dependency or asset has additional redistribution requirements before adding it.

## Security issues

Do not publish exploitable vulnerabilities in public issues. Follow `SECURITY.md`.
