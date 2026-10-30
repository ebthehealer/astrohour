#!/usr/bin/env bash
# Build the Linux package for this machine and install it, so the entry in the
# application menu runs the code you just pulled.
#
# `cargo tauri dev` does NOT touch the installed copy — it runs a debug binary
# from target/debug and never goes near /usr/bin. This script is what updates
# the app launcher.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

# Version of an installed package, or "none".
#
# Two traps here. rpm writes "package X is not installed" to *stdout*, so the
# result cannot simply be captured and filtered. And `--quiet` is not the fix
# either: it suppresses the queryformat output as well as the message, so an
# installed package yields an empty string.
installed_version() {
  if rpm -q "$1" >/dev/null 2>&1; then
    rpm -q --qf '%{VERSION}-%{RELEASE}' "$1"
  else
    echo none
  fi
}

# The package version does not change between rebuilds -- 0.3.0-1 is what every
# build produces until someone bumps the version -- so it cannot tell you
# whether the code you are about to install is the code you just pulled. Report
# the commit instead, and say so when the checkout is behind the remote. That
# way "it still looks the same" is a question with an answer.
COMMIT=$(git rev-parse --short HEAD)
BRANCH=$(git rev-parse --abbrev-ref HEAD)
echo "==> Building $COMMIT on $BRANCH"

if git rev-parse --abbrev-ref "@{upstream}" >/dev/null 2>&1; then
  if ! git fetch --quiet 2>/dev/null; then
    echo "    (could not reach the remote; building what is here)"
  elif [ -n "$(git log HEAD..@{upstream} --oneline 2>/dev/null)" ]; then
    echo
    echo "!! This checkout is BEHIND its remote. You are about to install"
    echo "   $COMMIT, not the latest code. Run 'git pull' first if that is"
    echo "   not what you want:"
    git log HEAD..@{upstream} --oneline | sed 's/^/     /'
    echo
  else
    echo "    up to date with $BRANCH"
  fi
fi

# Tauri compiles the frontend (build/) into the Rust binary. Cargo does not
# always notice that build/ changed, and then happily relinks the OLD UI into
# the new package. Forcing the app crates to recompile makes that impossible.
echo "==> Discarding cached app build so the current UI is embedded"
rm -rf build
cargo clean --release -p astrohour -p astrohour_lib 2>/dev/null || cargo clean --release -p astrohour

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
WANTED=$(rpm -qp --qf '%{VERSION}-%{RELEASE}' "$RPM")
HAVE=$(installed_version astro-hour)

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
AFTER=$(installed_version astro-hour)
if [ "$AFTER" != "$WANTED" ]; then
  echo
  echo "!! Install did not take: wanted $WANTED, have $AFTER" >&2
  echo "   Try installing the package by hand:" >&2
  echo "     sudo dnf install -y $RPM" >&2
  exit 1
fi

echo
echo "==> Installed: astro-hour-$AFTER (built from $COMMIT)"
echo "    Now quit any running AstroHour (tray icon → Quit) and"
echo "    relaunch it from the application menu. A process already running"
echo "    keeps executing the old binary until it is restarted."
