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

# Tauri CLI (repository-local via npm)
echo "==> Building frontend"
(cd frontend && npm run build)

echo "==> Launching bitfiddle"
(cd frontend && npx tauri dev)
