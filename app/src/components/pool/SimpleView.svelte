<script lang="ts">
  // The original randomizer's simple view: one checkbox per classic category.
  import { app } from "../../lib/state.svelte";
  import { facetLabel, groupBy, type Check } from "../../lib/pool";
  import type { PoolLabels } from "./PoolEditor.svelte";

  interface Props {
    checks: Check[];
    selected: Set<string>;
    set: (ids: string[], on: boolean) => void;
    labels: PoolLabels;
  }
  let { checks, selected, set }: Props = $props();

  const groups = $derived(groupBy(checks, "classic").sort(([a], [b]) => facetLabel("classic", a).localeCompare(facetLabel("classic", b))));
  const help = $derived(new Map(app.game!.categories.ClassicCategory.map((c) => [c.id, c.description ?? ""])));
</script>

<div class="simple">
  {#each groups as [cat, list] (cat)}
    {@const on = list.filter((c) => selected.has(c.id)).length}
    <label class="cat" class:any={on > 0} title={help.get(cat) || undefined}>
      <input
        type="checkbox"
        checked={on === list.length}
        indeterminate={on > 0 && on < list.length}
        onchange={(e) => set(list.map((c) => c.id), e.currentTarget.checked)}
      />
      <span class="name">{facetLabel("classic", cat)}</span>
      <span class="n">{on > 0 && on < list.length ? `${on}/` : "+"}{list.length}</span>
    </label>
  {/each}
</div>
<p class="small faint hint">Partly ticked categories have some of their checks changed in another view. Hover a category for details.</p>

<style>
  .simple {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
    gap: 4px 16px;
  }
  .cat {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .cat:hover {
    background: var(--surface-2);
  }
  .name {
    flex: 1;
  }
  .n {
    font-size: 12px;
    color: var(--text-3);
    font-variant-numeric: tabular-nums;
  }
  .cat.any .n {
    color: var(--accent);
  }
  .hint {
    margin-top: 4px;
  }
</style>
