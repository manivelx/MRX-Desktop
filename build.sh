#!/usr/bin/env bash
# Run the same validation gates used by CI before producing release binaries.
set -euo pipefail
cd "$(dirname -- "$0")"

bash ./check-deps.sh
cargo fmt --all
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
bash ./tests/static-check.sh
bash ./tests/mrx-shell-backend.sh
bash ./tests/diagnostics.sh
bash ./tests/installer-sandbox.sh
bash ./tests/xvfb-server-check.sh
bash ./tests/xvfb-smoke.sh
bash ./tests/xephyr-smoke.sh
cargo build --release --workspace
bash ./tests/package-release.sh

echo "PASS: MRX validation suite, release archive and release build completed."
