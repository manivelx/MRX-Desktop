#!/usr/bin/env bash
# Return a comma-separated X server font path for Ubuntu and Arch test runners.
mrx_test_font_path() {
    local path
    local -a font_dirs=()
    for path in \
        /usr/share/fonts/misc \
        /usr/share/fonts/75dpi \
        /usr/share/fonts/100dpi \
        /usr/share/fonts/X11/misc \
        /usr/share/fonts/X11/75dpi \
        /usr/share/fonts/X11/100dpi; do
        [[ -d "$path" ]] && font_dirs+=("$path")
    done
    if (("${#font_dirs[@]}" > 0)); then
        local IFS=,
        printf '%s,built-ins\n' "${font_dirs[*]}"
    else
        printf 'built-ins\n'
    fi
}
