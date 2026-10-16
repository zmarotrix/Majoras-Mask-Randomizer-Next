<script lang="ts">
  // Advanced view: a matrix of two facets. Click a cell, a row or a column to
  // toggle every check in it.
  import { app } from "../../lib/state.svelte";
  import { FACET_LABELS, facetLabel, facetValue, groupBy, typeInfo, type Check, type Facet } from "../../lib/pool";
  import Icon from "../Icon.svelte";
  import type { PoolLabels } from "./PoolEditor.svelte";

  interface Props {
    checks: Check[];
    selected: Set<string>;
    set: (ids: string[], on: boolean) => void;
    labels: PoolLabels;
  }
  let { checks, selected, set, labels }: Props = $props();

  const LAYOUTS: { label: string; rows: Facet; cols: Facet }[] = [
    { label: "Check type × area", rows: "type", cols: "area" },
    { label: "Vanilla item × check type (original)", rows: "itemCat", cols: "type" },
  ];
  const AXES: Facet[] = ["type", "area", "itemCat", "classic"];

  function load(): { rows: Facet; cols: Facet } {
    try {
      const v = JSON.parse(localStorage.getItem("mmr.gridAxes") ?? "null");
      if (v && AXES.includes(v.rows) && AXES.includes(v.cols) && v.rows !== v.cols) return v;
    } catch {}
    return LAYOUTS[0];
  }
  const initial = load();
  let rows = $state<Facet>(initial.rows);
  let cols = $state<Facet>(initial.cols);

  $effect(() => {
    try {
      localStorage.setItem("mmr.gridAxes", JSON.stringify({ rows, cols }));
    } catch {}
  });

  function setAxis(which: "rows" | "cols", f: Facet) {
    if (which === "rows") {
      if (f === cols) cols = rows;
      rows = f;
    } else {
      if (f === rows) rows = cols;
      cols = f;
    }
  }

  const rowGroups = $derived(groupBy(checks, rows));
  const colGroups = $derived(groupBy(checks, cols));
  const cells = $derived.by(() => {
    const m = new Map<string, Check[]>();
    for (const c of checks) {
      const k = `${facetValue(c, rows)}|${facetValue(c, cols)}`;
      let l = m.get(k);
      if (!l) m.set(k, (l = []));
      l.push(c);
    }
    return m;
  });
  const vertical = $derived(colGroups.length > 8);

  function toggle(list: Check[]) {
    const allOn = list.every((c) => selected.has(c.id));
    set(
      list.map((c) => c.id),
      !allOn,
    );
  }

  function cellState(list: Check[] | undefined): "all" | "some" | "none" | "empty" {
    if (!list?.length) return "empty";
    const n = list.filter((c) => selected.has(c.id)).length;
    return n === list.length ? "all" : n ? "some" : "none";
  }

  const label = (f: Facet, v: string) => facetLabel(f, v, app.names.regions);
</script>

<div class="axes">
  <label class="row small">
    Rows
    <select class="input" value={rows} onchange={(e) => setAxis("rows", e.currentTarget.value as Facet)}>
      {#each AXES as a (a)}<option value={a}>{FACET_LABELS[a]}</option>{/each}
    </select>
  </label>
  <label class="row small">
    Columns
    <select class="input" value={cols} onchange={(e) => setAxis("cols", e.currentTarget.value as Facet)}>
      {#each AXES as a (a)}<option value={a}>{FACET_LABELS[a]}</option>{/each}
    </select>
  </label>
  <span class="spacer"></span>
  {#each LAYOUTS as l (l.label)}
    <button class="btn ghost small" class:active={rows === l.rows && cols === l.cols} onclick={() => ((rows = l.rows), (cols = l.cols))}>{l.label}</button>
  {/each}
</div>

<div class="scroller">
  <table class:vertical>
    <thead>
      <tr>
        <th class="corner"></th>
        {#each colGroups as [cv, clist] (cv)}
          <th class="col state-{cellState(clist)}">
            <button onclick={() => toggle(clist)} title="Toggle all {label(cols, cv)} ({clist.length})">
              <span>{label(cols, cv)}</span>
            </button>
          </th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#each rowGroups as [rv, rlist] (rv)}
        <tr>
          <th class="rowh state-{cellState(rlist)}">
            <button onclick={() => toggle(rlist)} title="Toggle all {label(rows, rv)} ({rlist.length})">
              {#if rows === "type"}<Icon name={typeInfo(rv).icon} size={14} />{/if}
              <span>{label(rows, rv)}</span>
              <span class="faint">{rlist.length}</span>
            </button>
          </th>
          {#each colGroups as [cv] (cv)}
            {@const list = cells.get(`${rv}|${cv}`)}
            {@const s = cellState(list)}
            <td>
              {#if list}
                {@const n = list.filter((c) => selected.has(c.id)).length}
                <button
                  class="cell {s}"
                  onclick={() => toggle(list)}
                  title="{label(rows, rv)} · {label(cols, cv)}: {n} of {list.length} {labels.on.toLowerCase()}"
                >
                  {s === "some" ? `${n}/${list.length}` : list.length}
                </button>
              {/if}
            </td>
          {/each}
        </tr>
      {/each}
    </tbody>
  </table>
</div>
<div class="legend small muted">
  <span><span class="sw all"></span>{labels.on}</span>
  <span><span class="sw some"></span>Some</span>
  <span><span class="sw none"></span>{labels.off}</span>
  <span class="faint">Click a cell, row or column to toggle it.</span>
</div>

<style>
  .axes {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }
  .axes select {
    width: auto;
    padding: 4px 8px;
  }
  .axes .active {
    color: var(--accent);
    background: var(--accent-soft);
  }
  .scroller {
    overflow: auto;
    max-height: 62vh;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
  }
  table {
    border-collapse: separate;
    border-spacing: 3px;
    padding: 4px;
  }
  th {
    font-weight: 500;
    font-size: 12px;
    color: var(--text-2);
  }
  th button {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    border: none;
    background: none;
    padding: 3px 4px;
    border-radius: 4px;
    color: inherit;
    font: inherit;
    text-align: left;
    white-space: nowrap;
  }
  th button:hover {
    color: var(--accent);
    background: var(--accent-soft);
  }
  thead th {
    position: sticky;
    top: 0;
    background: var(--surface);
    z-index: 2;
    vertical-align: bottom;
  }
  .rowh {
    position: sticky;
    left: 0;
    background: var(--surface);
    z-index: 1;
    min-width: 170px;
  }
  .rowh button span:nth-child(2),
  .rowh button span:first-child:not(:only-child) {
    flex: 1;
  }
  .corner {
    left: 0;
    z-index: 3;
  }
  .vertical .col button {
    writing-mode: vertical-rl;
    transform: rotate(180deg);
    height: 120px;
    justify-content: flex-start;
    padding: 4px 2px;
  }
  .col.state-all,
  .rowh.state-all {
    color: var(--accent);
  }
  td {
    padding: 0;
  }
  .cell {
    width: 100%;
    min-width: 44px;
    height: 28px;
    border-radius: 5px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text-3);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
  }
  .cell:hover {
    border-color: var(--accent);
  }
  .cell.all {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-text);
  }
  .cell.some {
    background: var(--accent-soft);
    border-color: var(--accent);
    color: var(--accent);
  }
  .legend {
    display: flex;
    gap: 16px;
    align-items: center;
  }
  .sw {
    display: inline-block;
    width: 14px;
    height: 10px;
    border-radius: 3px;
    margin-right: 5px;
    vertical-align: -1px;
    border: 1px solid var(--border-strong);
  }
  .sw.all {
    background: var(--accent);
    border-color: var(--accent);
  }
  .sw.some {
    background: var(--accent-soft);
    border-color: var(--accent);
  }
</style>
