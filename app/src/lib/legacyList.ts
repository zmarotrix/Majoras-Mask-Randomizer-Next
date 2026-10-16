// MMR 1.16's compact item-list strings (e.g. "200000--1fbfc-5800000-"), as
// shown in its Item Randomization / Starting Items / Junk Locations boxes.
// Mirrors crates/mmr-settings/src/legacy.rs (`encode_item_list`).

import type { GameData } from "./types";

export type ListPool = "locations" | "starting";

function base(game: GameData, pool: ListPool): string[] {
  return game.items.filter((i) => (pool === "locations" ? i.location !== null : i.starting_item)).map((i) => i.id);
}

export function encodeList(game: GameData, pool: ListPool, items: string[]): string {
  const ids = base(game, pool);
  const index = new Map(ids.map((id, i) => [id, i]));
  const words = new Array<number>(Math.ceil(ids.length / 32)).fill(0);
  for (const item of items) {
    const i = index.get(item);
    if (i !== undefined) words[i >> 5] = (words[i >> 5] | (1 << (i & 31))) >>> 0;
  }
  return words
    .reverse()
    .map((w) => (w === 0 ? "" : w.toString(16)))
    .join("-");
}

/** `null` if the string doesn't fit this version's item list. */
export function decodeList(game: GameData, pool: ListPool, text: string): string[] | null {
  const ids = base(game, pool);
  const s = text.trim();
  if (!s) return [];
  const parts = s.split("-");
  if (parts.length !== Math.ceil(ids.length / 32)) return null;
  const out: string[] = [];
  for (const [w, part] of parts.reverse().entries()) {
    if (part && !/^[0-9a-fA-F]{1,8}$/.test(part)) return null;
    const word = part ? parseInt(part, 16) : 0;
    for (let bit = 0; bit < 32; bit++) {
      if ((word >>> bit) & 1) {
        const id = ids[w * 32 + bit];
        if (id === undefined) return null;
        out.push(id);
      }
    }
  }
  return out;
}

export function poolSize(game: GameData, pool: ListPool): number {
  return base(game, pool).length;
}
