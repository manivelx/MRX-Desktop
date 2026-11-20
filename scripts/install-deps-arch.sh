#!/usr/bin/env bash
# Install MRX's Arch/CachyOS build and runtime dependencies.
# Deliberately does NOT install, remove, rebuild, or reconfigure graphics drivers.
set -euo pipefail

if ! command -v pacman >/dev/null 2>&1; then
  echo "This dependency installer supports Arch Linux and CachyOS only." >&2
  exit 1
fi

packages=(
  # Rust/C/C++ build toolchain
  base-devel rust cargo git clang cmake ninja pkgconf uv python
  # X11 server, diagnostics and isolated test servers
  xorg-server xorg-server-xephyr xorg-server-xvfb xorg-xrandr xorg-xprop
  xorg-xwininfo xorg-xinput xorg-xdpyinfo xorg-xset xorg-xsetroot
  xdotool xsettingsd xterm xorg-xmessage xorg-fonts-misc
  # Quickshell / Qt QML frontend (X11/XCB)
  quickshell qt6-base qt6-declarative qt6-svg gtk4 libadwaita
  libsoup3 libportal-gtk4 gobject-introspection cairo dbus dconf pkgconf
  # Compositing and session utilities
  picom xss-lock libnotify xdg-utils xdg-user-dirs xdg-desktop-portal
  xdg-desktop-portal-gtk gammastep swappy
  # Desktop integrations used by end4-pC modules
  pipewire pipewire-pulse wireplumber playerctl cava pavucontrol-qt
  libdbusmenu-gtk3 networkmanager bluez bluez-utils upower brightnessctl
  ddcutil geoclue gnome-keyring polkit-gnome
  # End4 shell helper tools, file manager, media and image utilities
  jq imagemagick ffmpeg matugen feh cliphist maim xclip slop
  tesseract tesseract-data-eng fuzzel translate-shell libqalculate
  dolphin thunar eza fish kitty starship coreutils curl wget ripgrep rsync
  go-yq
  # Fonts used by the supplied shell theme
  fontconfig otf-space-grotesk ttf-jetbrains-mono-nerd
  ttf-readex-pro ttf-rubik-vf ttf-twemoji
  # X11 and software rendering diagnostics
  pciutils kmod mesa-utils
)

printf 'Installing MRX build/runtime dependencies with pacman...\n'
printf 'NVIDIA drivers, kernel packages, bootloader and existing desktop sessions will NOT be changed.\n'
sudo pacman -S --needed "${packages[@]}"

cat <<'MSG'

Dependencies installed.
Next steps:
  1. Run ./check-deps.sh
  2. Run ./build.sh
  3. Run ./tests/xvfb-smoke.sh

Quickshell must provide its X11/XCB backend. The installer does not replace
Quickshell or your NVIDIA driver; it uses the versions already supplied by
Arch/CachyOS repositories.
MSG
