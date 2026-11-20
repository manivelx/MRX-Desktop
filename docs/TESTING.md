# Testing guide

## Testing principles

- Test in a disposable VM or nested X server first.
- Keep the existing desktop session available for recovery.
- Never run experimental installers with elevated privileges without reviewing the script.
- Record actual output; do not infer success from a command returning no visible error.

## Static checks

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
./tests/static-check.sh
```

## Nested X server

After a successful build, start a nested server:

```bash
Xephyr :99 -screen 1280x800 -ac &
DISPLAY=:99 target/debug/mrx-wm
```

In another terminal, launch an X11 client with `DISPLAY=:99`. Check mapping, focus, close, workspace changes, restart behavior and logs. Do not use the nested server command as a substitute for a full integration test.

## Hardware validation record

For every GPU test, record:

- GPU model and driver version.
- Kernel and Xorg versions.
- Compositor and rendering backend.
- Resolution, refresh rate and monitor connection.
- Test scenario, duration and measured frame times.
- CPU/GPU utilization, visual artifacts and logs.

Do not enable an experimental animation renderer by default until the fallback path and failure recovery have been tested.
