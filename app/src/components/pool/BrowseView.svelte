<script lang="ts">
  // Search and filter checks, shown as toggle chips in cards (by region or
  // another facet). Bulk actions apply to everything that matches.
  import { app } from "../../lib/state.svelte";
  import {
    AREAS,
    collapseRuns,
    facetLabel,
    groupBy,
    isRun,
    matchesQuery,
    queryWords,
    shortName,
    typeInfo,
    type Check,
    type Facet,
    type Run,
  } from "../../lib/pool";
  import Icon from "../Icon.svelte";
  import type { PoolLabels } from "./PoolEditor.svelte";

  interface Props {
    checks: Check[];
    selected: Set<string>;
    set: (ids: string[], on: boolean) => void;
    labels: PoolLabels;
    group: Facet;
  }
  let { checks, selected, set, labels, group }: Props = $props();

  let query = $state("");
  let types = $state(new Set<string>());
  let areas = $state(new Set<string>());
  let itemCats = $state(new Set<string>());
  let status = $state<"all" | "on" | "off">("all");
  let expanded = $state(new Set<string>());
  let showItemMenu = $state(false);

  const byLocation = $derived(group === "region");

  const typeCounts = $derived(groupBy(checks, "type"));
  const itemCatCounts = $derived(groupBy(checks, "itemCat").sort((a, b) => b[1].length - a[1].length));
  const areaCounts = $derived(new Map(groupBy(checks, "area")));

  const matching = $derived.by(() => {
    const words = queryWords(query);
    return checks.filter(
      (c) =>
        matchesQuery(c, words) &&
        (!types.size || types.has(c.type)) &&
        (!areas.size || areas.has(c.area)) &&
        (!itemCats.size || itemCats.has(c.itemCat)) &&
        (status === "all" || (status === "on") === selected.has(c.id)),
    );
  });
  const matchingOn = $derived(matching.filter((c) => selected.has(c.id)).length);
  const cards = $derived(groupBy(matching, group));
  const filtering = $derived(queryWords(query).length > 0 || types.size + areas.size + itemCats.size > 0 || status !== "all");

  function flip(s: Set<string>, v: string): Set<string> {
    const n = new Set(s);
    n.has(v) ? n.delete(v) : n.add(v);
    return n;
  }

  function clearFilters() {
    query = "";
    types = new Set();
    areas = new Set();
    itemCats = new Set();
    status = "all";
  }

  function toggleRun(run: Run) {
    const all = run.checks.every((c) => selected.has(c.id));
    set(
      run.checks.map((c) => c.id),
      !all,
    );
  }

  function cardTitle(v: string) {
    return facetLabel(group, v, app.names.regions);
  }
</script>

<div class="filters">
  <div class="row top">
    <div class="search grow"><input class="input" placeholder="Search names, items or places: heart, palace, pot…" bind:value={query} /></div>
    <div class="status" role="radiogroup" aria-label="Show">
      {#each [["all", "All"], ["on", labels.on], ["off", labels.off]] as [v, l] (v)}
        <button role="radio" aria-checked={status === v} class:on={status === v} onclick={() => (status = v as typeof status)}>{l}</button>
      {/each}
    </div>
  </div>

  {#if byLocation}
    <div class="facet">
      <span class="flabel">Area</span>
      {#each AREAS.filter((a) => areaCounts.has(a.id)) as a (a.id)}
        <button class="f" class:on={areas.has(a.id)} onclick={() => (areas = flip(areas, a.id))}>
          {a.label}
        </button>
      {/each}
    </div>
    <div class="facet">
      <span class="flabel">Check</span>
      {#each typeCounts as [t, list] (t)}
        <button class="f" class:on={types.has(t)} onclick={() => (types = flip(types, t))} title="{list.length} checks">
          <Icon name={typeInfo(t).icon} size={13} />
          {typeInfo(t).label}
        </button>
      {/each}
    </div>
  {/if}
  <div class="facet">
    <span class="flabel">{byLocation ? "Vanilla item" : "Item type"}</span>
    {#each itemCatCounts.slice(0, 10) as [ic] (ic)}
      <button class="f" class:on={itemCats.has(ic)} onclick={() => (itemCats = flip(itemCats, ic))}>{facetLabel("itemCat", ic)}</button>
    {/each}
    {#if itemCatCounts.length > 10}
      <div class="more">
        <button class="f" onclick={() => (showItemMenu = !showItemMenu)}>
          More{#if [...itemCats].some((i) => !itemCatCounts.slice(0, 10).some(([x]) => x === i))} ({itemCats.size}){/if}
          <Icon name="chevron-down" size={12} />
        </button>
        {#if showItemMenu}
          <div class="menu card">
            {#each itemCatCounts.slice(10) as [ic, list] (ic)}
              <label class="mi">
                <input type="checkbox" checked={itemCats.has(ic)} onchange={() => (itemCats = flip(itemCats, ic))} />
                <span>{facetLabel("itemCat", ic)}</span>
                <span class="faint small">{list.length}</span>
              </label>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<div class="bulk">
  <span class="grow">
    <strong>{matching.length.toLocaleString()}</strong> {filtering ? "matching" : ""} {labels.noun} · {matchingOn.toLocaleString()} {labels.on.toLowerCase()}
    {#if filtering}<button class="link" onclick={clearFilters}>Clear filters</button>{/if}
  </span>
  <button
    class="btn small"
    disabled={matchingOn === matching.length}
    onclick={() =>
      set(
        matching.map((c) => c.id),
        true,
      )}>{labels.add} {filtering ? "all matching" : "all"}</button
  >
  <button
    class="btn small"
    disabled={matchingOn === 0}
    onclick={() =>
      set(
        matching.map((c) => c.id),
        false,
      )}>{labels.remove} {filtering ? "all matching" : "all"}</button
  >
</div>

<div class="cards">
  {#each cards as [g, list] (g)}
    {@const on = list.filter((c) => selected.has(c.id)).length}
    <section class="card group">
      <header>
        <span class="gname">{cardTitle(g)}</span>
        <span class="faint small">{on}/{list.length}</span>
        <button
          class="icon-btn"
          title={on === list.length ? `${labels.remove} all shown here` : `${labels.add} all shown here`}
          onclick={() =>
            set(
              list.map((c) => c.id),
              on !== list.length,
            )}
        >
          <Icon name={on === list.length ? "x" : "check"} size={14} />
        </button>
      </header>
      <div class="chips">
        {#each collapseRuns(list) as x (isRun(x) ? x.key : x.id)}
          {#if isRun(x) && !expanded.has(x.key)}
            {@const n = x.checks.filter((c) => selected.has(c.id)).length}
            <span class="chip run" class:on={n === x.checks.length} class:some={n > 0 && n < x.checks.length}>
              <button class="main" onclick={() => toggleRun(x)} title="{x.checks.length} similar checks: {x.label}">
                <Icon name={typeInfo(x.checks[0].type).icon} size={13} />
                {byLocation ? shortName(x.label, x.checks[0].regionName) : x.label}
                <span class="times">×{x.checks.length}</span>
              </button>
              <button class="exp" title="Show each one" onclick={() => (expanded = flip(expanded, x.key))}>
                <Icon name="chevron-right" size={12} />
              </button>
            </span>
          {:else}
            {#each isRun(x) ? x.checks : [x] as c (c.id)}
              <button
                class="chip"
                class:on={selected.has(c.id)}
                onclick={() => set([c.id], !selected.has(c.id))}
                title="{c.name}{byLocation ? ` · normally: ${c.item} · ${typeInfo(c.type).label}` : ''}"
              >
                {#if byLocation}<Icon name={typeInfo(c.type).icon} size={13} />{/if}
                <span>{byLocation ? shortName(c.name, c.regionName) : c.name}</span>
                {#if byLocation}<span class="v">{c.item}</span>{/if}
              </button>
            {/each}
            {#if isRun(x)}
              <button class="chip collapse" onclick={() => (expanded = flip(expanded, x.key))} title="Group these again">
                <Icon name="chevron-down" size={12} /> Group
              </button>
            {/if}
          {/if}
        {/each}
      </div>
    </section>
  {:else}
    <p class="muted empty">Nothing matches. Try fewer filters.</p>
  {/each}
</div>

<style>
  .filters {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .top {
    gap: 10px;
  }
  .grow {
    flex: 1;
  }
  .status {
    display: inline-flex;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }
  .status button {
    border: none;
    border-right: 1px solid var(--border-strong);
    background: var(--surface);
    padding: 6px 12px;
    font-size: 13px;
  }
  .status button:last-child {
    border-right: none;
  }
  .status button.on {
    background: var(--accent);
    color: var(--accent-text);
  }
  .facet {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
  }
  .flabel {
    width: 86px;
    font-size: 12px;
    color: var(--text-3);
  }
  .f {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 3px 10px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text-2);
    font-size: 12.5px;
  }
  .f:hover {
    border-color: var(--border-strong);
    color: var(--text);
  }
  .f.on {
    background: var(--accent-soft);
    border-color: var(--accent);
    color: var(--accent);
  }
  .more {
    position: relative;
  }
  .menu {
    position: absolute;
    z-index: 20;
    top: calc(100% + 4px);
    left: 0;
    min-width: 230px;
    max-height: 300px;
    overflow: auto;
    padding: 4px;
    box-shadow: var(--shadow-lg);
  }
  .mi {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 6px;
    border-radius: 4px;
    font-size: 13px;
  }
  .mi span:nth-child(2) {
    flex: 1;
  }
  .mi:hover {
    background: var(--surface-2);
  }
  .bulk {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text-2);
  }
  .bulk strong {
    color: var(--text);
  }
  .link {
    margin-left: 8px;
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
    font-size: 13px;
  }
  .cards {
    columns: 2 360px;
    column-gap: 12px;
  }
  .group {
    break-inside: avoid;
    margin-bottom: 12px;
    padding: 10px 12px 12px;
    box-shadow: none;
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }
  .gname {
    flex: 1;
    font-weight: 600;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    max-width: 100%;
    padding: 3px 8px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--text-2);
    font-size: 12.5px;
    text-align: left;
  }
  .chip:hover {
    border-color: var(--accent);
  }
  .chip span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .chip .v {
    color: var(--text-3);
    font-size: 11.5px;
  }
  .chip.on {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-text);
  }
  .chip.on .v {
    color: inherit;
    opacity: 0.75;
  }
  .chip.some {
    background: var(--accent-soft);
    border-color: var(--accent);
    color: var(--accent);
  }
  .chip.run {
    padding: 0;
    gap: 0;
  }
  .chip.run .main {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 3px 6px 3px 8px;
    border: none;
    background: none;
    color: inherit;
    font: inherit;
  }
  .chip.run .exp {
    display: inline-grid;
    place-items: center;
    align-self: stretch;
    padding: 0 6px;
    border: none;
    border-left: 1px solid currentColor;
    background: none;
    color: inherit;
    opacity: 0.7;
  }
  .times {
    font-weight: 600;
  }
  .chip.collapse {
    border-style: dashed;
    color: var(--text-3);
  }
  .empty {
    padding: 20px 0;
  }
</style>
