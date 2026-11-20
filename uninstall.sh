#!/usr/bin/env bash
set -euo pipefail
USER_BIN="${HOME}/.local/bin"
USER_CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/mrx"
QS_CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/quickshell/mrx"
SESSION_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/xsessions"
rm -f "$USER_BIN/mrx-wm" "$USER_BIN/mrxctl" "$USER_BIN/mrx-session" "$SESSION_DIR/mrx.desktop"
if [[ -d "$USER_CONFIG" ]]; then mv "$USER_CONFIG" "${USER_CONFIG}.removed.$(date +%Y%m%d-%H%M%S)"; fi
if [[ -d "$QS_CONFIG" ]]; then mv "$QS_CONFIG" "${QS_CONFIG}.removed.$(date +%Y%m%d-%H%M%S)"; fi
echo "If you manually registered MRX system-wide, remove /usr/share/xsessions/mrx.desktop yourself with sudo."
echo "MRX user files removed to timestamped backups. System-wide sessions and other desktops were not changed."
