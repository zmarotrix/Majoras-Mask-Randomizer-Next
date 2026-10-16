<script lang="ts">
  // Two searchable lists ("off" and "on"). Select rows (click, Ctrl/Shift for
  // more) and move them across, or double-click a row to move it.
  import { AREAS, matchesQuery, queryWords, typeInfo, type Check } from "../../lib/pool";
  import Icon from "../Icon.svelte";
  import type { PoolLabels } from "./PoolEditor.svelte";

  interface Props {
    checks: Check[];
    selected: Set<string>;
    set: (ids: string[], on: boolean) => void;
    labels: PoolLabels;
    kind: "locations" | "items";
  }
  let { checks, selected, set, labels, kind }: Props = $props();

  type Side = "off" | "on";
  const pane = $state({
    off: { query: "", area: "", picked: new Set<string>(), anchor: null as string | null },
    on: { query: "", area: "", picked: new Set<string>(), anchor: null as string | null },
  });

  const isLocations = $derived(kind === "locations");
  const sorted = $derived(
    [...checks].sort((a, b) => (isLocations ? a.regionName.localeCompare(b.regionName) || a.index - b.index : a.name.localeCompare(b.name))),
  );

  function rowsFor(side: Side): Check[] {
    const p = pane[side];
    const words = queryWords(p.query);
    return sorted.filter((c) => selected.has(c.id) === (side === "on") && (!p.area || c.area === p.area) && matchesQuery(c, words));
  }
  const rows = $derived({ off: rowsFor("off"), on: rowsFor("on") });
  const totals = $derived({ on: checks.filter((c) => selected.has(c.id)).length, off: checks.filter((c) => !selected.has(c.id)).length });

  function click(side: Side, c: Check, e: MouseEvent) {
    const p = pane[side];
    const next = new Set(e.ctrlKey || e.metaKey ? p.picked : []);
    if (e.shiftKey && p.anchor) {
      const list = rows[side];
      const a = list.findIndex((x) => x.id === p.anchor);
      const b = list.findIndex((x) => x.id === c.id);
      if (a >= 0 && b >= 0) for (const x of list.slice(Math.min(a, b), Math.max(a, b) + 1)) next.add(x.id);
    } else if ((e.ctrlKey || e.metaKey) && next.has(c.id)) {
      next.delete(c.id);
    } else {
      next.add(c.id);
      p.anchor = c.id;
    }
    p.picked = next;
  }

  function move(side: Side, ids: string[]) {
    if (!ids.length) return;
    set(ids, side === "off");
    pane[side].picked = new Set();
  }

  function onkey(side: Side, e: KeyboardEvent) {
    if (e.key === "Enter") move(side, [...pane[side].picked]);
    if ((e.ctrlKey || e.metaKey) && e.key === "a") {
      pane[side].picked = new Set(rows[side].map((c) => c.id));
      e.preventDefault();
    }
  }

  const pickedCount = (side: Side) => [...pane[side].picked].filter((id) => rows[side].some((c) => c.id === id)).length;
</script>

<div class="lists">
  {#each ["off", "on"] as const as side (side)}
    {#if side === "on"}
      <div class="middle">
        <button class="btn small" disabled={!pickedCount("off")} onclick={() => move("off", [...pane.off.picked])} title="{labels.add} the selected ones">
          {labels.add} <Icon name="arrow-right" size={14} />
        </button>
        <button class="btn small" disabled={!pickedCount("on")} onclick={() => move("on", [...pane.on.picked])} title="{labels.remove} the selected ones">
          <Icon name="arrow-left" size={14} /> {labels.remove}
        </button>
        <hr />
        <button class="btn ghost small" disabled={!rows.off.length} onclick={() => move("off", rows.off.map((c) => c.id))} title="Move every row shown on the left">
          All shown <Icon name="arrow-right" size={14} />
        </button>
        <button class="btn ghost small" disabled={!rows.on.length} onclick={() => move("on", rows.on.map((c) => c.id))} title="Move every row shown on the right">
          <Icon name="arrow-left" size={14} /> All shown
        </button>
      </div>
    {/if}
    <section class="pane card" class:on-side={side === "on"}>
      <header>
        <strong>{side === "on" ? labels.on : labels.off}</strong>
        <span class="faint small">{totals[side].toLocaleString()}</span>
        {#if pickedCount(side)}<span class="badge accent">{pickedCount(side)} selected</span>{/if}
      </header>
      <div class="tools">
        <div class="search grow"><input class="input" placeholder="Search…" bind:value={pane[side].query} /></div>
        {#if isLocations}
          <select class="input area" bind:value={pane[side].area} aria-label="Area">
            <option value="">All areas</option>
            {#each AREAS as a (a.id)}<option value={a.id}>{a.label}</option>{/each}
          </select>
        {/if}
      </div>
      <div class="rows" role="listbox" aria-multiselectable="true" tabindex="0" onkeydown={(e) => onkey(side, e)}>
        {#each rows[side] as c (c.id)}
          <button
            role="option"
            aria-selected={pane[side].picked.has(c.id)}
            class="r"
            class:picked={pane[side].picked.has(c.id)}
            onclick={(e) => click(side, c, e)}
            ondblclick={() => move(side, [c.id])}
            title="Double-click to move"
          >
            {#if isLocations}<Icon name={typeInfo(c.type).icon} size={14} />{/if}
            <span class="name">{c.name}</span>
            <span class="sub">{isLocations ? `${c.regionName} · ${c.item}` : ""}</span>
          </button>
        {:else}
          <p class="small faint none">{pane[side].query || pane[side].area ? "Nothing matches." : "Empty."}</p>
        {/each}
      </div>
    </section>
  {/each}
</div>
<p class="small faint">Click to select, Ctrl or Shift + click for more, double-click or Enter to move.</p>

<style>
  .lists {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
    gap: 10px;
    height: 62vh;
    min-height: 360px;
  }
  .pane {
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 10px;
    gap: 8px;
    box-shadow: none;
  }
  .pane.on-side {
    border-color: var(--accent);
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .tools {
    display: flex;
    gap: 6px;
  }
  .grow {
    flex: 1;
  }
  .area {
    width: 150px;
  }
  .rows {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--border);
    padding-top: 4px;
    outline: none;
  }
  .r {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    grid-template-rows: auto auto;
    column-gap: 8px;
    align-items: center;
    padding: 5px 8px;
    border: none;
    border-radius: 4px;
    background: none;
    text-align: left;
    user-select: none;
  }
  .r :global(.icon) {
    grid-row: span 2;
    color: var(--text-3);
  }
  .r:hover {
    background: var(--surface-2);
  }
  .r.picked {
    background: var(--accent-soft);
  }
  .r.picked .name {
    color: var(--accent);
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
  }
  .sub {
    grid-column: 2;
    font-size: 11.5px;
    color: var(--text-3);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sub:empty {
    display: none;
  }
  .middle {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 6px;
  }
  .middle hr {
    border: none;
    border-top: 1px solid var(--border);
    margin: 6px 0;
  }
  .none {
    padding: 10px 8px;
  }
</style>
