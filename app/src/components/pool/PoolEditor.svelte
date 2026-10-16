<script lang="ts" module>
  export type PoolView = "simple" | "grid" | "browse" | "lists";

  export interface PoolLabels {
    /** e.g. "Shuffled" / "Junk" / "Starting with" */
    on: string;
    /** e.g. "Vanilla" / "Can hold anything" / "Not starting with" */
    off: string;
    /** Bulk actions, e.g. "Shuffle" / "Make vanilla". */
    add: string;
    remove: string;
    /** "checks" or "items" */
    noun: string;
  }
</script>

<script lang="ts">
  import Icon from "../Icon.svelte";
  import SimpleView from "./SimpleView.svelte";
  import GridView from "./GridView.svelte";
  import BrowseView from "./BrowseView.svelte";
  import ListsView from "./ListsView.svelte";
  import type { Check } from "../../lib/pool";

  interface Props {
    /** Used to remember the chosen view. */
    id: string;
    checks: Check[];
    value: string[];
    onchange: (value: string[]) => void;
    views: PoolView[];
    labels: PoolLabels;
    /** Locations have areas, regions and check types; plain items don't. */
    kind: "locations" | "items";
  }
  let { id, checks, value, onchange, views, labels, kind }: Props = $props();

  const VIEW_INFO: Record<PoolView, { label: string; icon: string; help: string }> = {
    simple: { label: "Simple", icon: "checks", help: "Toggle whole categories, like the original randomizer." },
    grid: { label: "Grid", icon: "grid", help: "Toggle groups at the crossing of two categories." },
    browse: { label: "Browse", icon: "tags", help: "Search and filter, then toggle single checks or everything that matches." },
    lists: { label: "Lists", icon: "columns", help: "Two searchable lists; move checks between them." },
  };

  const storageKey = $derived(`mmr.poolView.${id}`);
  function initialView(): PoolView {
    try {
      const v = localStorage.getItem(`mmr.poolView.${id}`) as PoolView | null;
      if (v && views.includes(v)) return v;
    } catch {}
    return views[0];
  }
  // svelte-ignore state_referenced_locally (initial view only)
  let view = $state<PoolView>(initialView());

  function choose(v: PoolView) {
    view = v;
    try {
      localStorage.setItem(storageKey, v);
    } catch {}
  }

  const selected = $derived(new Set(value));
  const total = $derived(checks.length);
  const count = $derived(checks.filter((c) => selected.has(c.id)).length);

  /** Turn a set of checks on or off, keeping game-data order. */
  function set(ids: string[], on: boolean) {
    const next = new Set(value);
    for (const i of ids) on ? next.add(i) : next.delete(i);
    const order = new Map(checks.map((c, i) => [c.id, i]));
    onchange([...next].sort((a, b) => (order.get(a) ?? 1e9) - (order.get(b) ?? 1e9)));
  }
</script>

<div class="editor">
  <div class="head">
    <div class="count">
      <strong>{count.toLocaleString()}</strong> of {total.toLocaleString()} {labels.noun} {labels.on.toLowerCase()}
    </div>
    {#if views.length > 1}
      <div class="views" role="tablist" aria-label="Editor view">
        {#each views as v (v)}
          <button role="tab" aria-selected={view === v} class:on={view === v} title={VIEW_INFO[v].help} onclick={() => choose(v)}>
            <Icon name={VIEW_INFO[v].icon} size={14} />
            {VIEW_INFO[v].label}
          </button>
        {/each}
      </div>
    {/if}
  </div>

  {#if view === "simple"}
    <SimpleView {checks} {selected} {set} {labels} />
  {:else if view === "grid"}
    <GridView {checks} {selected} {set} {labels} />
  {:else if view === "browse"}
    <BrowseView {checks} {selected} {set} {labels} group={kind === "locations" ? "region" : "itemCat"} />
  {:else}
    <ListsView {checks} {selected} {set} {labels} {kind} />
  {/if}
</div>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .count {
    color: var(--text-2);
  }
  .count strong {
    color: var(--text);
    font-size: 15px;
  }
  .views {
    display: inline-flex;
    padding: 3px;
    gap: 2px;
    border-radius: 8px;
    background: var(--surface-3);
  }
  .views button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 12px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text-2);
    font-size: 13px;
    font-weight: 500;
  }
  .views button:hover {
    color: var(--text);
  }
  .views button.on {
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow);
  }
</style>
