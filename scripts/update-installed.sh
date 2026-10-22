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

# dnf5's `reinstall` only resolves package *names* from repositories — handed a
# local .rpm path it reports "available, but not installed" and exits 0 having
# done nothing. So pick the command by comparing versions instead:
#
#   fresh install or upgrade -> dnf install (resolves dependencies, upgrades)
#   same version already there -> rpm -U, because dnf calls that "nothing to do"
#     and would leave the old files in place after a rebuild without a bump.
# --quiet matters: without it rpm writes "package astro-hour is not installed"
# to *stdout*, which would land in $HAVE alongside the fallback.
WANTED=$(rpm -qp --qf '%{VERSION}-%{RELEASE}' "$RPM")
HAVE=$(rpm -q --quiet --qf '%{VERSION}-%{RELEASE}' astro-hour 2>/dev/null || echo none)

if [ "$HAVE" = "$WANTED" ]; then
  echo "==> Already at $WANTED; replacing files"
  sudo rpm -Uvh --replacepkgs --replacefiles "$RPM"
else
  sudo dnf install -y "$RPM"
fi

# KDE caches the application menu; nudge it to re-read the desktop entry.
if command -v kbuildsycoca6 >/dev/null 2>&1; then
  kbuildsycoca6 --noincremental >/dev/null 2>&1 || true
fi

# Confirm the install actually took. Without this the script reports success
# after a dnf invocation that quietly decided there was nothing to do.
AFTER=$(rpm -q --quiet --qf '%{VERSION}-%{RELEASE}' astro-hour 2>/dev/null || echo none)
if [ "$AFTER" != "$WANTED" ]; then
  echo
  echo "!! Install did not take: wanted $WANTED, have $AFTER" >&2
  echo "   Try installing the package by hand:" >&2
  echo "     sudo dnf install -y $RPM" >&2
  exit 1
fi

echo
echo "==> Installed: astro-hour-$AFTER"
echo "    Now quit any running AstroHour (tray icon → Quit) and"
echo "    relaunch it from the application menu. A process already running"
echo "    keeps executing the old binary until it is restarted."
