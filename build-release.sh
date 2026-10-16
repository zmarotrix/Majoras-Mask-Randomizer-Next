#!/usr/bin/env sh
# Build MMR Next for release on this machine (macOS / Linux). Everything ends
# up in the "releases" folder: the app's packages, browser version and CLI.
#   macOS: .dmg (pass --universal for one app for Intel + Apple Silicon)
#   Linux: .AppImage, .deb and .rpm
# Linux needs WebKitGTK first, e.g. on Debian/Ubuntu:
#   sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
set -e
cd "$(dirname "$0")/app"
command -v npm >/dev/null || { echo "Node.js (npm) was not found. Install it from https://nodejs.org and try again."; exit 1; }
command -v cargo >/dev/null || { echo "Rust (cargo) was not found. Install it from https://rustup.rs and try again."; exit 1; }
rustup target list --installed | grep -qx wasm32-unknown-unknown || rustup target add wasm32-unknown-unknown
if [ "$1" = "--universal" ]; then rustup target add aarch64-apple-darwin x86_64-apple-darwin; fi
[ -d node_modules ] || npm install
node scripts/release.mjs "$@"
