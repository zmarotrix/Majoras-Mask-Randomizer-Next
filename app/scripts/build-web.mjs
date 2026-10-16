// The browser build: the randomizer compiled to wasm, the data the UI loads
// at startup, then the page itself (app/dist, static files: host anywhere).
//   npm run build:web            (needs: rustup target add wasm32-unknown-unknown)
//   npm run build:web -- --wasm-only   (just refresh public/mmr.wasm for npm run dev)

import { execFileSync } from "node:child_process";
import { copyFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const app = join(dirname(fileURLToPath(import.meta.url)), "..");
const root = join(app, "..");
// No shell: paths here may have spaces and quotes in them.
const run = (cmd, args, cwd = root) => execFileSync(cmd, args, { cwd, stdio: "inherit" });

run("cargo", ["build", "--release", "--target", "wasm32-unknown-unknown", "-p", "mmr-web"]);
copyFileSync(join(root, "target/wasm32-unknown-unknown/release/mmr_web.wasm"), join(app, "public/mmr.wasm"));
run("cargo", ["run", "--release", "-q", "-p", "mmr-cli", "--", "ui-data", "-o", join(app, "public/preview-data.json")]);
if (!process.argv.includes("--wasm-only")) run(process.execPath, [join(app, "node_modules/vite/bin/vite.js"), "build"], app);
