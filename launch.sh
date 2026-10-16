#!/usr/bin/env sh
# Launch the MMR Next desktop app in development mode (macOS / Linux).
# The first launch after code changes takes a minute to compile.
set -e
cd "$(dirname "$0")/app"
command -v npm >/dev/null || { echo "Node.js (npm) was not found. Install it from https://nodejs.org and try again."; exit 1; }
[ -d node_modules ] || npm install
npm run tauri dev
