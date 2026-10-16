// Human-readable names for logic ids, entries and settings conditions.

import type { Entry, EntryKind, Item, SettingCondition, SettingDef } from "./types";

export interface NameIndex {
  items: Map<string, Item>;
  settings: Map<string, SettingDef>;
  regions: Map<string, string>;
}

/** "AreaWoodFallTempleAccess" -> "Woodfall Temple Access" */
export function humanize(id: string): string {
  if (/\s/.test(id)) return id; // already human (helpers, tricks)
  let s = id.replace(/^(Area|Other|Setting)(?=[A-Z])/, "");
  s = s.replace(/([a-z0-9])([A-Z])/g, "$1 $2").replace(/([A-Z]+)([A-Z][a-z])/g, "$1 $2");
  return s.replace(/Wood Fall/g, "Woodfall");
}

export function kindOf(entry: Entry, idx: NameIndex): EntryKind {
  if (entry.trick) return "trick";
  if (entry.setting) return "setting";
  const item = idx.items.get(entry.id);
  if (item) return item.location ? "location" : "event";
  return "helper";
}

/** Whether a reference to `id` means "has this item". */
export function isItemRef(id: string, idx: NameIndex): boolean {
  const item = idx.items.get(id);
  return !!item && item.name !== null;
}

/** Name for an entry, as shown in the entry list and editor header. */
export function entryName(id: string, idx: NameIndex): string {
  const item = idx.items.get(id);
  if (item?.location) return item.location;
  return humanize(id);
}

/** Name for a reference inside requirements. */
export function refName(id: string, idx: NameIndex): string {
  const item = idx.items.get(id);
  if (item?.name) return item.name;
  return humanize(id);
}

export function regionName(id: string, idx: NameIndex): string | null {
  const item = idx.items.get(id);
  return item?.region ? idx.regions.get(item.region) ?? item.region : null;
}

export const KIND_LABELS: Record<EntryKind, string> = {
  location: "Location",
  event: "Area / event",
  trick: "Trick",
  setting: "Setting",
  helper: "Helper",
};

export const KIND_HELP: Record<EntryKind, string> = {
  location: "A place where an item can be found.",
  event: "An area you can reach or an event that can happen.",
  trick: "A technique players can opt into. Satisfied when the trick is enabled.",
  setting: "Satisfied when the randomizer settings match a condition.",
  helper: "A named set of requirements that other entries reuse.",
};

function optionLabel(def: SettingDef | undefined, value: unknown): string {
  const opt = def?.options?.find((o) => o.value === value);
  return opt?.label ?? String(value);
}

/** "Enemy Mode doesn't include Randomized" */
export function describeCondition(c: SettingCondition, idx: NameIndex, negate = false): string {
  if ("not" in c) return describeCondition(c.not, idx, !negate);
  if ("randomized" in c) {
    const loc = idx.items.get(c.randomized)?.location ?? humanize(c.randomized);
    return `${loc} is ${negate ? "not " : ""}randomized`;
  }
  const def = idx.settings.get(c.setting);
  const label = def?.label ?? humanize(c.setting);
  if (c.is !== undefined) return `${label} is ${negate ? "not " : ""}${optionLabel(def, c.is)}`;
  if (c.has !== undefined) return `${label} ${negate ? "doesn't include" : "includes"} ${optionLabel(def, c.has)}`;
  return `${label} is ${negate ? "off" : "on"}`;
}
