#!/usr/bin/env bash
set -euo pipefail
USER_BIN="${HOME}/.local/bin"
USER_CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/mrx"
QS_CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/quickshell/mrx"
SESSION_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/xsessions"
restore_latest_backup() {
  local destination="$1"
  local backups=()
  shopt -s nullglob
  backups=( "${destination}.backup."* )
  shopt -u nullglob
  if (("${#backups[@]}" == 0)); then
    return 0
  fi
  # Backup names use sortable YYYYMMDD-HHMMSS suffixes.
  local latest="${backups[-1]}"
  mkdir -p "$(dirname -- "$destination")"
  cp -a "$latest" "$destination"
  echo "Restored pre-existing file: $destination"
}

rm -f "$USER_BIN/mrx-wm" "$USER_BIN/mrxctl" "$USER_BIN/mrx-session" "$USER_BIN/mrx-wm-supervisor" "$USER_BIN/mrx-diagnose" "$SESSION_DIR/mrx.desktop"
restore_latest_backup "$USER_BIN/mrx-wm"
restore_latest_backup "$USER_BIN/mrxctl"
restore_latest_backup "$USER_BIN/mrx-session"
restore_latest_backup "$USER_BIN/mrx-wm-supervisor"
restore_latest_backup "$USER_BIN/mrx-diagnose"
restore_latest_backup "$SESSION_DIR/mrx.desktop"

# Preserve the current MRX shell tree for recovery, then restore the user's
# pre-install tree when one was backed up by install.sh.
if [[ -d "$QS_CONFIG" ]]; then mv "$QS_CONFIG" "${QS_CONFIG}.removed.$(date +%Y%m%d-%H%M%S)"; fi
restore_latest_backup "$QS_CONFIG"

# The installer never overwrites settings.toml or picom.conf. Keep this
# user-editable configuration directory in place on uninstall.
if [[ -d "$USER_CONFIG" ]]; then
  echo "Preserved user configuration: $USER_CONFIG"
fi

echo "If you manually registered MRX system-wide, remove /usr/share/xsessions/mrx.desktop yourself with sudo."
echo "MRX files removed; pre-existing files restored where backups exist. System-wide sessions and other desktops were not changed."
