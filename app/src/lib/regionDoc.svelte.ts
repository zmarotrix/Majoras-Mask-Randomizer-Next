// State of the logic editor: the open region logic, what's selected, undo
// and redo, problems, and lookups (names, groups, ways in, used by).

import { backend, errorMessage } from "./backend";
import { app, toasts } from "./state.svelte";
import { humanize } from "./names";
import { refsOf } from "./rules";
import type { Conversion, Entry, LogicIssue, LogicRegion, RegionLogic, Rule, Spot, Time } from "./types";

export const REGION_PREFIX = "Region: ";

/** The page shown in the middle column. */
export type Page = { kind: "region"; name: string } | { kind: "entry"; id: string };

/** What the inspector (right column) edits. */
export type Target =
  | { kind: "region"; region: string }
  | { kind: "exit"; region: string; index: number }
  | { kind: "location" | "event"; region: string; index: number }
  | { kind: "entry"; id: string };

export type RefKind = "item" | "region" | "event" | "helper" | "trick" | "setting" | "location" | "unknown";

export type NavTab = "regions" | "helpers" | "tricks" | "settings";

export interface PickOption {
  id: string;
  label: string;
  kind: RefKind;
  detail: string;
}

interface Place {
  /** Where the reference is, for "used by" lists. */
  label: string;
  page: Page;
  target: Target;
}

class RegionDoc {
  file = $state<RegionLogic | null>(null);
  /** Where it came from: bundled id or a path. */
  source = $state<{ kind: "bundled"; id: string; name: string } | { kind: "file"; path: string } | null>(null);
  /** Set when the file was converted from an older format when opened. */
  conversion = $state<(Conversion & { format: string }) | null>(null);
  dirty = $state(false);
  loading = $state(false);

  page = $state<Page | null>(null);
  target = $state<Target | null>(null);
  nav = $state<NavTab>("regions");
  query = $state("");

  issues = $state.raw<LogicIssue[]>([]);

  #undo: { label: string; snapshot: string }[] = [];
  #redo: { label: string; snapshot: string }[] = [];
  undoLabel = $state<string | null>(null);
  redoLabel = $state<string | null>(null);
  #history: Page[] = [];

  // ---- Lookups ----------------------------------------------------------

  regions = $derived<LogicRegion[]>(this.file?.regions ?? []);
  regionByName = $derived(new Map(this.regions.map((r) => [r.name, r])));
  entryById = $derived(new Map((this.file?.entries ?? []).map((e) => [e.id, e])));
  /** Event ids defined inside regions. */
  eventIds = $derived(new Set(this.regions.flatMap((r) => (r.events ?? []).map((e) => e.id))));

  /** Exits leading into each region. */
  waysIn = $derived.by(() => {
    const map = new Map<string, { from: string; index: number; rule: Rule | undefined }[]>();
    for (const r of this.regions) {
      (r.exits ?? []).forEach((e, index) => {
        let list = map.get(e.to);
        if (!list) map.set(e.to, (list = []));
        list.push({ from: r.name, index, rule: e.rule });
      });
    }
    return map;
  });

  /** Everywhere each id is referenced. */
  usedBy = $derived.by(() => {
    const map = new Map<string, Place[]>();
    const add = (rule: Rule | undefined, extra: string[] | undefined, place: Place) => {
      for (const id of new Set([...refsOf(rule), ...(extra ?? [])])) {
        let list = map.get(id);
        if (!list) map.set(id, (list = []));
        list.push(place);
      }
    };
    for (const r of this.regions) {
      const page: Page = { kind: "region", name: r.name };
      (r.exits ?? []).forEach((e, index) => add(e.rule, undefined, { label: `${r.name} → ${e.to}`, page, target: { kind: "exit", region: r.name, index } }));
      (r.locations ?? []).forEach((s, index) =>
        add(s.rule, s.temporary_requires, { label: `${this.spotName(s.id)} (${r.name})`, page, target: { kind: "location", region: r.name, index } }),
      );
      (r.events ?? []).forEach((s, index) =>
        add(s.rule, s.temporary_requires, { label: `${this.refName(s.id)} (${r.name})`, page, target: { kind: "event", region: r.name, index } }),
      );
    }
    for (const e of this.file?.entries ?? []) {
      const rule: Rule = { all: [...(e.requires ?? []), ...(e.any_of?.length ? [{ any: e.any_of.map((g) => ({ all: g })) }] : [])] };
      add(rule, undefined, { label: e.id, page: { kind: "entry", id: e.id }, target: { kind: "entry", id: e.id } });
    }
    return map;
  });

  /** Regions grouped by game area for the navigator. */
  groups = $derived.by(() => {
    const game = app.game;
    const AREA_NAMES: Record<string, string> = {
      Town: "Clock Town and Termina Field",
      Swamp: "Southern Swamp and Woodfall",
      Mountain: "Snowhead",
      Ranch: "Romani Ranch",
      Ocean: "Great Bay",
      Canyon: "Ikana",
    };
    const areaOfGameRegion = new Map((game?.regions ?? []).map((r) => [r.id, r.area ?? ""]));
    const areaByName = new Map((game?.regions ?? []).map((r) => [r.name.toLowerCase(), r.area ?? ""]));
    const area = new Map<string, string>();
    // 1. From the name ("Woodfall Temple: Main Room") or the checks inside.
    for (const r of this.regions) {
      const prefix = r.name.split(":")[0].trim().toLowerCase();
      let a = areaByName.get(prefix) ?? areaByName.get(r.name.toLowerCase()) ?? "";
      if (!AREA_NAMES[a]) {
        const counts = new Map<string, number>();
        for (const sp of r.locations ?? []) {
          const ga = areaOfGameRegion.get(app.names.items.get(sp.id)?.region ?? "") ?? "";
          if (AREA_NAMES[ga]) counts.set(ga, (counts.get(ga) ?? 0) + 1);
        }
        a = [...counts.entries()].sort((x, y) => y[1] - x[1])[0]?.[0] ?? "";
      }
      if (AREA_NAMES[a]) area.set(r.name, a);
    }
    // 2. Otherwise, from the regions it connects to.
    for (let pass = 0; pass < 4; pass++) {
      for (const r of this.regions) {
        if (area.has(r.name)) continue;
        const near = [...(r.exits ?? []).map((e) => e.to), ...(this.waysIn.get(r.name) ?? []).map((w) => w.from)];
        const a = near.map((n) => area.get(n)).find(Boolean);
        if (a) area.set(r.name, a);
      }
    }
    const map = new Map<string, LogicRegion[]>();
    for (const r of this.regions) {
      const g = r.name === this.file?.start ? "Start" : (AREA_NAMES[area.get(r.name) ?? ""] ?? "Other places");
      let list = map.get(g);
      if (!list) map.set(g, (list = []));
      list.push(r);
    }
    for (const list of map.values()) list.sort((a, b) => a.name.localeCompare(b.name));
    const order = (g: string) => (g === "Start" ? "0" : g === "Other places" ? "z" : "1" + g);
    return [...map.entries()].sort(([a], [b]) => order(a).localeCompare(order(b)));
  });

  issuesFor = (id: string) => this.issues.filter((i) => i.entry === id);

  // ---- Names ------------------------------------------------------------

  kindOf(id: string): RefKind {
    if (id.startsWith(REGION_PREFIX)) return "region";
    const item = app.names.items.get(id);
    if (item?.name) return "item";
    const entry = this.entryById.get(id);
    if (entry?.trick) return "trick";
    if (entry?.setting) return "setting";
    if (this.eventIds.has(id) || (item && !item.location)) return "event";
    if (entry) return item?.location ? "location" : "helper";
    if (item?.location) return "location";
    return "unknown";
  }

  /** Readable name of a reference. */
  refName(id: string): string {
    if (id.startsWith(REGION_PREFIX)) return id.slice(REGION_PREFIX.length);
    const item = app.names.items.get(id);
    if (item?.name) return item.name;
    return humanize(id);
  }

  /** Readable name of a check. */
  spotName(id: string): string {
    return app.names.items.get(id)?.location ?? humanize(id);
  }

  // ---- Picker options -----------------------------------------------------

  /** Everything a rule can refer to. */
  refOptions = $derived.by((): PickOption[] => {
    const out: PickOption[] = [];
    for (const item of app.game?.items ?? []) if (item.name) out.push({ id: item.id, label: item.name, kind: "item", detail: item.id });
    for (const r of this.regions) out.push({ id: REGION_PREFIX + r.name, label: r.name, kind: "region", detail: "Region" });
    for (const id of this.eventIds) out.push({ id, label: humanize(id), kind: "event", detail: id });
    for (const e of this.file?.entries ?? []) {
      if (app.names.items.get(e.id)?.name) continue;
      const kind = this.kindOf(e.id);
      out.push({ id: e.id, label: humanize(e.id), kind, detail: e.trick?.description ?? (kind === "setting" ? "Setting" : e.id) });
    }
    return out;
  });

  /** Checks (locations) not yet placed in any region. */
  freeLocations = $derived.by((): PickOption[] => {
    const placed = new Set(this.regions.flatMap((r) => (r.locations ?? []).map((s) => s.id)));
    return (app.game?.items ?? [])
      .filter((i) => i.location && !placed.has(i.id) && !this.entryById.has(i.id))
      .map((i) => ({ id: i.id, label: i.location!, kind: "location" as RefKind, detail: i.name ?? i.id }));
  });

  regionOptions = $derived(this.regions.map((r) => ({ id: r.name, label: r.name, kind: "region" as RefKind, detail: "" })));

  // ---- Loading and saving ----------------------------------------------

  async open(source: string) {
    if (this.dirty && !confirm("Discard your unsaved changes?")) return;
    this.loading = true;
    try {
      const r = await backend.logicOpenRegions(source);
      this.file = r.logic;
      this.conversion = r.conversion ? { ...r.conversion, format: r.format } : null;
      if (source.startsWith("bundled:")) {
        const id = source.slice("bundled:".length);
        this.source = { kind: "bundled", id, name: id === "glitched" ? "Glitched logic" : "Casual logic" };
      } else {
        this.source = { kind: "file", path: source };
      }
      this.dirty = r.format !== "regions";
      this.#undo = [];
      this.#redo = [];
      this.#syncLabels();
      this.#history = [];
      this.page = { kind: "region", name: r.logic.start };
      this.target = null;
      this.check();
    } catch (e) {
      toasts.error(e);
    } finally {
      this.loading = false;
    }
  }

  async save(path: string, oldFormat = false) {
    if (!this.file) return;
    try {
      await backend.logicSaveRegions(path, $state.snapshot(this.file) as RegionLogic, oldFormat);
      if (!oldFormat) {
        this.source = { kind: "file", path };
        this.dirty = false;
      }
      toasts.show(oldFormat ? "Exported in the old format" : "Logic saved");
    } catch (e) {
      toasts.error(e);
    }
  }

  #checkTimer: ReturnType<typeof setTimeout> | undefined;
  check() {
    clearTimeout(this.#checkTimer);
    this.#checkTimer = setTimeout(async () => {
      if (!this.file) return;
      try {
        this.issues = await backend.logicCheckRegions($state.snapshot(this.file) as RegionLogic);
      } catch (e) {
        toasts.show(`Couldn't check the logic: ${errorMessage(e)}`, "error");
      }
    }, 400);
  }

  // ---- Navigation -------------------------------------------------------

  go(page: Page, target: Target | null = null) {
    if (this.page) this.#history.push(this.page);
    this.page = page;
    this.target = target ?? (page.kind === "entry" ? { kind: "entry", id: page.id } : null);
  }

  back() {
    const p = this.#history.pop();
    if (p) {
      this.page = p;
      this.target = null;
    }
  }

  /** Open whatever an id refers to (region, helper, trick...). */
  follow(id: string) {
    if (id.startsWith(REGION_PREFIX)) {
      const name = id.slice(REGION_PREFIX.length);
      if (this.regionByName.has(name)) this.go({ kind: "region", name });
      return;
    }
    if (this.entryById.has(id)) {
      this.go({ kind: "entry", id });
      return;
    }
    // A check or event: open its region with it selected.
    for (const r of this.regions) {
      const li = (r.locations ?? []).findIndex((s) => s.id === id);
      if (li >= 0) return this.go({ kind: "region", name: r.name }, { kind: "location", region: r.name, index: li });
      const ei = (r.events ?? []).findIndex((s) => s.id === id);
      if (ei >= 0) return this.go({ kind: "region", name: r.name }, { kind: "event", region: r.name, index: ei });
    }
  }

  // ---- Editing ----------------------------------------------------------

  /** Apply an edit with undo. */
  edit(label: string, f: (logic: RegionLogic) => void) {
    if (!this.file) return;
    this.#undo.push({ label, snapshot: JSON.stringify($state.snapshot(this.file)) });
    if (this.#undo.length > 100) this.#undo.shift();
    this.#redo = [];
    f(this.file);
    this.dirty = true;
    this.#syncLabels();
    this.check();
  }

  undo() {
    const step = this.#undo.pop();
    if (!step || !this.file) return;
    this.#redo.push({ label: step.label, snapshot: JSON.stringify($state.snapshot(this.file)) });
    this.file = JSON.parse(step.snapshot);
    this.dirty = true;
    this.#syncLabels();
    this.#fixSelection();
    this.check();
  }

  redo() {
    const step = this.#redo.pop();
    if (!step || !this.file) return;
    this.#undo.push({ label: step.label, snapshot: JSON.stringify($state.snapshot(this.file)) });
    this.file = JSON.parse(step.snapshot);
    this.dirty = true;
    this.#syncLabels();
    this.#fixSelection();
    this.check();
  }

  #syncLabels() {
    this.undoLabel = this.#undo.at(-1)?.label ?? null;
    this.redoLabel = this.#redo.at(-1)?.label ?? null;
  }

  #fixSelection() {
    if (this.page?.kind === "region" && !this.regionByName.has(this.page.name)) this.page = { kind: "region", name: this.file!.start };
    if (this.page?.kind === "entry" && !this.entryById.has(this.page.id)) this.page = { kind: "region", name: this.file!.start };
    if (this.target && !this.ruleOf(this.target).exists) this.target = null;
  }

  region(name: string): LogicRegion | undefined {
    return this.file?.regions.find((r) => r.name === name);
  }

  /** The spot or exit a target points at (live, editable). */
  spotOf(t: Target): Spot | undefined {
    if (t.kind !== "location" && t.kind !== "event") return undefined;
    return (t.kind === "location" ? this.region(t.region)?.locations : this.region(t.region)?.events)?.[t.index];
  }

  /** The rule a target edits. Entries are shown as one rule. */
  ruleOf(t: Target): { exists: boolean; rule: Rule | undefined } {
    switch (t.kind) {
      case "region":
        return { exists: !!this.region(t.region), rule: undefined };
      case "exit": {
        const e = this.region(t.region)?.exits?.[t.index];
        return { exists: !!e, rule: e?.rule };
      }
      case "location":
      case "event": {
        const s = this.spotOf(t);
        return { exists: !!s, rule: s?.rule };
      }
      case "entry": {
        const e = this.entryById.get(t.id);
        return { exists: !!e, rule: e ? entryRule(e) : undefined };
      }
    }
  }

  setRule(t: Target, rule: Rule) {
    this.edit("Edit requirements", (f) => {
      const region = (name: string) => f.regions.find((r) => r.name === name);
      if (t.kind === "exit") {
        const e = region(t.region)?.exits?.[t.index];
        if (e) e.rule = rule;
      } else if (t.kind === "location" || t.kind === "event") {
        const list = t.kind === "location" ? region(t.region)?.locations : region(t.region)?.events;
        const s = list?.[t.index];
        if (s) s.rule = rule;
      } else if (t.kind === "entry") {
        const e = f.entries.find((x) => x.id === t.id);
        if (e) setEntryRule(e, rule);
      }
    });
  }

  setTime(t: Target, time: Time) {
    this.edit("Edit time of day", (f) => {
      const region = f.regions.find((r) => r.name === (t as { region?: string }).region);
      if (t.kind === "region" && region) region.time = time;
      else if (t.kind === "location" || t.kind === "event") {
        const s = (t.kind === "location" ? region?.locations : region?.events)?.[t.index];
        if (s) s.time = time;
      } else if (t.kind === "entry") {
        const e = f.entries.find((x) => x.id === t.id);
        if (e) e.time = time;
      }
    });
  }

  addExit(regionName: string, to: string) {
    this.edit("Add exit", (f) => {
      const r = f.regions.find((x) => x.name === regionName);
      if (!r) return;
      (r.exits ??= []).push({ to });
      this.target = { kind: "exit", region: regionName, index: r.exits.length - 1 };
    });
  }

  addSpot(regionName: string, kind: "location" | "event", id: string) {
    this.edit(kind === "location" ? "Add check" : "Add event", (f) => {
      const r = f.regions.find((x) => x.name === regionName);
      if (!r) return;
      const list = kind === "location" ? (r.locations ??= []) : (r.events ??= []);
      list.push({ id });
      this.target = { kind, region: regionName, index: list.length - 1 };
    });
  }

  remove(t: Target) {
    this.edit("Delete", (f) => {
      const r = f.regions.find((x) => x.name === (t as { region?: string }).region);
      if (t.kind === "exit") r?.exits?.splice(t.index, 1);
      else if (t.kind === "location") r?.locations?.splice(t.index, 1);
      else if (t.kind === "event") r?.events?.splice(t.index, 1);
      else if (t.kind === "entry") f.entries = f.entries.filter((e) => e.id !== t.id);
      else if (t.kind === "region") f.regions = f.regions.filter((x) => x.name !== t.region);
    });
    this.target = null;
    if (t.kind === "entry" || t.kind === "region") this.page = { kind: "region", name: this.file!.start };
  }

  addRegion(name: string, linkFrom?: string) {
    this.edit("Add region", (f) => {
      f.regions.push({ name });
      if (linkFrom) f.regions.find((r) => r.name === linkFrom)?.exits?.push({ to: name });
    });
    this.go({ kind: "region", name });
  }

  renameRegion(oldName: string, newName: string) {
    if (!newName.trim() || oldName === newName || this.regionByName.has(newName)) return false;
    this.edit("Rename region", (f) => {
      const ref = (x: string) => (x === REGION_PREFIX + oldName ? REGION_PREFIX + newName : x);
      const fix = (r: Rule | undefined): Rule | undefined => {
        if (r === undefined) return r;
        if (typeof r === "string") return ref(r);
        if ("all" in r) return { all: r.all.map((x) => fix(x)!) };
        if ("any" in r) return { any: r.any.map((x) => fix(x)!) };
        return { count: r.count, of: r.of.map(ref) };
      };
      for (const r of f.regions) {
        if (r.name === oldName) r.name = newName;
        for (const e of r.exits ?? []) {
          if (e.to === oldName) e.to = newName;
          e.rule = fix(e.rule);
        }
        for (const s of [...(r.locations ?? []), ...(r.events ?? [])]) s.rule = fix(s.rule);
      }
      if (f.start === oldName) f.start = newName;
      for (const e of f.entries) {
        e.requires = e.requires?.map(ref);
        e.any_of = e.any_of?.map((g) => g.map(ref));
      }
    });
    this.page = { kind: "region", name: newName };
    return true;
  }

  updateEntry(id: string, label: string, f: (e: Entry) => void) {
    this.edit(label, (file) => {
      const e = file.entries.find((x) => x.id === id);
      if (e) f(e);
    });
  }

  addEntry(entry: Entry) {
    this.edit("Add", (f) => {
      f.entries.push(entry);
    });
    this.go({ kind: "entry", id: entry.id });
  }
}

/** An old-style entry (requires + any_of) as one rule. */
export function entryRule(e: Entry): Rule {
  const all: Rule[] = [...(e.requires ?? [])];
  const groups = e.any_of ?? [];
  if (groups.length === 1) all.push(...groups[0]);
  else if (groups.length > 1) all.push({ any: groups.map((g) => (g.length === 1 ? g[0] : { all: g })) });
  return all.length === 1 ? all[0] : { all };
}

/** Alternatives, each a list of ids all needed ("disjunctive normal form").
 * `[]` means impossible, `[[]]` always. Same as the Rust `Rule::dnf`. */
export function dnf(rule: Rule): string[][] {
  if (typeof rule === "string") return [[rule]];
  if ("any" in rule) return rule.any.flatMap(dnf);
  if ("all" in rule) {
    let acc: string[][] = [[]];
    for (const part of rule.all) {
      const alts = dnf(part);
      acc = acc.flatMap((a) => alts.map((b) => [...new Set([...a, ...b])]));
    }
    return acc;
  }
  // N of a list: every way of choosing N.
  const out: string[][] = [];
  const pick = (start: number, chosen: string[]) => {
    if (chosen.length === rule.count) return void out.push(chosen);
    for (let i = start; i < rule.of.length; i++) pick(i + 1, [...chosen, rule.of[i]]);
  };
  pick(0, []);
  return out;
}

/** Store a rule back on an old-style entry, as requires (needed by every
 * alternative) + any_of (the rest of each alternative). Exact. */
export function setEntryRule(e: Entry, rule: Rule) {
  const alts = dnf(rule);
  const common = alts.length ? alts[0].filter((id) => alts.every((a) => a.includes(id))) : [];
  const rest = alts.map((a) => a.filter((id) => !common.includes(id)));
  e.requires = common.length ? common : undefined;
  e.any_of = rest.length && !rest.some((g) => g.length === 0) ? rest : undefined;
  if (alts.length === 0) e.requires = ["(impossible)"];
}

export const doc = new RegionDoc();
