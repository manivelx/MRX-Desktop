#!/usr/bin/env bash
# Shared compositor/session helpers for shell scripts.
# MRX is an X11 session backed by the Rust WM and mrxctl IPC client.

mrx_detect_compositor() {
    local combined
    combined="$(printf '%s %s' "${XDG_CURRENT_DESKTOP:-}" "${XDG_SESSION_DESKTOP:-}" | tr '[:upper:]' '[:lower:]')"
    case "$combined" in
        *mrx*) printf 'mrx\n' ;;
        *bspwm*) printf 'bspwm\n' ;;
        *niri*) printf 'niri\n' ;;
        *hyprland*) printf 'hyprland\n' ;;
        *sway*) printf 'sway\n' ;;
        *) printf 'unknown\n' ;;
    esac
}

mrx_is_x11_session() {
    case "$1" in
        bspwm|mrx) return 0 ;;
        *) return 1 ;;
    esac
}

mrx_get_display_geometry() {
    if command -v xdotool >/dev/null 2>&1; then
        xdotool getdisplaygeometry
    elif command -v xrandr >/dev/null 2>&1; then
        xrandr --current | sed -nE 's/^Screen [0-9]+: .*current ([0-9]+) x ([0-9]+).*/\1 \2/p' | head -n1
    else
        return 1
    fi
}

mrx_get_monitor_names() {
    case "$(mrx_detect_compositor)" in
        mrx) mrxctl snapshot | jq -r '.monitors[].name' ;;
        bspwm) bspc query -M --names ;;
        niri) niri msg -j outputs | jq -r 'keys[]' ;;
        *) hyprctl monitors -j | jq -r '.[].name' ;;
    esac
}

mrx_get_active_monitor() {
    case "$(mrx_detect_compositor)" in
        mrx) mrxctl snapshot | jq -r '.monitors[] | select(.focused == true) | .name' ;;
        bspwm) bspc query -M -m focused --names ;;
        niri) niri msg -j workspaces | jq -r '.[] | select(.is_focused == true) | .output' ;;
        *) hyprctl monitors -j | jq -r '.[] | select(.focused == true) | .name' ;;
    esac
}

# Output: scale, x, y, width, height for the focused monitor.
mrx_get_focused_monitor_geometry() {
    case "$(mrx_detect_compositor)" in
        mrx)
            mrxctl snapshot | jq -r '.monitors[] | select(.focused == true) | [(.scale // 1), .x, .y, .width, .height] | @tsv'
            ;;
        bspwm)
            local width height
            read -r width height < <(mrx_get_display_geometry) || return 1
            printf '1\t0\t0\t%s\t%s\n' "$width" "$height"
            ;;
        *)
            hyprctl monitors -j | jq -r '.[] | select(.focused == true) | [.scale, .x, .y, .width, .height] | @tsv'
            ;;
    esac
}
