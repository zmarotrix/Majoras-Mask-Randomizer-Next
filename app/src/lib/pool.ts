// Checks (randomizable locations) and the facets used to group, filter and
// bulk-edit them in the item pool editors.

import { humanize } from "./names";
import type { GameData, Item } from "./types";

export interface Check {
  id: string;
  /** Location name, or the item name for item pools (starting items). */
  name: string;
  /** What's normally found here. */
  item: string;
  region: string;
  regionName: string;
  area: string;
  type: string;
  itemCat: string;
  classic: string;
  /** Position in game data, for stable ordering. */
  index: number;
}

export type Facet = "type" | "area" | "region" | "itemCat" | "classic";

export const FACET_LABELS: Record<Facet, string> = {
  type: "Check type",
  area: "Area",
  region: "Region",
  itemCat: "Vanilla item",
  classic: "Classic category",
};

export const AREAS: { id: string; label: string }[] = [
  { id: "Town", label: "Clock Town and Field" },
  { id: "Swamp", label: "Swamp" },
  { id: "Mountain", label: "Mountain" },
  { id: "Ocean", label: "Great Bay" },
  { id: "Canyon", label: "Ikana" },
  { id: "Ranch", label: "Ranch" },
  { id: "None", label: "Other" },
];

/** Check types (legacy LocationCategory), with friendlier names and icons. */
export const TYPES: Record<string, { label: string; icon: string; order: number }> = {
  Chests: { label: "Chests", icon: "chest", order: 1 },
  NpcRewards: { label: "NPC rewards", icon: "user", order: 2 },
  Purchases: { label: "Shops", icon: "shop", order: 3 },
  Minigames: { label: "Minigames", icon: "gamepad", order: 4 },
  BossFights: { label: "Bosses", icon: "crown", order: 5 },
  MoonItems: { label: "Moon", icon: "moon", order: 6 },
  StartingItems: { label: "Starting items", icon: "flag", order: 7 },
  Events: { label: "Events", icon: "star", order: 8 },
  Freestanding: { label: "Freestanding", icon: "diamond", order: 10 },
  HitSpots: { label: "Hit spots", icon: "target", order: 11 },
  InvisibleItems: { label: "Invisible", icon: "eye", order: 12 },
  EnemySpawn: { label: "Enemy drops", icon: "sword", order: 13 },
  Fairies: { label: "Fairies", icon: "sparkles", order: 14 },
  Butterflies: { label: "Butterflies", icon: "butterfly", order: 15 },
  SoftSoil: { label: "Soft soil", icon: "sprout", order: 16 },
  Jars: { label: "Pots", icon: "pot", order: 20 },
  Crates: { label: "Crates", icon: "crate", order: 21 },
  Barrels: { label: "Barrels", icon: "barrel", order: 22 },
  Rocks: { label: "Rocks", icon: "rock", order: 23 },
  Grass: { label: "Grass", icon: "grass", order: 24 },
  SmallSnowballs: { label: "Small snowballs", icon: "snow", order: 25 },
  LargeSnowballs: { label: "Large snowballs", icon: "snow", order: 26 },
  Beehives: { label: "Beehives", icon: "hive", order: 27 },
  Trees: { label: "Trees", icon: "tree", order: 28 },
  NotebookEntries: { label: "Notebook", icon: "notebook", order: 30 },
  Scoops: { label: "Bottle scoops", icon: "scoop", order: 31 },
  Frogs: { label: "Frogs", icon: "frog", order: 32 },
  GlitchesRequired: { label: "Glitches required", icon: "bug", order: 40 },
};

export function typeInfo(type: string) {
  return TYPES[type] ?? { label: humanize(type), icon: "help", order: 99 };
}

/** Human label for a facet value. */
export function facetLabel(facet: Facet, value: string, regions?: Map<string, string>): string {
  switch (facet) {
    case "type":
      return typeInfo(value).label;
    case "area":
      return AREAS.find((a) => a.id === value)?.label ?? value;
    case "region":
      return regions?.get(value) ?? humanize(value);
    default:
      return sentence(humanize(value));
  }
}

/** "Pieces Of Heart" -> "Pieces of heart" */
function sentence(s: string): string {
  return s.charAt(0) + s.slice(1).toLowerCase();
}

/** Sort key for a facet value. */
export function facetOrder(facet: Facet, value: string): number | string {
  if (facet === "type") return typeInfo(value).order;
  if (facet === "area") return AREAS.findIndex((a) => a.id === value);
  return value;
}

export function locationChecks(game: GameData): Check[] {
  const regions = new Map(game.regions.map((r) => [r.id, r]));
  return game.items
    .filter((i) => i.location && i.location_category !== "Fake")
    .map((i) => toCheck(i, i.location!, regions));
}

/** Locations that are shuffled: the only ones that can be junk locations. */
export function shuffledChecks(game: GameData, shuffled: string[]): Check[] {
  const on = new Set(shuffled);
  return locationChecks(game).filter((c) => on.has(c.id));
}

export function startingItemChecks(game: GameData): Check[] {
  const regions = new Map(game.regions.map((r) => [r.id, r]));
  return game.items.filter((i) => i.starting_item).map((i) => toCheck(i, i.name!, regions));
}

function toCheck(i: Item, name: string, regions: Map<string, { name: string; area: string | null }>): Check {
  const region = i.region ?? "VariousPlaces";
  const r = regions.get(region);
  return {
    id: i.id,
    name,
    item: i.name ?? "",
    region,
    regionName: r?.name ?? humanize(region),
    area: r?.area && r.area !== "None" ? r.area : "None",
    type: i.location_category ?? "Events",
    itemCat: i.item_category ?? "None",
    classic: i.classic_category ?? "None",
    index: i.index,
  };
}

export function facetValue(c: Check, f: Facet): string {
  return c[f];
}

/** Words of a search query that all have to match. */
export function matchesQuery(c: Check, words: string[]): boolean {
  if (!words.length) return true;
  const hay = `${c.name} ${c.item} ${c.regionName} ${typeInfo(c.type).label} ${facetLabel("itemCat", c.itemCat)}`.toLowerCase();
  return words.every((w) => hay.includes(w));
}

export function queryWords(q: string): string[] {
  return q.trim().toLowerCase().split(/\s+/).filter(Boolean);
}

/** Group checks by a facet, in display order. */
export function groupBy(checks: Check[], f: Facet): [string, Check[]][] {
  const map = new Map<string, Check[]>();
  for (const c of checks) {
    const k = facetValue(c, f);
    let list = map.get(k);
    if (!list) map.set(k, (list = []));
    list.push(c);
  }
  return [...map.entries()].sort(([a], [b]) => {
    const x = facetOrder(f, a);
    const y = facetOrder(f, b);
    return typeof x === "number" && typeof y === "number" ? x - y : String(x).localeCompare(String(y));
  });
}

/**
 * Collapse runs of near-identical checks ("Deku Palace Item", "... Item 2",
 * ... "Item 17") into one group, so long runs of rupees and pots don't drown
 * out everything else.
 */
export interface Run {
  key: string;
  label: string;
  checks: Check[];
}

export function collapseRuns(checks: Check[], min = 4): (Check | Run)[] {
  const base = (c: Check) => `${c.type}|${c.name.replace(/\s*#?\d+$/, "")}`;
  const counts = new Map<string, number>();
  for (const c of checks) counts.set(base(c), (counts.get(base(c)) ?? 0) + 1);
  const out: (Check | Run)[] = [];
  const runs = new Map<string, Run>();
  for (const c of checks) {
    const k = base(c);
    if ((counts.get(k) ?? 0) < min) {
      out.push(c);
      continue;
    }
    let run = runs.get(k);
    if (!run) {
      run = { key: k, label: c.name.replace(/\s*#?\d+$/, ""), checks: [] };
      runs.set(k, run);
      out.push(run);
    }
    run.checks.push(c);
  }
  return out;
}

export function isRun(x: Check | Run): x is Run {
  return "checks" in x;
}

/** Remove a region's name from the start of a check name: "Deku Palace Pot 2" -> "Pot 2". */
export function shortName(name: string, regionName: string): string {
  const trimmed = name.startsWith(regionName + " ") ? name.slice(regionName.length + 1) : name;
  return trimmed || name;
}
