// Working with logic rules (region format): an id, all of / any of several
// rules, or "N of" a list of ids.

import type { Rule } from "./types";

export const ALWAYS: Rule = { all: [] };

export function isAlways(r: Rule | undefined): boolean {
  return r === undefined || (typeof r === "object" && "all" in r && r.all.length === 0);
}

export function isRef(r: Rule): r is string {
  return typeof r === "string";
}
export function isAll(r: Rule): r is { all: Rule[] } {
  return typeof r === "object" && "all" in r;
}
export function isAny(r: Rule): r is { any: Rule[] } {
  return typeof r === "object" && "any" in r;
}
export function isCount(r: Rule): r is { count: number; of: string[] } {
  return typeof r === "object" && "count" in r;
}

/** Every id a rule mentions. */
export function refsOf(r: Rule | undefined, out: string[] = []): string[] {
  if (r === undefined) return out;
  if (isRef(r)) out.push(r);
  else if (isAll(r)) r.all.forEach((x) => refsOf(x, out));
  else if (isAny(r)) r.any.forEach((x) => refsOf(x, out));
  else out.push(...r.of);
  return out;
}

/** Simplify: drop empty groups inside groups, unwrap groups of one. Not
 * applied while editing (a new group starts with one child). */
export function tidy(r: Rule): Rule {
  if (isRef(r) || isCount(r)) return r;
  const kids = (isAll(r) ? r.all : r.any).map(tidy).filter((k) => !(typeof k === "object" && !isCount(k) && (isAll(k) ? k.all : k.any).length === 0));
  if (kids.length === 1) return kids[0];
  return isAll(r) ? { all: kids } : { any: kids };
}

/** Path to a node inside a rule: indexes into all / any lists. */
export type RulePath = number[];

export function children(r: Rule): Rule[] | null {
  if (isAll(r)) return r.all;
  if (isAny(r)) return r.any;
  return null;
}

/** A copy of `root` with the node at `path` replaced (or removed if null). */
export function replaceAt(root: Rule, path: RulePath, next: Rule | null): Rule {
  if (path.length === 0) return next ?? ALWAYS;
  const kids = children(root);
  if (!kids) return root;
  const [i, ...rest] = path;
  const copy = [...kids];
  if (rest.length === 0) {
    if (next === null) copy.splice(i, 1);
    else copy[i] = next;
  } else {
    copy[i] = replaceAt(copy[i], rest, next);
  }
  return isAll(root) ? { all: copy } : { any: copy };
}

/** Add a child to the group at `path`. */
export function appendAt(root: Rule, path: RulePath, child: Rule): Rule {
  const target = path.reduce<Rule>((r, i) => children(r)![i], root);
  const kids = children(target);
  if (!kids) return root;
  const grown = isAll(target) ? { all: [...kids, child] } : { any: [...kids, child] };
  return replaceAt(root, path, grown);
}
