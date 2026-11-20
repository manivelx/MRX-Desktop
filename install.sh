#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "$0")" && pwd)"

case "${1:-}" in
  --install-deps)
    exec "$ROOT/scripts/install-deps-arch.sh"
    ;;
  --help|-h)
    cat <<'MSG'
MRX Desktop installer

Usage:
  ./install.sh                 Install already-built MRX files for this user
  ./install.sh --install-deps  Install build/runtime dependencies on Arch/CachyOS
  ./build.sh                   Format, test and build the Rust workspace
  ./tests/xvfb-smoke.sh        Test the WM in an isolated Xvfb server

The dependency installer never installs or changes NVIDIA drivers, kernels,
bootloaders, SDDM themes, BSPWM sessions or the existing lock screen.
MSG
    exit 0
    ;;
  "") ;;
  *) echo "Unknown option: $1 (use --help)" >&2; exit 2 ;;
esac

BIN="$ROOT/target/release"
USER_BIN="${HOME}/.local/bin"
USER_CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/mrx"
QS_CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/quickshell/mrx"
SESSION_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/xsessions"

missing=()
for command_name in qs; do
  command -v "$command_name" >/dev/null 2>&1 || missing+=("$command_name")
done
if ((${#missing[@]})); then
  printf 'Missing recommended/build commands: %s\n' "${missing[*]}" >&2
  echo "On Arch/CachyOS, run: ./install.sh --install-deps" >&2
  exit 1
fi

[[ -x "$BIN/mrx-wm" && -x "$BIN/mrxctl" ]] || {
  echo "Release binaries are missing. Run ./build.sh first." >&2
  exit 1
}

# Quickshell's X11 frontend needs the XCB QPA plugin. Do not alter global Qt
# plugin paths or replace the user's installed Qt/Quickshell packages.
if [[ "${QT_QPA_PLATFORM:-xcb}" != "xcb" ]]; then
  echo "Warning: QT_QPA_PLATFORM is '${QT_QPA_PLATFORM}', expected xcb for MRX X11." >&2
fi

mkdir -p "$USER_BIN" "$USER_CONFIG" "$QS_CONFIG" "$SESSION_DIR"
install -m755 "$BIN/mrx-wm" "$USER_BIN/mrx-wm"
install -m755 "$BIN/mrxctl" "$USER_BIN/mrxctl"
install -m755 "$ROOT/session/mrx-session" "$USER_BIN/mrx-session"
[[ -e "$USER_CONFIG/settings.toml" ]] || install -m644 "$ROOT/config/settings.toml" "$USER_CONFIG/settings.toml"
install -m644 "$ROOT/config/picom.conf" "$USER_CONFIG/picom.conf"

# Preserve only the MRX-specific shell directory if reinstalling it.
if [[ -d "$QS_CONFIG" ]]; then
  cp -a "$QS_CONFIG" "${QS_CONFIG}.backup.$(date +%Y%m%d-%H%M%S)"
fi
cp -a "$ROOT/shell/." "$QS_CONFIG/"
install -m644 "$ROOT/session/mrx.desktop" "$SESSION_DIR/mrx.desktop"

if [[ -d /usr/share/xsessions ]]; then
  if [[ -w /usr/share/xsessions ]]; then
    install -m644 "$ROOT/session/mrx.desktop" /usr/share/xsessions/mrx.desktop
  elif command -v sudo >/dev/null 2>&1; then
    echo "To expose MRX in SDDM, run: sudo install -Dm644 '$ROOT/session/mrx.desktop' /usr/share/xsessions/mrx.desktop"
  fi
fi

cat <<MSG
MRX user files installed.
Ensure $USER_BIN is in PATH.
Build validation commands: cargo test --workspace and ./tests/xvfb-smoke.sh.
Your existing BSPWM session, SDDM theme and lock-screen directory were not edited.
MSG
