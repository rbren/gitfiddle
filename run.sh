#!/bin/sh
# bitfiddle launcher (macOS first; portable to Linux/Windows).
#
# Installs or builds repository-local dependencies when needed, builds the
# frontend, and launches the Tauri app. No undocumented manual setup.
set -e

cd "$(dirname "$0")"

command -v cargo >/dev/null 2>&1 || {
  echo "error: Rust toolchain not found. Install from https://rustup.rs" >&2
  exit 1
}
command -v npm >/dev/null 2>&1 || {
  echo "error: Node.js/npm not found. Install from https://nodejs.org" >&2
  exit 1
}

# Frontend dependencies
if [ ! -d frontend/node_modules ]; then
  echo "==> Installing frontend dependencies"
  (cd frontend && npm install)
fi

echo "==> Building frontend"
(cd frontend && npm run build)

# On Linux, Tauri needs a display server to open a window.
if [ "$(uname -s)" = "Linux" ] && [ -z "${DISPLAY:-}" ] && [ -z "${WAYLAND_DISPLAY:-}" ]; then
  echo "error: no display server found (DISPLAY/WAYLAND_DISPLAY unset)." >&2
  echo "bitfiddle is a desktop app and cannot launch headless." >&2
  echo "Build artifacts are still available; run 'cargo test' to verify the engine." >&2
  exit 1
fi

echo "==> Launching bitfiddle"
# The Tauri CLI resolves tauri.conf.json from the app crate directory; the
# CLI binary itself is a repository-local npm dev dependency of the frontend.
(cd crates/bitfiddle-app && exec ../../frontend/node_modules/.bin/tauri dev)
