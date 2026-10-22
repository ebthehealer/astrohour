#!/usr/bin/env bash
# Build the Linux package for this machine and install it, so the entry in the
# application menu runs the code you just pulled.
#
# `cargo tauri dev` does NOT touch the installed copy — it runs a debug binary
# from target/debug and never goes near /usr/bin. This script is what updates
# the app launcher.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

echo "==> Building release packages (this takes a few minutes)…"
cargo tauri build --bundles rpm

RPM=$(ls -t target/release/bundle/rpm/*.rpm | head -1)
echo "==> Installing $RPM"

# reinstall when already present, otherwise a same-version install is a no-op.
if rpm -q astro-hour >/dev/null 2>&1; then
  sudo dnf reinstall -y "$RPM"
else
  sudo dnf install -y "$RPM"
fi

# KDE caches the application menu; nudge it to re-read the desktop entry.
if command -v kbuildsycoca6 >/dev/null 2>&1; then
  kbuildsycoca6 --noincremental >/dev/null 2>&1 || true
fi

echo
echo "==> Installed. Now quit any running AstroHour (tray icon → Quit) and"
echo "    relaunch it from the application menu. A process already running"
echo "    keeps executing the old binary until it is restarted."
