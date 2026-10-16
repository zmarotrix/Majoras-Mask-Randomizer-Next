#!/usr/bin/env sh
# Build the browser version of MMR Next and open it (macOS / Linux).
# The randomizer runs entirely in the browser; your ROM never leaves it.
set -e
cd "$(dirname "$0")/app"
command -v npm >/dev/null || { echo "Node.js (npm) was not found. Install it from https://nodejs.org and try again."; exit 1; }
command -v cargo >/dev/null || { echo "Rust (cargo) was not found. Install it from https://rustup.rs and try again."; exit 1; }
rustup target list --installed | grep -qx wasm32-unknown-unknown || rustup target add wasm32-unknown-unknown
[ -d node_modules ] || npm install
npm run build:web
echo
echo "MMR Next is running at http://localhost:4173 (Ctrl+C to stop)"
npx vite preview --port 4173 --open
