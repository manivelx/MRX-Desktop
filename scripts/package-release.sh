#!/usr/bin/env bash
# Build a deterministic, user-installable Linux release archive.
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="${MRX_BIN_DIR:-$ROOT/target/release}"
OUT_DIR="${MRX_RELEASE_OUT_DIR:-$ROOT/dist}"

VERSION="$(awk '
  /^\[workspace\.package\]$/ { in_package=1; next }
  /^\[/ { in_package=0 }
  in_package && /^version[[:space:]]*=/ {
    sub(/^[^"]*"/, ""); sub(/".*$/, ""); print; exit
  }
' "$ROOT/Cargo.toml")"
[[ -n "$VERSION" ]] || { echo "Could not read workspace package version" >&2; exit 1; }

for binary in mrx-wm mrxctl; do
  [[ -x "$BIN_DIR/$binary" ]] || {
    echo "Missing release binary: $BIN_DIR/$binary (run cargo build --release --workspace)" >&2
    exit 1
  }
done

ARCH="$(uname -m)"
[[ "$ARCH" == "x86_64" || "$ARCH" == "aarch64" ]] || {
  echo "Unsupported release architecture: $ARCH" >&2
  exit 1
}
STAGE_NAME="mrx-desktop-$VERSION-linux-$ARCH"
mkdir -p "$OUT_DIR"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
STAGE="$TMP/$STAGE_NAME"
mkdir -p "$STAGE/bin" "$STAGE/config" "$STAGE/session" "$STAGE/scripts"

install -m755 "$BIN_DIR/mrx-wm" "$STAGE/bin/mrx-wm"
install -m755 "$BIN_DIR/mrxctl" "$STAGE/bin/mrxctl"
install -m755 "$ROOT/session/mrx-session" "$STAGE/session/mrx-session"
install -m755 "$ROOT/session/mrx-wm-supervisor" "$STAGE/session/mrx-wm-supervisor"
install -m644 "$ROOT/session/mrx.desktop" "$STAGE/session/mrx.desktop"
install -m644 "$ROOT/config/settings.toml" "$STAGE/config/settings.toml"
install -m644 "$ROOT/config/picom.conf" "$STAGE/config/picom.conf"
install -m755 "$ROOT/scripts/install-deps-arch.sh" "$STAGE/scripts/install-deps-arch.sh"
install -m755 "$ROOT/scripts/hardware-report.sh" "$STAGE/scripts/hardware-report.sh"
install -m755 "$ROOT/check-deps.sh" "$STAGE/check-deps.sh"
install -m755 "$ROOT/install.sh" "$STAGE/install.sh"
install -m755 "$ROOT/uninstall.sh" "$STAGE/uninstall.sh"
cp -a "$ROOT/shell" "$STAGE/shell"
cp "$ROOT/README.md" "$ROOT/LICENSE" "$ROOT/THIRD_PARTY_NOTICES.md" "$ROOT/CHANGELOG.md" "$ROOT/ROADMAP.md" "$STAGE/"
cp -a "$ROOT/docs" "$STAGE/docs"

cat > "$STAGE/INSTALL.txt" <<'EOF'
MRX Desktop binary release

1. Extract this archive.
2. Read README.md and docs/VALIDATION.md before installing.
3. Install the Arch/CachyOS runtime dependencies with:
     ./install.sh --install-deps
4. Install user-scoped files with:
     ./install.sh

The installer does not modify SDDM, the existing BSPWM session, the Orbital
theme, the existing Quickshell lock screen, the kernel, NVIDIA drivers or the
bootloader. MRX remains experimental; test in a disposable VM first.
EOF

SOURCE_EPOCH="${SOURCE_DATE_EPOCH:-}"
if [[ -z "$SOURCE_EPOCH" ]]; then
  SOURCE_EPOCH="$(git -C "$ROOT" log -1 --format=%ct 2>/dev/null || printf '0')"
fi
[[ "$SOURCE_EPOCH" =~ ^[0-9]+$ ]] || { echo "SOURCE_DATE_EPOCH must be an integer" >&2; exit 2; }

ARCHIVE="$OUT_DIR/$STAGE_NAME.tar.gz"
(
  cd "$TMP"
  tar --sort=name --mtime="@$SOURCE_EPOCH" --owner=0 --group=0 --numeric-owner -cf - "$STAGE_NAME" | gzip -n > "$ARCHIVE"
)
(
  cd "$OUT_DIR"
  sha256sum "$STAGE_NAME.tar.gz" > "$STAGE_NAME.tar.gz.sha256"
)
echo "PASS: created $ARCHIVE"
echo "PASS: checksum $ARCHIVE.sha256"
