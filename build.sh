#!/usr/bin/env bash
# Run the same validation gates used by CI before producing release binaries.
set -euo pipefail
cd "$(dirname -- "$0")"

cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
bash ./tests/static-check.sh
bash ./tests/xvfb-server-check.sh
bash ./tests/xvfb-smoke.sh
cargo build --release --workspace

echo "PASS: MRX validation suite and release build completed."
