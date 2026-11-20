#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
STATE_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/mrx"
VENV="$STATE_DIR/.venv"
mkdir -p "$STATE_DIR"
if ! command -v uv >/dev/null 2>&1; then
    echo "uv is required. Install it with: sudo pacman -S uv" >&2
    exit 1
fi
uv python install 3.12
uv venv --python 3.12 "$VENV"
uv pip install --python "$VENV/bin/python" -r "$ROOT/requirements.txt"
echo "Created the isolated Python environment at $VENV"
