#!/usr/bin/env bash
# Build Linux packages: .deb, .rpm, .AppImage
# Run from the repo root. Output lands in src-tauri/target/release/bundle/.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

echo "==> Building Linux packages…"
cargo tauri build --bundles deb,rpm,appimage

echo
echo "==> Output:"
find src-tauri/target/release/bundle -name "*.deb" -o -name "*.rpm" -o -name "*.AppImage" \
  | sort | while read -r f; do
    echo "  $(du -sh "$f" | cut -f1)  $f"
  done
