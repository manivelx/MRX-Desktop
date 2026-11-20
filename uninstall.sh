#!/usr/bin/env bash
set -euo pipefail
USER_BIN="${HOME}/.local/bin"
USER_CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/mrx"
QS_CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/quickshell/mrx"
SESSION_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/xsessions"
rm -f "$USER_BIN/mrx-wm" "$USER_BIN/mrxctl" "$USER_BIN/mrx-session" "$SESSION_DIR/mrx.desktop"
if [[ -d "$USER_CONFIG" ]]; then mv "$USER_CONFIG" "${USER_CONFIG}.removed.$(date +%Y%m%d-%H%M%S)"; fi
if [[ -d "$QS_CONFIG" ]]; then mv "$QS_CONFIG" "${QS_CONFIG}.removed.$(date +%Y%m%d-%H%M%S)"; fi
if [[ -w /usr/share/xsessions/mrx.desktop ]]; then rm -f /usr/share/xsessions/mrx.desktop; else echo "If installed system-wide, remove /usr/share/xsessions/mrx.desktop manually with sudo."; fi
echo "MRX user files removed to timestamped backups. Other sessions were not changed."
