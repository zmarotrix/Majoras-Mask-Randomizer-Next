<script lang="ts">
  // Custom hint priorities (MMR 2.0's "Customize Hint Priorities"): ranked
  // tiers of locations. Each tier can show its importance, and can cap how
  // many of its locations are hinted (the rest are forced to be junk).
  import Modal from "../Modal.svelte";
  import PoolEditor from "../pool/PoolEditor.svelte";
  import Icon from "../Icon.svelte";
  import { app } from "../../lib/state.svelte";
  import { copy } from "../../lib/colors";
  import { locationChecks } from "../../lib/pool";

  const TIERS = "OverrideHintPriorities";
  const IMPORTANCE = "OverrideImportanceIndicatorTiers";
  const CAPS = "OverrideHintItemCaps";

  const tiers = $derived((app.values[TIERS] as string[][] | null) ?? null);
  const importance = $derived(new Set((app.values[IMPORTANCE] as number[] | null) ?? []));
  const caps = $derived((app.values[CAPS] as number[] | null) ?? []);
  const checks = $derived(locationChecks(app.game!));
  const name = (id: string) => app.names.items.get(id)?.location ?? id;
  let editing = $state<number | null>(null);

  /** Save all three settings together, keeping tier indexes in step. */
  function save(next: { tiers: string[][]; importance: boolean[]; caps: number[] } | null) {
    if (!next) {
      app.setValue(TIERS, null);
      app.setValue(IMPORTANCE, null);
      app.setValue(CAPS, null);
      return;
    }
    app.setValue(TIERS, next.tiers);
    app.setValue(IMPORTANCE, next.importance.flatMap((on, i) => (on ? [i] : [])));
    app.setValue(CAPS, next.caps);
  }
  function current() {
    const t = copy(tiers ?? []);
    return { tiers: t, importance: t.map((_, i) => importance.has(i)), caps: t.map((_, i) => caps[i] ?? 0) };
  }
  function edit(f: (c: ReturnType<typeof current>) => void) {
    const c = current();
    f(c);
    save(c);
  }
  function move(i: number, by: number) {
    edit((c) => {
      for (const list of [c.tiers, c.importance, c.caps] as unknown[][]) {
        const [x] = list.splice(i, 1);
        list.splice(i + by, 0, x);
      }
    });
  }
  function remove(i: number) {
    edit((c) => {
      for (const list of [c.tiers, c.importance, c.caps] as unknown[][]) list.splice(i, 1);
    });
  }
  function add() {
    edit((c) => {
      c.tiers.push([]);
      c.importance.push(false);
      c.caps.push(0);
    });
    editing = (tiers?.length ?? 0);
  }
</script>

<div class="hp" onmouseenter={() => (app.help = TIERS)} role="group" aria-label="Hint priorities">
  <label class="check">
    <input type="checkbox" checked={tiers !== null} onchange={(e) => save(e.currentTarget.checked ? { tiers: [[]], importance: [false], caps: [0] } : null)} />
    Use my own hint priorities
  </label>
  {#if tiers === null}
    <p class="small muted">Off: hints use the randomizer's built-in priorities.</p>
  {:else}
    <p class="small muted">Higher tiers are hinted first. Locations in the same tier are picked at random.</p>
    {#each tiers as tier, i (i)}
      <div class="tier">
        <span class="n">{i + 1}</span>
        <span class="locs small" title={tier.map(name).join(", ")}>
          {#if tier.length}{tier.slice(0, 3).map(name).join(", ")}{tier.length > 3 ? ` and ${tier.length - 3} more` : ""}{:else}<em class="faint">no locations</em>{/if}
        </span>
        <button class="btn small" onclick={() => (editing = i)}>Locations</button>
        <label class="check small" title="Hints for this tier say how important the item is">
          <input type="checkbox" checked={importance.has(i)} onchange={(e) => edit((c) => (c.importance[i] = e.currentTarget.checked))} />
          Importance
        </label>
        <label class="cap small" title="How many locations in this tier are hinted (0 = all); the rest are forced to be junk">
          Cap
          <input class="input" type="number" min="0" value={caps[i] ?? 0} onchange={(e) => edit((c) => (c.caps[i] = Math.max(0, Number(e.currentTarget.value) || 0)))} />
        </label>
        <span class="order">
          <button class="icon-btn" aria-label="Move up" disabled={i === 0} onclick={() => move(i, -1)}><Icon name="chevron-down" size={14} /></button>
          <button class="icon-btn down" aria-label="Move down" disabled={i === tiers.length - 1} onclick={() => move(i, 1)}><Icon name="chevron-down" size={14} /></button>
          <button class="icon-btn" aria-label="Remove tier" onclick={() => remove(i)}><Icon name="x" size={14} /></button>
        </span>
      </div>
    {/each}
    <button class="btn small add" onclick={add}>Add a tier</button>
  {/if}
</div>

{#if editing !== null && tiers?.[editing]}
  <Modal title="Tier {editing + 1} locations" wide onclose={() => (editing = null)}>
    <PoolEditor
      id="hint-tier"
      kind="locations"
      {checks}
      value={tiers[editing]}
      onchange={(v) => edit((c) => (c.tiers[editing!] = v))}
      views={["browse", "lists"]}
      labels={{ on: "In this tier", off: "Not in this tier", add: "Add", remove: "Remove", noun: "locations" }}
    />
    {#snippet footer()}
      <button class="btn primary" onclick={() => (editing = null)}>Done</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .hp {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .hp p {
    margin: 0;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
  }
  .check input {
    accent-color: var(--accent);
  }
  .tier {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
  }
  .n {
    display: inline-grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 700;
    font-size: 12px;
  }
  .locs {
    flex: 1;
    min-width: 120px;
    color: var(--text-2);
  }
  .cap {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .cap input {
    width: 52px;
    padding: 2px 6px;
  }
  .order {
    display: flex;
  }
  .order button:first-child :global(svg) {
    transform: rotate(180deg);
  }
  .add {
    align-self: flex-start;
  }
</style>
