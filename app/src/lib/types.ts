// Types mirroring the Rust core (crates/*). Keep in sync with the serde shapes.

export interface Item {
  id: string;
  index: number;
  name: string | null;
  location: string | null;
  region: string | null;
  item_category: string | null;
  location_category: string | null;
  classic_category: string | null;
  flags: string[];
  multi_location: string[] | null;
  entrance: boolean;
  starting_item: boolean;
}

export interface Region {
  id: string;
  name: string;
  area: string | null;
}

export interface Category {
  id: string;
  value: number;
  description: string | null;
}

export interface GameData {
  items: Item[];
  regions: Region[];
  categories: { ItemCategory: Category[]; LocationCategory: Category[]; ClassicCategory: Category[] };
}

export type SettingKind = "bool" | "number" | "enum" | "flags" | "item_list" | "tricks" | "file" | "custom";

export interface OptionDef {
  value: string;
  label: string;
  description?: string;
}

export interface SettingDef {
  key: string;
  label: string;
  category: string;
  description: string | null;
  type: SettingKind;
  default: unknown;
  options?: OptionDef[];
  min?: number;
  max?: number;
  integer: boolean;
  nullable: boolean;
  pool?: "locations" | "starting_items";
  excludes?: { when: unknown; disables: string[] }[];
  /** For "custom" settings: the editor that shows it. */
  editor?: StructuredEditor;
  /** Edited inside another setting's editor; not shown on its own. */
  companion_of?: string;
  /** Transformation forms the value has an entry for. */
  forms?: string[];
  /** Editable parts (e.g. HUD elements), value = key in the value object. */
  fields?: OptionDef[];
}

export type StructuredEditor =
  | "tunic-colors"
  | "energy-colors"
  | "instruments"
  | "hud-colors"
  | "dpad"
  | "trap-weights"
  | "starting-groups"
  | "hint-priorities";

export interface CategoryDef {
  id: string;
  label: string;
  description: string;
}

export interface LayoutGroup {
  id: string;
  title: string;
  icon: string;
  description: string;
  settings: string[];
  /** Related settings shown under a small sub-heading. */
  more?: { label: string; settings: string[] };
}

export interface LayoutSection {
  id: string;
  label: string;
  icon: string;
  description: string;
  groups: LayoutGroup[];
}

export interface Layout {
  sections: LayoutSection[];
  /** Friendlier labels, by setting key. */
  labels: Record<string, string>;
  /** Friendlier option names: setting key -> option value -> label. */
  optionLabels: Record<string, Record<string, string>>;
}

export interface SettingsSchema {
  settings: SettingDef[];
  categories: CategoryDef[];
  defaults: Values;
  layout: Layout;
}

export type Values = Record<string, unknown>;

export interface Preset {
  id: string;
  name: string;
  description?: string | null;
  highlights?: string[];
  settings: Values;
}

export interface SettingsIssue {
  key: string;
  message: string;
}

export interface Inactive {
  key: string;
  because: string;
  message: string;
}

// ---- Logic ----

export type Period = "Day1" | "Night1" | "Day2" | "Night2" | "Day3" | "Night3";
export const PERIODS: Period[] = ["Day1", "Night1", "Day2", "Night2", "Day3", "Night3"];

export interface Time {
  needed?: Period[];
  available?: Period[];
  setup?: Period[];
  /** No coming back within the cycle (the Moon, the Clock Tower roof). */
  no_return?: boolean;
  /** The times are opening hours (shop doors, grottos). */
  opening_hours?: boolean;
}

/** A requirement: an id, or all / any of several, or N of a list. */
export type Rule = string | { all: Rule[] } | { any: Rule[] } | { count: number; of: string[] };

export interface Exit {
  to: string;
  rule?: Rule;
}

/** A check (location) or event inside a region. */
export interface Spot {
  id: string;
  rule?: Rule;
  time?: Time;
  /** Must happen before these in the same cycle. */
  before?: string[];
  /** Also needed when what's found here doesn't last through time. */
  temporary_requires?: string[];
}

export interface LogicRegion {
  name: string;
  exits?: Exit[];
  locations?: Spot[];
  events?: Spot[];
  time?: Time;
}

/** Logic in the region format (version 2): what the logic editor edits. */
export interface RegionLogic {
  format: "mmr-logic";
  version: 2;
  start: string;
  regions: LogicRegion[];
  /** Helpers, tricks, settings, and anything not (yet) in a region. */
  entries: Entry[];
}

export interface Conversion {
  equivalent: boolean;
  scenarios: number;
  entries: number;
  regions: number;
  unmapped: number;
  mismatched: string[];
}

export interface OpenedRegions {
  logic: RegionLogic;
  format: "regions" | "entries" | "old";
  conversion: Conversion | null;
}

export interface Trick {
  description?: string;
  category?: string;
  url?: string;
}

export type SettingCondition =
  | { not: SettingCondition }
  | { randomized: string }
  | { setting: string; is?: unknown; has?: string };

export interface Entry {
  id: string;
  requires?: string[];
  any_of?: string[][];
  time?: Time;
  trick?: Trick;
  setting?: SettingCondition;
  note?: string;
}

export interface LogicFile {
  format: "mmr-logic";
  version: number;
  description?: string;
  entries: Entry[];
}

export type EntryKind = "location" | "event" | "trick" | "setting" | "helper";

export type Severity = "error" | "warning" | "info";

export interface LogicIssue {
  severity: Severity;
  code: string;
  entry: string | null;
  target: string | null;
  message: string;
}

export interface Scenario {
  items: string[];
  tricks: string[];
  settings: Values;
}

export interface Playthrough {
  beatable: boolean;
  spheres: string[][];
  unreachable: string[];
}

/** A finished seed (`generate`). */
export interface HashIcon {
  name: string;
  /** PNG data URL. */
  png: string;
}

export interface Generated {
  romPath: string | null;
  patchPath: string | null;
  spoilerPath: string | null;
  trackerPath: string | null;
  /** Every file written. */
  files: string[];
  seed: string;
  /** Build hash, hex. */
  hash: string;
  hashIcons: HashIcon[];
  millis: number;
  /** Anything worth knowing about how the seed was made. */
  notes: string[];
}

export interface Patched {
  romPath: string;
  /** What the patch says about its seed. */
  info: { seed?: string; settings?: string; hash?: string; hashIcons?: string[]; version?: string } | null;
}

export interface RomCheck {
  ok: boolean;
  message: string;
  info: { header: { name: string; game_code: string }; original_order: string } | null;
}
