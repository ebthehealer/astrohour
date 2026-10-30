#!/usr/bin/env bash
# Run the app in development mode, clearing up a previous session first.
#
# `cargo tauri dev` does not clean up after itself when it is killed rudely —
# Ctrl-C in one terminal, closing the window, or a crash leaves the Vite dev
# server holding port 1420 and a cargo process holding the build directory
# lock. The next run then fails with "Blocking waiting for file lock on build
# directory" followed by "Port 1420 is already in use".
#
# Note this kills only dev processes. It will not touch an installed copy
# running from /usr/bin, which is deliberate: that is the one you want to
# keep, and running both at once gives you two tray icons.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

leftovers=$(pgrep -f "tauri dev|target/debug/astrohour" || true)
if [ -n "$leftovers" ]; then
  echo "==> Stopping a previous dev session"
  pkill -f "tauri dev" || true
  pkill -f "target/debug/astrohour" || true
  sleep 1
fi

# The Vite server can outlive the parent that spawned it.
if command -v fuser >/dev/null 2>&1; then
  fuser -k 1420/tcp 2>/dev/null || true
fi

if pgrep -f "target/release/astrohour" >/dev/null 2>&1; then
  echo "!! An installed AstroHour is running. Quit it from the tray first,"
  echo "   or you will get two tray icons and duplicate notifications."
fi

exec cargo tauri dev
