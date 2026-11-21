#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/bin"
for binary in mrx-wm mrxctl; do
  printf '#!/bin/sh\nexit 0\n' > "$TMP/bin/$binary"
  chmod 755 "$TMP/bin/$binary"
done

export MRX_BIN_DIR="$TMP/bin"
export MRX_RELEASE_OUT_DIR="$TMP/dist"
export SOURCE_DATE_EPOCH=1700000000
bash "$ROOT/scripts/package-release.sh"
ARCHIVE="$(find "$MRX_RELEASE_OUT_DIR" -maxdepth 1 -name '*.tar.gz' -print -quit)"
[[ -n "$ARCHIVE" && -s "$ARCHIVE" ]]
(
  cd "$MRX_RELEASE_OUT_DIR"
  sha256sum -c "$(basename "$ARCHIVE").sha256"
)
tar -tzf "$ARCHIVE" > "$TMP/files.txt"
for expected in '/bin/mrx-wm' '/bin/mrxctl' '/install.sh' '/uninstall.sh' '/session/mrx-session' '/session/mrx-wm-supervisor' '/shell/' '/LICENSE' '/THIRD_PARTY_NOTICES.md' '/docs/VALIDATION.md'; do
  grep -Fq "$expected" "$TMP/files.txt" || { echo "FAIL: release archive omits $expected" >&2; exit 1; }
done
FIRST_SUM="$(sha256sum "$ARCHIVE" | cut -d' ' -f1)"
bash "$ROOT/scripts/package-release.sh"
SECOND_SUM="$(sha256sum "$ARCHIVE" | cut -d' ' -f1)"
[[ "$FIRST_SUM" == "$SECOND_SUM" ]] || { echo "FAIL: release archive is not reproducible" >&2; exit 1; }
echo "PASS: release archive contents, SHA-256 verification and reproducible packaging"
