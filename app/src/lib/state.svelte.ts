// Global app state (Svelte 5 runes).

import { backend, errorMessage } from "./backend";
import type { NameIndex } from "./names";
import type {
  Entry,
  GameData,
  Inactive,
  LogicFile,
  LogicIssue,
  Preset,
  RomCheck,
  Scenario,
  SettingsIssue,
  SettingsSchema,
  Values,
} from "./types";

/** A settings category id, "search" for settings search results, or a tool. */
export type Screen = string;

class Toasts {
  list = $state<{ id: number; text: string; kind: "info" | "error" }[]>([]);
  #next = 0;
  show(text: string, kind: "info" | "error" = "info") {
    const id = this.#next++;
    this.list.push({ id, text, kind });
    setTimeout(() => (this.list = this.list.filter((t) => t.id !== id)), kind === "error" ? 8000 : 3500);
  }
  error(e: unknown) {
    this.show(errorMessage(e), "error");
  }
}

export const toasts = new Toasts();

/** This is the logic editor's own window (by its window label in the
 * desktop app; `#logic-editor` in a browser, for previewing). */
export const isEditorWindow =
  typeof window !== "undefined" &&
  (location.hash === "#logic-editor" || (window as unknown as { __TAURI_INTERNALS__?: { metadata?: { currentWindow?: { label?: string } } } }).__TAURI_INTERNALS__?.metadata?.currentWindow?.label === "logic-editor");

export type Layout = "guided" | "classic";

/** What generating writes (MMR 1.16's "Outputs" box). Not part of settings. */
export interface Outputs {
  rom: boolean;
  patch: boolean;
  spoiler: boolean;
  /** The interactive, spoiler-free HTML tracker. */
  tracker: boolean;
}

/** Saved app preferences (backend.prefsLoad / prefsSave), not seed settings. */
export type Theme = "system" | "light" | "dark";

type Prefs = {
  layout: Layout;
  classicTab: string;
  outputs: Outputs;
  theme: Theme;
  /** The moving background (Majora's lair). */
  motion: boolean;
};

const DEFAULT_PREFS: Prefs = {
  layout: "guided",
  classicTab: "main",
  outputs: { rom: true, patch: false, spoiler: true, tracker: true },
  theme: "system",
  motion: true,
};

function applyTheme(theme: Theme) {
  if (typeof document === "undefined") return;
  if (theme === "system") delete document.documentElement.dataset.theme;
  else document.documentElement.dataset.theme = theme;
}

/** Preferences from before they were saved to a file (browser storage). */
function oldPrefs(): Partial<Prefs> {
  const read = (key: string) => {
    try {
      const v = localStorage.getItem(key);
      return v === null ? undefined : JSON.parse(v);
    } catch {
      return undefined;
    }
  };
  const out: Partial<Prefs> = {};
  const layout = read("mmr.layout")?.v;
  if (layout === "guided" || layout === "classic") out.layout = layout;
  const outputs = read("mmr.outputs");
  if (outputs) out.outputs = { ...DEFAULT_PREFS.outputs, ...outputs };
  try {
    const tab = localStorage.getItem("mmr.classicTab");
    if (tab) out.classicTab = tab;
  } catch {}
  return out;
}

class AppState {
  /** Guided (cards and help) or classic (MMR 1.16's window layout). */
  layout = $state<Layout>(DEFAULT_PREFS.layout);
  /** Ask which layout to use (first launch). */
  chooseLayout = $state(false);
  /** The classic layout's open tab. */
  classicTab = $state(DEFAULT_PREFS.classicTab);
  outputs = $state<Outputs>({ ...DEFAULT_PREFS.outputs });
  theme = $state<Theme>(DEFAULT_PREFS.theme);
  motion = $state(DEFAULT_PREFS.motion);
  screen = $state<Screen>("quick");
  /** The open group tab on each settings section. */
  groupTab = $state<Record<string, string>>({});
  /** The setting the help panel explains (hovered or focused). */
  help = $state<string | null>(null);
  /** The option of that setting being pointed at (e.g. one goal chip). */
  helpOption = $state<string | null>(null);
  /** Sidebar open on narrow screens. */
  navOpen = $state(false);
  /** Settings search, typed in the sidebar. */
  query = $state("");
  seed = $state("");
  ready = $state(false);
  loadError = $state<string | null>(null);

  game = $state.raw<GameData | null>(null);
  schema = $state.raw<SettingsSchema | null>(null);
  presets = $state.raw<Preset[]>([]);
  names = $state.raw<NameIndex>({ items: new Map(), settings: new Map(), regions: new Map() });

  /** Current setting values (always complete). */
  values = $state<Values>({});
  /** The preset the values came from, if they haven't been changed since. */
  presetId = $state<string | null>(null);
  /** The last preset applied, even if settings were changed afterwards. */
  basePresetId = $state<string | null>(null);
  settingsIssues = $state.raw<SettingsIssue[]>([]);
  inactive = $state.raw<Map<string, Inactive>>(new Map());

  rom = $state<{ path: string; check: RomCheck } | null>(null);

  async init() {
    try {
      const [game, schema, presets, saved] = await Promise.all([
        backend.gameData(),
        backend.settingsSchema(),
        backend.presets(),
        backend.prefsLoad().catch(() => ({})),
      ]);
      const old = oldPrefs();
      const prefs = { ...DEFAULT_PREFS, ...old, ...(saved as Partial<Prefs>) };
      // First launch: no layout has ever been chosen.
      this.chooseLayout = !("layout" in (saved as object)) && !old.layout;
      this.layout = prefs.layout === "classic" ? "classic" : "guided";
      this.classicTab = prefs.classicTab;
      this.outputs = { ...DEFAULT_PREFS.outputs, ...prefs.outputs };
      this.theme = ["light", "dark"].includes(prefs.theme) ? prefs.theme : "system";
      this.motion = prefs.motion !== false;
      applyTheme(this.theme);
      this.game = game;
      this.schema = schema;
      this.presets = presets;
      this.names = {
        items: new Map(game.items.map((i) => [i.id, i])),
        settings: new Map(schema.settings.map((s) => [s.key, s])),
        regions: new Map(game.regions.map((r) => [r.id, r.name])),
      };
      this.values = structuredClone(schema.defaults);
      await this.checkSettings();
      this.ready = true;
    } catch (e) {
      this.loadError = errorMessage(e);
    }
  }

  setValue(key: string, value: unknown) {
    this.values[key] = value;
    this.presetId = null;
    this.checkSettings();
  }

  replaceValues(values: Values, presetId: string | null = null) {
    this.values = values;
    this.presetId = presetId;
    this.basePresetId = presetId;
    this.checkSettings();
  }

  applyPreset(preset: Preset) {
    const values: Values = structuredClone(this.schema!.defaults);
    for (const [k, v] of Object.entries(this.values)) if (k.startsWith("Cosmetic.")) values[k] = v;
    for (const [k, v] of Object.entries(preset.settings)) if (k in values) values[k] = structuredClone(v);
    this.replaceValues(values, preset.id);
  }

  /** Display label for a setting (layout label, else schema label). */
  label(key: string): string {
    return this.schema?.layout.labels[key] ?? this.names.settings.get(key)?.label ?? key;
  }

  /** Display label for one option of an enum/flags setting. */
  optionLabel(key: string, value: string): string {
    return (
      this.schema?.layout.optionLabels?.[key]?.[value] ??
      this.names.settings.get(key)?.options?.find((o) => o.value === value)?.label ??
      value
    );
  }

  /** The layout group and section a setting lives in. */
  placeOf(key: string) {
    for (const section of this.schema?.layout.sections ?? []) {
      for (const group of section.groups) {
        if (group.settings.includes(key) || group.more?.settings.includes(key)) return { section, group };
      }
    }
    return null;
  }

  setLayout(layout: Layout) {
    this.layout = layout;
    this.chooseLayout = false;
    if (layout === "classic" && this.screen !== "logic-editor") this.screen = "quick";
    this.savePrefs();
  }

  setClassicTab(tab: string) {
    this.classicTab = tab;
    this.savePrefs();
  }

  setTheme(theme: Theme) {
    this.theme = theme;
    applyTheme(theme);
    this.savePrefs();
  }

  setMotion(on: boolean) {
    this.motion = on;
    this.savePrefs();
  }

  setOutput(key: keyof Outputs, on: boolean) {
    this.outputs[key] = on;
    this.savePrefs();
  }

  /** Save preferences right away, so the next launch opens the same way. */
  savePrefs() {
    const prefs: Prefs = {
      layout: this.layout,
      classicTab: this.classicTab,
      outputs: $state.snapshot(this.outputs),
      theme: this.theme,
      motion: this.motion,
    };
    backend.prefsSave(prefs).catch((e) => toasts.show(`Couldn't save preferences: ${errorMessage(e)}`, "error"));
  }

  go(screen: Screen) {
    this.screen = screen;
    this.query = "";
    this.help = null;
    this.navOpen = false;
  }

  isChanged(key: string): boolean {
    return JSON.stringify(sorted(this.values[key])) !== JSON.stringify(sorted(this.schema?.defaults[key]));
  }

  async checkSettings() {
    const r = await backend.settingsCheck($state.snapshot(this.values));
    this.settingsIssues = r.issues;
    this.inactive = new Map(r.inactive.map((i) => [i.key, i]));
  }
}

function sorted(v: unknown): unknown {
  return Array.isArray(v) ? [...v].map((x) => JSON.stringify(x)).sort() : v;
}

export const app = new AppState();

// ---- Logic editor document ----

interface Change {
  /** Entries by id before and after; null means "didn't exist". */
  before: Map<string, { entry: Entry; index: number } | null>;
  after: Map<string, { entry: Entry; index: number } | null>;
  label: string;
}


