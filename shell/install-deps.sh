#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "$0")/.." && pwd)"
echo "Use MRX's root dependency installer; the old BSPWM dependency path is retired."
exec "$ROOT/scripts/install-deps-arch.sh" "$@"
