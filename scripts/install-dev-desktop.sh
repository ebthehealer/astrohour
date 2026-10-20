#!/usr/bin/env bash
# Install the AstroHour .desktop entry + hicolor icons into the user's
# local XDG data dirs, so KDE/GNOME can resolve the taskbar icon by app_id.
#
# Why this is needed: on Wayland the compositor does NOT read the window's
# embedded icon. It matches the window's app_id against an installed
# .desktop file and uses that file's Icon= entry. Without this, the app
# shows a generic placeholder in the taskbar/dock.
#
# Run after `cargo tauri dev` has started at least once, or any time the
# icon artwork changes. Re-run is idempotent.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
app_id="com.astrohour.app"
icons="$repo/src-tauri/icons"

apps_dir="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
hicolor="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor"

mkdir -p "$apps_dir"

# .desktop entry
install -m 644 "$repo/packaging/$app_id.desktop" "$apps_dir/$app_id.desktop"

# Icons into the hicolor theme under the app_id name
declare -A map=(
  [32x32.png]=32x32
  [64x64.png]=64x64
  [128x128.png]=128x128
  [128x128@2x.png]=256x256
  [icon.png]=512x512
)
for src in "${!map[@]}"; do
  size="${map[$src]}"
  [ -f "$icons/$src" ] || continue
  mkdir -p "$hicolor/$size/apps"
  install -m 644 "$icons/$src" "$hicolor/$size/apps/$app_id.png"
done

# Refresh caches so the change is picked up without a re-login
command -v gtk-update-icon-cache >/dev/null && \
  gtk-update-icon-cache -f -t "$hicolor" >/dev/null 2>&1 || true
command -v update-desktop-database >/dev/null && \
  update-desktop-database "$apps_dir" >/dev/null 2>&1 || true
command -v kbuildsycoca6 >/dev/null && kbuildsycoca6 >/dev/null 2>&1 || true
command -v kbuildsycoca5 >/dev/null && kbuildsycoca5 >/dev/null 2>&1 || true

echo "Installed:"
echo "  $apps_dir/$app_id.desktop"
for size in 32x32 64x64 128x128 256x256 512x512; do
  [ -f "$hicolor/$size/apps/$app_id.png" ] && echo "  $hicolor/$size/apps/$app_id.png"
done
echo
echo "Restart AstroHour (cargo tauri dev) to pick up the new app_id."
