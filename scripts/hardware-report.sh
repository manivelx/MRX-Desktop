#!/usr/bin/env bash
# Read-only hardware/driver report for support requests. Does not change the system.
set -u
section() { printf '\n===== %s =====\n' "$1"; }
section 'Kernel and distribution'
uname -a
if [[ -r /etc/os-release ]]; then . /etc/os-release; printf '%s %s\n' "${PRETTY_NAME:-Linux}" "${VERSION_ID:-}"; fi
section 'GPU and bound kernel driver'
if command -v lspci >/dev/null 2>&1; then lspci -nnk | grep -A4 -E 'VGA|3D|Display' || true; else echo 'lspci unavailable (install pciutils)'; fi
section 'NVIDIA module and version'
if command -v modinfo >/dev/null 2>&1; then modinfo nvidia 2>&1 | grep -E '^(version|filename|vermagic):' | head -10 || true; fi
cat /proc/driver/nvidia/version 2>/dev/null || true
if command -v nvidia-smi >/dev/null 2>&1; then nvidia-smi 2>&1 || true; else echo 'nvidia-smi unavailable'; fi
section 'Loaded graphics modules'
lsmod 2>/dev/null | grep -E '^(nvidia|nouveau|drm)' || true
section 'Kernel configuration and exported symbol'
KBUILD="/usr/lib/modules/$(uname -r)/build"
if [[ -r "$KBUILD/.config" ]]; then grep -E 'CONFIG_DRM_AUDIO_COMPONENT|CONFIG_DRM=' "$KBUILD/.config" || true; else echo "No readable kernel config at $KBUILD/.config"; fi
if [[ -r "/boot/System.map-$(uname -r)" ]]; then grep -w 'drm_audio_component' "/boot/System.map-$(uname -r)" | head || true; fi
section 'X11 and GLX'
printf 'DISPLAY=%s\n' "${DISPLAY:-<unset>}"
if [[ -n "${DISPLAY:-}" ]] && command -v xdpyinfo >/dev/null 2>&1; then xdpyinfo -queryExtensions 2>/dev/null | grep -Ei 'Composite|DAMAGE|RANDR|XFIXES|GLX|Present' || true; fi
if command -v glxinfo >/dev/null 2>&1; then glxinfo -B 2>&1 | head -35; else echo 'glxinfo unavailable (install mesa-utils)'; fi
section 'Quickshell and session'
command -v qs || true
qs --version 2>&1 | head -5 || true
printf 'QT_QPA_PLATFORM=%s\n' "${QT_QPA_PLATFORM:-<unset>}"
printf 'XDG_SESSION_TYPE=%s\n' "${XDG_SESSION_TYPE:-<unset>}"
section 'Recent graphics kernel messages'
journalctl -k -b --no-pager 2>/dev/null | grep -iE 'nvidia|nouveau|drm|unknown symbol' | tail -60 || true
