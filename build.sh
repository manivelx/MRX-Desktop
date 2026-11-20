#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname -- "$0")"
cargo fmt --all -- --check
cargo test --workspace
cargo build --release --workspace
