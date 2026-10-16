// Actions shared by both layouts (guided and classic): ROM, settings files
// and strings, generating. Dialogs they open are shown by Dialogs.svelte.

import { backend } from "./backend";
import type { Generated, Patched } from "./types";
import { app, toasts } from "./state.svelte";

class Dialogs {
  /** The "paste settings string" dialog is open. */
  pasting = $state(false);
  /** A message to show in a dialog (e.g. why generating isn't possible). */
  notice = $state<{ title: string; text: string } | null>(null);
  /** A seed is being built. */
  building = $state(false);
  /** The last finished seed, shown until dismissed. */
  generated = $state<Generated | null>(null);
  /** A patch was applied. */
  patched = $state<Patched | null>(null);
  /** The tracker of the last seed made this session. */
  lastTracker = $state<string | null>(null);
}

export const dialogs = new Dialogs();

export async function openManual() {
  try {
    await backend.openManual();
  } catch (e) {
    toasts.error(e);
  }
}

/** Open the last seed's tracker, or pick one (any seed's tracker .html). */
export async function openTracker() {
  try {
    const path =
      dialogs.lastTracker ??
      (await backend.pickFile("Open a seed's tracker", [
        { name: "Tracker", extensions: ["html"] },
        { name: "All files", extensions: ["*"] },
      ]));
    if (path) await backend.openFile(path);
  } catch (e) {
    toasts.error(e);
  }
}

export async function chooseRom() {
  const path = await backend.pickFile("Choose your Majora's Mask ROM", [
    { name: "N64 ROM", extensions: ["z64", "n64", "v64"] },
    { name: "All files", extensions: ["*"] },
  ]);
  if (path) await checkRom(path);
}

export async function checkRom(path: string) {
  try {
    app.rom = { path, check: await backend.romCheck(path) };
    if (!app.rom.check.ok) toasts.show(app.rom.check.message, "error");
  } catch (e) {
    toasts.error(e);
  }
}

export async function copySettingsString() {
  await navigator.clipboard.writeText(await backend.settingsEncode($state.snapshot(app.values), false));
  toasts.show("Settings string copied. Cosmetics aren't included.");
}

/** Load a settings string; returns whether it worked. */
export async function pasteSettingsString(text: string): Promise<boolean> {
  try {
    app.replaceValues(await backend.settingsDecode(text));
    toasts.show("Settings loaded");
    return true;
  } catch (e) {
    toasts.error(e);
    return false;
  }
}

export async function openSettingsFile() {
  const path = await backend.pickFile("Open settings", [{ name: "Settings", extensions: ["json"] }]);
  if (!path) return;
  try {
    const r = await backend.settingsOpen(path);
    app.replaceValues(r.values);
    toasts.show(
      r.legacy
        ? `Imported settings from an older MMR${r.skipped.length ? ` (skipped ${r.skipped.length} options that no longer exist)` : ""}`
        : "Settings loaded",
    );
  } catch (e) {
    toasts.error(e);
  }
}

export async function saveSettingsFile() {
  const path = await backend.pickSavePath("Save settings", [{ name: "Settings", extensions: ["json"] }], "settings.json");
  if (!path) return;
  try {
    await backend.settingsSave(path, $state.snapshot(app.values));
    toasts.show("Settings saved");
  } catch (e) {
    toasts.error(e);
  }
}

export function resetSettings() {
  if (confirm("Reset every setting, including cosmetics, to its default?")) app.replaceValues(structuredClone(app.schema!.defaults));
}

/** The logic editor opens in its own window (in the browser preview, as a page). */
export async function openLogicEditor() {
  try {
    if (!(await backend.openLogicEditor())) app.go("logic-editor");
  } catch (e) {
    toasts.error(e);
  }
}

export function rollSeed() {
  app.seed = String(Math.floor(Math.random() * 2 ** 31));
}

export async function generate() {
  if (!app.rom?.check.ok) {
    await chooseRom();
    return;
  }
  const rom = app.rom.path;
  const folder = rom.replace(/[^\\/]*$/, "");
  const name = `MMRN-${app.seed.trim() ? app.seed.trim().replace(/[^\w-]+/g, "_") : "random"}.z64`;
  const out = await backend.pickSavePath("Save the randomized ROM", [{ name: "N64 ROM", extensions: ["z64"] }], folder + name);
  if (!out) return;
  dialogs.building = true;
  try {
    dialogs.generated = await backend.generate(rom, out, $state.snapshot(app.values), app.seed, $state.snapshot(app.outputs));
    if (dialogs.generated.trackerPath) dialogs.lastTracker = dialogs.generated.trackerPath;
  } catch (e) {
    dialogs.notice = { title: "Couldn't make this seed", text: String(e) };
  } finally {
    dialogs.building = false;
  }
}

/** Turn a patch (.mmrn) and the clean ROM into the randomized ROM. */
export async function applyPatch() {
  if (!app.rom?.check.ok) {
    await chooseRom();
    if (!app.rom?.check.ok) return;
  }
  const patch = await backend.pickFile("Open a patch", [{ name: "MMR Next patch", extensions: ["mmrn"] }]);
  if (!patch) return;
  const out = await backend.pickSavePath("Save the patched ROM", [{ name: "N64 ROM", extensions: ["z64"] }], patch.replace(/\.mmrn$/i, ".z64"));
  if (!out) return;
  dialogs.building = true;
  try {
    dialogs.patched = await backend.applyPatch(app.rom!.path, patch, out);
  } catch (e) {
    dialogs.notice = { title: "Couldn't apply the patch", text: String(e) };
  } finally {
    dialogs.building = false;
  }
}
