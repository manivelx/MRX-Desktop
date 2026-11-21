#!/usr/bin/env bash
# Collect privacy-conscious MRX/X11 startup diagnostics without elevated access.
set -uo pipefail

OUTPUT=""
REASON=""
while (($#)); do
  case "$1" in
    --output) OUTPUT="${2:?--output requires a path}"; shift 2 ;;
    --reason) REASON="${2:?--reason requires a value}"; shift 2 ;;
    --help|-h)
      echo "Usage: mrx-diagnose [--reason TEXT] [--output PATH]"
      exit 0
      ;;
    *) echo "Unknown option: $1" >&2; exit 2 ;;
  esac
done

if [[ -n "$OUTPUT" ]]; then
  mkdir -p "$(dirname -- "$OUTPUT")"
  umask 077
  : > "$OUTPUT"
  chmod 600 "$OUTPUT"
  exec > "$OUTPUT" 2>&1
fi

section() { printf '\n## %s\n' "$1"; }
available() {
  if command -v "$1" >/dev/null 2>&1; then
    printf '%-18s %s\n' "$1" "$(command -v "$1")"
  else
    printf '%-18s unavailable\n' "$1"
  fi
}

printf 'MRX startup diagnostics\n'
printf 'Generated (UTC): %s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
printf 'Reason: %s\n' "${REASON:-manual diagnostic capture}"
printf 'Kernel: %s\n' "$(uname -srmo 2>/dev/null || echo unavailable)"
if [[ -r /etc/os-release ]]; then
  . /etc/os-release
  printf 'OS: %s %s\n' "${PRETTY_NAME:-unknown}" "${VERSION_ID:-}"
fi

section "Session environment"
for name in XDG_SESSION_TYPE XDG_CURRENT_DESKTOP XDG_SESSION_DESKTOP DISPLAY XDG_RUNTIME_DIR; do
  printf '%s=%s\n' "$name" "${!name:-unset}"
done

section "Required and optional commands"
for name in mrx-wm mrxctl qs picom xrandr xss-lock notify-send jq; do
  available "$name"
done

CTL="${MRX_CTL_BIN:-$(command -v mrxctl 2>/dev/null || true)}"
if [[ -n "$CTL" && -x "$CTL" ]]; then
  section "MRX IPC health"
  if output="$("$CTL" ping 2>&1)"; then
    printf 'MRX IPC: %s\n' "$output"
  else
    printf 'MRX IPC: unavailable (%s)\n' "$output"
  fi
  if output="$("$CTL" health 2>&1)"; then
    printf 'MRX health response:\n%s\n' "$output"
  else
    printf 'MRX health: unavailable (%s)\n' "$output"
  fi
  if snapshot="$("$CTL" snapshot 2>&1)"; then
    if command -v jq >/dev/null 2>&1 && printf '%s' "$snapshot" | jq -e . >/dev/null 2>&1; then
      printf '%s\n' "$snapshot" | jq -r '
        "MRX monitors: " + ([.monitors[]?.name] | join(", ")) +
        "\nMRX workspaces: " + ([.workspaces[]? | ((.monitor // "?") + ":" + (.id|tostring) + (if .active then "*" else "" end))] | join(", ")) +
        "\nMRX managed windows: " + ((.windows // []) | length | tostring)
      '
    else
      printf 'MRX snapshot: received but could not summarize JSON\n'
    fi
  else
    printf 'MRX snapshot: unavailable (%s)\n' "$snapshot"
  fi
else
  section "MRX IPC health"
  echo "mrxctl is not installed or not executable."
fi

section "X11 monitor topology"
if command -v xrandr >/dev/null 2>&1; then
  xrandr --listmonitors 2>&1 || echo "xrandr could not query the active display."
else
  echo "xrandr is not installed."
fi

section "MRX-related process names"
if command -v ps >/dev/null 2>&1; then
  ps -u "${USER:-$(id -un)}" -o pid=,comm= 2>/dev/null |
    awk '$2 ~ /^(mrx-wm|mrx-wm-supervisor|mrxctl|qs|picom|xss-lock)$/ { print }' ||
    true
fi

section "MRX window-manager log (last 80 lines)"
CACHE_HOME="${XDG_CACHE_HOME:-$HOME/.cache}"
WM_LOG="${MRX_WM_LOG:-$CACHE_HOME/mrx/wm.log}"
if [[ -r "$WM_LOG" ]]; then
  # Replace the home path so reports can be shared without exposing the username.
  sed "s#${HOME}#~#g" "$WM_LOG" | tail -n 80
else
  printf 'No readable WM log at %s\n' "$WM_LOG"
fi

section "Privacy"
echo "Window titles, application names, environment secrets and file contents are not collected."
echo "This report does not use sudo or modify the session, drivers, kernel or bootloader."
if [[ -n "$OUTPUT" ]]; then
  printf '\nReport saved to: %s\n' "$OUTPUT"
fi
