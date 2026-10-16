// Build everything this machine can and collect it in mmr/releases with
// consistent names:
//   MMRNext-<version>-<os>-<arch>-setup.exe / .msi / .dmg / .AppImage / .deb / .rpm
//   MMRNext-<version>-windows-x64-portable.exe
//   MMRNext-<version>-web.zip          (the browser version)
//   mmr-cli-<version>-<os>-<arch>[.exe]
// Each OS builds its own apps (no cross-compiling); GitHub's Release
// workflow builds all three.
//   node scripts/release.mjs [--universal]   (--universal: one macOS app for Intel + Apple Silicon)

import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const app = join(dirname(fileURLToPath(import.meta.url)), "..");
const root = join(app, "..");
const out = join(root, "releases");
const version = JSON.parse(readFileSync(join(app, "package.json"), "utf8")).version;
const universal = process.argv.includes("--universal");
const os = { win32: "windows", darwin: "macos", linux: "linux" }[process.platform] ?? process.platform;
const arch = universal ? "universal" : { x64: "x64", arm64: "arm64" }[process.arch] ?? process.arch;
const exe = process.platform === "win32" ? ".exe" : "";
const name = (rest) => `MMRNext-${version}-${rest}`;

// No shell: paths here may have spaces and quotes in them.
const run = (cmd, args, cwd = root) => execFileSync(cmd, args, { cwd, stdio: "inherit" });
const node = (script, args) => run(process.execPath, [join(app, script), ...args], app);

// 1. The desktop app and its installers.
node("node_modules/@tauri-apps/cli/tauri.js", ["build", ...(universal ? ["--target", "universal-apple-darwin"] : [])]);
const target = join(root, "target", universal ? "universal-apple-darwin/release" : "release");
const bundle = join(target, "bundle");

rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
const take = (dir, ext, as) => {
  const d = join(bundle, dir);
  if (!existsSync(d)) return;
  for (const f of readdirSync(d).filter((f) => f.endsWith(ext))) {
    copyFileSync(join(d, f), join(out, as));
    console.log(`  ${as}`);
  }
};
console.log(`\nCollecting into ${out}:`);
take("nsis", "-setup.exe", name(`${os}-${arch}-setup.exe`));
take("msi", ".msi", name(`${os}-${arch}.msi`));
take("dmg", ".dmg", name(`${os}-${arch}.dmg`));
take("appimage", ".AppImage", name(`${os}-${arch}.AppImage`));
take("deb", ".deb", name(`${os}-${arch}.deb`));
take("rpm", ".rpm", name(`${os}-${arch}.rpm`));
if (os === "windows") {
  copyFileSync(join(target, "MMRNext.exe"), join(out, name(`windows-${arch}-portable.exe`)));
  console.log(`  ${name(`windows-${arch}-portable.exe`)}`);
}

// 2. The command-line tool.
run("cargo", ["build", "--release", "-p", "mmr-cli"]);
const cli = `mmr-cli-${version}-${os}-${process.arch === "arm64" ? "arm64" : "x64"}${exe}`;
copyFileSync(join(root, "target/release", `mmr${exe}`), join(out, cli));
console.log(`  ${cli}`);

// 3. The browser version (after the app: it rebuilds app/dist).
node("scripts/build-web.mjs", []);
const zip = join(out, name("web.zip"));
if (process.platform === "win32") {
  // Windows' own tar (bsdtar), not Compress-Archive: Windows PowerShell
  // writes "\" in zip paths, which breaks unzipping on macOS and Linux.
  rmSync(zip, { force: true });
  run(join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe"), ["-a", "-c", "-f", zip, ...readdirSync(join(app, "dist"))], join(app, "dist"));
} else {
  run("zip", ["-qr", zip, "."], join(app, "dist"));
}
console.log(`  ${name("web.zip")}\n\nDone: ${out}`);
