// The one interface between the UI and the randomizer core.
//
// The desktop app implements it with Tauri commands (app/src-tauri). In a
// browser it's the core compiled to wasm (crates/mmr-web, run in a worker),
// with browser file pickers and downloads instead of paths.

import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import type { Outputs } from "./state.svelte";
import type { Entry, Generated, Patched, GameData, Inactive, LogicFile, LogicIssue, Playthrough, Preset, RomCheck, Scenario, SettingsIssue, SettingsSchema, Values, OpenedRegions, RegionLogic } from "./types";

export interface FileFilter {
  name: string;
  extensions: string[];
}

export interface Backend {
  gameData(): Promise<GameData>;
  settingsSchema(): Promise<SettingsSchema>;
  presets(): Promise<Preset[]>;
  settingsCheck(values: Values): Promise<{ issues: SettingsIssue[]; inactive: Inactive[] }>;
  settingsEncode(values: Values, includeCosmetics: boolean): Promise<string>;
  settingsDecode(s: string): Promise<Values>;
  settingsOpen(path: string): Promise<{ values: Values; skipped: string[]; legacy: boolean }>;
  settingsSave(path: string, values: Values): Promise<void>;

  logicBundledList(): Promise<{ id: string; name: string }[]>;
  /** `source` is `bundled:<id>` or a file path. */
  logicOpen(source: string): Promise<{ file: LogicFile; format: "current" | "legacy" }>;
  /** The tricks of a bundled logic (`casual`, `glitched`). */
  logicTricks(id: string): Promise<Entry[]>;
  logicSave(path: string, file: LogicFile, legacy: boolean): Promise<void>;
  logicValidate(file: LogicFile): Promise<LogicIssue[]>;
  logicEvaluate(file: LogicFile, scenario: Scenario): Promise<string[]>;
  logicPlaythrough(file: LogicFile, scenario: Scenario): Promise<Playthrough>;

  /** Logic editor (region format). `source` is `bundled:<id>` or a path. */
  logicOpenRegions(source: string): Promise<OpenedRegions>;
  logicSaveRegions(path: string, logic: RegionLogic, oldFormat: boolean): Promise<void>;
  logicCheckRegions(logic: RegionLogic): Promise<LogicIssue[]>;
  logicReachRegions(logic: RegionLogic, scenario: Scenario): Promise<string[]>;

  romCheck(path: string): Promise<RomCheck>;
  /** Randomize and build. An empty seed picks a random one. Writes what `outputs` asks for, next to `outPath`. */
  generate(romPath: string, outPath: string, values: Values, seed: string, outputs: Outputs): Promise<Generated>;
  /** Rebuild a randomized ROM from a clean ROM and an MMR Next patch (.mmrn). */
  applyPatch(romPath: string, patchPath: string, outPath: string): Promise<Patched>;
  /** Show a file in the system file manager. */
  revealFile(path: string): Promise<void>;
  /** Open a file with its usual app (e.g. the tracker in a browser). */
  openFile(path: string): Promise<void>;
  /** Open the manual (every setting, preset and trick) in the browser. */
  openManual(): Promise<void>;

  /** App preferences (layout, last tab, outputs...), not seed settings. */
  prefsLoad(): Promise<Record<string, unknown>>;
  prefsSave(prefs: Record<string, unknown>): Promise<void>;
  /** Open the logic editor window (false: no separate windows here). */
  openLogicEditor(): Promise<boolean>;

  pickFile(title: string, filters: FileFilter[]): Promise<string | null>;
  pickSavePath(title: string, filters: FileFilter[], defaultPath?: string): Promise<string | null>;
  /** In the browser: a link to download a file the randomizer made. */
  downloadUrl?(path: string): string | null;
  /** Open a web link in the user's browser. */
  openLink(url: string): Promise<void>;
}

const tauriBackend: Backend = {
  gameData: () => invoke("game_data"),
  settingsSchema: () => invoke("settings_schema"),
  presets: () => invoke("presets"),
  settingsCheck: (values) => invoke("settings_check", { values }),
  settingsEncode: (values, includeCosmetics) => invoke("settings_encode", { values, includeCosmetics }),
  settingsDecode: (string) => invoke("settings_decode", { string }),
  settingsOpen: (path) => invoke("settings_open", { path }),
  settingsSave: (path, values) => invoke("settings_save", { path, values }),

  logicBundledList: () => invoke("logic_bundled_list"),
  logicOpen: (source) => invoke("logic_open", { source }),
  logicTricks: (id) => invoke("logic_tricks", { id }),
  logicSave: (path, file, legacy) => invoke("logic_save", { path, file, legacy }),
  logicValidate: (file) => invoke("logic_validate", { file }),
  logicEvaluate: (file, scenario) => invoke("logic_evaluate", { file, scenario }),
  logicPlaythrough: (file, scenario) => invoke("logic_playthrough", { file, scenario }),
  logicOpenRegions: (source) => invoke("logic_open_regions", { source }),
  logicSaveRegions: (path, logic, oldFormat) => invoke("logic_save_regions", { path, logic, oldFormat }),
  logicCheckRegions: (logic) => invoke("logic_check_regions", { logic }),
  logicReachRegions: (logic, scenario) => invoke("logic_reach_regions", { logic, scenario }),

  romCheck: (path) => invoke("rom_check", { path }),
  generate: (romPath, outPath, values, seed, outputs) => invoke("generate", { romPath, outPath, values, seed, outputs }),
  applyPatch: (romPath, patchPath, outPath) => invoke("apply_patch", { romPath, patchPath, outPath }),
  revealFile: (path) => revealItemInDir(path),
  openFile: (path) => invoke("open_file", { path }),
  openManual: () => invoke("open_manual"),
  prefsLoad: () => invoke("prefs_load"),
  prefsSave: (prefs) => invoke("prefs_save", { prefs }),
  openLogicEditor: async () => (await invoke("open_logic_editor"), true),

  pickFile: async (title, filters) => {
    const r = await open({ title, filters, multiple: false, directory: false });
    return typeof r === "string" ? r : null;
  },
  pickSavePath: async (title, filters, defaultPath) => (await save({ title, filters, defaultPath })) ?? null,
  openLink: (url) => openUrl(url),
};

/**
 * Read-only stand-in used when the UI runs in a plain browser (e.g. `npm run
 * dev` opened directly) so screens can be previewed without the desktop app.
 * Data comes from `public/preview-data.json` (`mmr ui-data -o ...`).
 * A real web build will replace this with the core compiled to wasm.
 */
function previewBackend(): Backend {
  let data: Promise<any> | null = null;
  const load = () => (data ??= fetch("/preview-data.json").then((r) => r.json()));
  const unavailable = async () => {
    throw new Error("Not available in the browser preview. Run the desktop app for this.");
  };
  return {
    gameData: async () => (await load()).game,
    settingsSchema: async () => (await load()).schema,
    presets: async () => (await load()).presets,
    settingsCheck: async (values) => {
      const schema = (await load()).schema;
      const inactive = [];
      for (const def of schema.settings) {
        for (const ex of def.excludes ?? []) {
          if (JSON.stringify(values[def.key]) === JSON.stringify(ex.when)) {
            for (const key of ex.disables) inactive.push({ key, because: def.key, message: `Has no effect when ${def.label} is ${String(ex.when)}` });
          }
        }
      }
      return { issues: [], inactive };
    },
    settingsEncode: async () => "MMRN.preview",
    settingsDecode: unavailable,
    settingsOpen: unavailable,
    settingsSave: unavailable,
    logicBundledList: async () => [
      { id: "casual", name: "Casual" },
      { id: "glitched", name: "Glitched" },
    ],
    logicOpen: async (source) => {
      const file = (await load()).logic[source.replace("bundled:", "")];
      if (!file) throw new Error("Only bundled logic is available in the browser preview.");
      return { file, format: "current" };
    },
    logicTricks: async (id) => ((await load()).logic[id]?.entries ?? []).filter((e: Entry) => e.trick),
    logicSave: unavailable,
    logicValidate: async () => [],
    logicEvaluate: async () => [],
    logicPlaythrough: unavailable,
    logicOpenRegions: async (source) => {
      const logic = (await load()).regions?.[source.replace("bundled:", "")];
      if (!logic) throw new Error("Only bundled logic is available in the browser preview.");
      return { logic, format: "regions", conversion: null };
    },
    logicSaveRegions: unavailable,
    logicCheckRegions: async () => [],
    logicReachRegions: async () => [],
    romCheck: unavailable,
    generate: unavailable,
    applyPatch: unavailable,
    openFile: unavailable,
    openManual: unavailable,
    revealFile: unavailable,
    prefsLoad: async () => {
      try {
        return JSON.parse(localStorage.getItem("mmr.prefs") ?? "{}");
      } catch {
        return {};
      }
    },
    openLogicEditor: async () => false,
    prefsSave: async (prefs) => {
      try {
        localStorage.setItem("mmr.prefs", JSON.stringify(prefs));
      } catch {}
    },
    pickFile: async () => null,
    pickSavePath: async () => null,
    openLink: async (url) => void window.open(url, "_blank"),
  };
}

/**
 * The browser build: the preview's data plus the randomizer itself (wasm in
 * a worker). Files the player picks are kept in memory under made-up paths;
 * files the randomizer makes are offered as downloads.
 */
function webBackend(): Backend {
  const base = previewBackend();
  const files = new Map<string, Blob>();
  const urls = new Map<string, string>();
  let next = 0;
  let worker: Worker | null = null;
  const pending = new Map<number, { resolve: (r: { json: any; blobs: Uint8Array[] }) => void; reject: (e: Error) => void }>();
  function run(name: string, input: unknown = null, blobs: Uint8Array[] = []): Promise<{ json: any; blobs: Uint8Array[] }> {
    if (!worker) {
      worker = new Worker(new URL("./wasmWorker.ts", import.meta.url), { type: "module" });
      worker.onmessage = (e) => {
        const p = pending.get(e.data.id);
        pending.delete(e.data.id);
        if (e.data.error) p?.reject(new Error(e.data.error));
        else p?.resolve({ json: e.data.json, blobs: e.data.blobs });
      };
    }
    const id = next++;
    return new Promise((resolve, reject) => {
      pending.set(id, { resolve, reject });
      worker!.postMessage({ id, name, input, blobs }, blobs.map((b) => b.buffer as ArrayBuffer));
    });
  }
  const bytes = async (path: string) => {
    const f = files.get(path);
    if (!f) throw new Error("That file is no longer open: choose it again.");
    return new Uint8Array(await f.arrayBuffer());
  };
  /** Keep a made file; its path is its name (unique per seed). */
  const keep = (name: string, data: Uint8Array, type = "application/octet-stream") => {
    const path = `out/${name}`;
    const old = urls.get(path);
    if (old) URL.revokeObjectURL(old);
    files.set(path, new Blob([data as BlobPart], { type }));
    urls.delete(path);
    return path;
  };
  const url = (path: string) => {
    let u = urls.get(path);
    if (!u) {
      const f = files.get(path);
      if (!f) return null;
      u = URL.createObjectURL(f);
      urls.set(path, u);
    }
    return u;
  };
  const download = (path: string) => {
    const u = url(path);
    if (!u) throw new Error("That file is gone: make it again.");
    const a = document.createElement("a");
    a.href = u;
    a.download = path.split("/").pop()!;
    a.click();
  };
  const typeOf = (name: string) => (name.endsWith(".html") ? "text/html" : name.endsWith(".txt") ? "text/plain" : name.endsWith(".png") ? "image/png" : "application/octet-stream");
  /** A random 64-bit seed (the desktop app gets one from the OS). */
  const randomSeed = () => {
    const [hi, lo] = crypto.getRandomValues(new Uint32Array(2));
    return ((BigInt(hi) << 32n) | BigInt(lo)).toString();
  };

  return {
    ...base,
    settingsCheck: async (values) => (await run("settings_check", { values })).json,
    settingsEncode: async (values, includeCosmetics) => (await run("settings_encode", { values, includeCosmetics })).json,
    settingsDecode: async (string) => (await run("settings_decode", { string })).json,
    settingsOpen: async (path) => (await run("settings_open", { text: await files.get(path)!.text() })).json,
    settingsSave: async (path, values) => {
      const r = await run("settings_save", { values });
      download(keep(path.split("/").pop() || "settings.json", r.blobs[0], "application/json"));
    },
    romCheck: async (path) => (await run("rom_check", null, [await bytes(path)])).json,
    generate: async (romPath, _outPath, values, seed, outputs) => {
      const started = performance.now();
      const r = await run("generate", { values, seed: seed.trim() || randomSeed(), outputs }, [await bytes(romPath)]);
      const made: string[] = r.json.files.map((f: { name: string }, i: number) => keep(f.name, r.blobs[i], typeOf(f.name)));
      const find = (kind: string) => {
        const i = r.json.files.findIndex((f: { kind: string }) => f.kind === kind);
        return i < 0 ? null : made[i];
      };
      return {
        romPath: find("rom"),
        patchPath: find("patch"),
        spoilerPath: find("spoiler"),
        trackerPath: find("tracker"),
        files: made,
        seed: r.json.seed,
        hash: r.json.hash,
        hashIcons: r.json.hashIcons,
        millis: performance.now() - started,
        notes: r.json.notes,
      };
    },
    applyPatch: async (romPath, patchPath, outPath) => {
      const r = await run("apply_patch", null, [await bytes(romPath), await bytes(patchPath)]);
      const name = (outPath.split("/").pop() || "patched.z64").replace(/^\d+-/, "");
      const path = keep(name, r.blobs[0]);
      download(path);
      return { romPath: path, info: r.json.info };
    },
    revealFile: async (path) => download(path),
    openFile: async (path) => {
      const u = url(path);
      if (!u) throw new Error("That file is gone: make it again.");
      if (path.endsWith(".html")) window.open(u, "_blank");
      else download(path);
    },
    openManual: async () => {
      const r = await run("manual");
      window.open(url(keep("MMR Next Manual.html", r.blobs[0], "text/html"))!, "_blank");
    },
    pickFile: (_title, filters) =>
      new Promise((resolve) => {
        const input = document.createElement("input");
        input.type = "file";
        const exts = filters.flatMap((f) => f.extensions).filter((e) => e !== "*");
        if (exts.length && !filters.some((f) => f.extensions.includes("*"))) input.accept = exts.map((e) => `.${e}`).join(",");
        input.onchange = () => {
          const f = input.files?.[0];
          if (!f) return resolve(null);
          const path = `in/${next++}-${f.name}`;
          files.set(path, f);
          resolve(path);
        };
        input.addEventListener("cancel", () => resolve(null));
        input.click();
      }),
    // Nothing to choose: files download under these names.
    pickSavePath: async (_title, _filters, defaultPath) => (defaultPath ?? "file").split(/[\\/]/).pop()!,
    downloadUrl: (path) => url(path),
  };
}

export const isDesktop = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
export const backend: Backend = isDesktop ? tauriBackend : webBackend();

/** Turn a thrown command error into a message for the user. */
export function errorMessage(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return String(e);
}
