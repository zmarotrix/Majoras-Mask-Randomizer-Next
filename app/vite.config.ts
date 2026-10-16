import { defineConfig, type Plugin } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { rmSync } from "node:fs";
import { join } from "node:path";

// The desktop app has the randomizer built in: leave out the browser-only
// files (public/mmr.wasm, public/preview-data.json) when Tauri builds it.
function desktopOnly(): Plugin {
  let outDir = "dist";
  return {
    name: "mmr-desktop-only",
    apply: "build",
    configResolved(c) {
      outDir = c.build.outDir;
    },
    closeBundle() {
      if (!process.env.TAURI_ENV_PLATFORM) return;
      for (const f of ["mmr.wasm", "preview-data.json"]) rmSync(join(outDir, f), { force: true });
    },
  };
}

// Tauri expects a fixed port and doesn't want the screen cleared.
export default defineConfig({
  plugins: [svelte(), desktopOnly()],
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: { target: "es2022", sourcemap: false },
});
