#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

mkdir -p "$TMP/bin" "$TMP/mock" "$TMP/home/.config/mrx" "$TMP/home/.config/quickshell/mrx" "$TMP/home/.local/share/xsessions" "$TMP/home/.local/bin"
printf '#!/bin/sh\nexit 0\n' > "$TMP/mock/qs"
chmod +x "$TMP/mock/qs"
for name in mrx-wm mrxctl; do
  printf 'new-%s\n' "$name" > "$TMP/bin/$name"
  chmod +x "$TMP/bin/$name"
done
printf 'user-picom-config\n' > "$TMP/home/.config/mrx/picom.conf"
printf 'user-settings\n' > "$TMP/home/.config/mrx/settings.toml"
printf 'user-shell-file\n' > "$TMP/home/.config/quickshell/mrx/user.qml"
printf 'pre-existing-session\n' > "$TMP/home/.local/share/xsessions/mrx.desktop"
printf 'pre-existing-wm\n' > "$TMP/home/.local/bin/mrx-wm"
printf 'pre-existing-diagnose\n' > "$TMP/home/.local/bin/mrx-diagnose"
chmod +x "$TMP/home/.local/bin/mrx-wm" "$TMP/home/.local/bin/mrx-diagnose"

export HOME="$TMP/home"
export XDG_CONFIG_HOME="$HOME/.config"
export XDG_DATA_HOME="$HOME/.local/share"
export MRX_BIN_DIR="$TMP/bin"
export PATH="$TMP/mock:$PATH"

bash "$ROOT/install.sh"
[[ "$(cat "$HOME/.config/mrx/picom.conf")" == user-picom-config ]]
[[ "$(cat "$HOME/.config/mrx/settings.toml")" == user-settings ]]
[[ "$(cat "$HOME/.local/share/xsessions/mrx.desktop")" == "$(cat "$ROOT/session/mrx.desktop")" ]]
[[ -x "$HOME/.local/bin/mrx-wm" && -x "$HOME/.local/bin/mrxctl" && -x "$HOME/.local/bin/mrx-wm-supervisor" && -x "$HOME/.local/bin/mrx-diagnose" ]]
[[ "$(cat "$HOME/.local/bin/mrx-wm")" == new-mrx-wm ]]
[[ "$(cat "$HOME/.local/bin/mrx-diagnose" | head -n1)" == '#!/usr/bin/env bash' ]]
compgen -G "$HOME/.local/bin/mrx-wm.backup.*" >/dev/null
compgen -G "$HOME/.local/bin/mrx-diagnose.backup.*" >/dev/null
[[ -f "$HOME/.config/quickshell/mrx/user.qml" ]]
compgen -G "$HOME/.local/share/xsessions/mrx.desktop.backup.*" >/dev/null

# Reinstall must keep user settings and Picom config and retain a shell backup.
bash "$ROOT/install.sh"
[[ "$(cat "$HOME/.config/mrx/picom.conf")" == user-picom-config ]]
[[ "$(cat "$HOME/.config/mrx/settings.toml")" == user-settings ]]
compgen -G "$HOME/.config/quickshell/mrx.backup.*" >/dev/null

bash "$ROOT/uninstall.sh"
[[ "$(cat "$HOME/.local/bin/mrx-wm")" == pre-existing-wm ]]
[[ "$(cat "$HOME/.local/bin/mrx-diagnose")" == pre-existing-diagnose ]]
[[ ! -e "$HOME/.local/bin/mrxctl" && ! -e "$HOME/.local/bin/mrx-wm-supervisor" ]]
[[ "$(cat "$HOME/.local/share/xsessions/mrx.desktop")" == pre-existing-session ]]
[[ "$(cat "$HOME/.config/quickshell/mrx/user.qml")" == user-shell-file ]]
[[ "$(cat "$HOME/.config/mrx/settings.toml")" == user-settings ]]
[[ "$(cat "$HOME/.config/mrx/picom.conf")" == user-picom-config ]]
compgen -G "$HOME/.config/quickshell/mrx.removed.*" >/dev/null
[[ -d "$HOME/.config/mrx" ]]
! compgen -G "$HOME/.config/mrx.removed.*" >/dev/null
printf 'PASS: isolated install/reinstall, rollback restoration and preservation of user settings\n'
