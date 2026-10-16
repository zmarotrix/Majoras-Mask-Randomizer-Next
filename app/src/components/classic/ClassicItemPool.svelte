<script lang="ts">
  // MMR 1.16's Item Randomization tab: list code + Edit, the category
  // checkboxes, and the Advanced View grid.
  import Modal from "../Modal.svelte";
  import PoolEditor from "../pool/PoolEditor.svelte";
  import SimpleView from "../pool/SimpleView.svelte";
  import GridView from "../pool/GridView.svelte";
  import { app, toasts } from "../../lib/state.svelte";
  import { decodeList, encodeList } from "../../lib/legacyList";
  import { locationChecks } from "../../lib/pool";

  const LABELS = { on: "Shuffled", off: "Vanilla", add: "Shuffle", remove: "Make vanilla", noun: "checks" };

  let advanced = $state(false);
  let editing = $state(false);

  const checks = $derived(locationChecks(app.game!));
  const value = $derived((app.values.CustomItemList as string[] | undefined) ?? []);
  const selected = $derived(new Set(value));
  const code = $derived(encodeList(app.game!, "locations", value));

  function set(ids: string[], on: boolean) {
    const next = new Set(value);
    for (const i of ids) on ? next.add(i) : next.delete(i);
    const order = new Map(checks.map((c, i) => [c.id, i]));
    app.setValue("CustomItemList", [...next].sort((a, b) => (order.get(a) ?? 1e9) - (order.get(b) ?? 1e9)));
  }

  function pasteCode(text: string) {
    const items = decodeList(app.game!, "locations", text);
    if (items) app.setValue("CustomItemList", items);
    else toasts.show("That list code is from a different randomizer version, so it can't be read.", "error");
  }
</script>

<div class="pool" onmouseenter={() => (app.help = "CustomItemList")} role="group" aria-label="Item Randomization">
  <div class="line">
    <input class="input grow mono" value={code} onchange={(e) => pasteCode(e.currentTarget.value)} aria-label="List code" spellcheck="false" />
    <button class="btn small" onclick={() => (editing = true)}>Edit</button>
  </div>
  <span class="small muted">{selected.size}/{checks.length} items randomized</span>
  <label class="check">
    <input type="checkbox" bind:checked={advanced} />
    Advanced View
  </label>
  {#if advanced}
    <GridView {checks} {selected} {set} labels={LABELS} />
  {:else}
    <SimpleView {checks} {selected} {set} labels={LABELS} />
  {/if}
</div>

{#if editing}
  <Modal title="Item Randomization" subtitle="Choose exactly which checks are shuffled." wide onclose={() => (editing = false)}>
    <PoolEditor
      id="classic-shuffled"
      kind="locations"
      {checks}
      {value}
      onchange={(v) => app.setValue("CustomItemList", v)}
      views={["browse", "lists", "grid", "simple"]}
      labels={LABELS}
    />
    {#snippet footer()}
      <button class="btn primary" onclick={() => (editing = false)}>Done</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .pool {
    display: flex;
    flex-direction: column;
    gap: 8px;
    grid-area: pool;
    min-width: 0;
  }
  .line {
    display: flex;
    gap: 6px;
  }
  .grow {
    flex: 1;
    min-width: 0;
    padding: 4px 8px;
  }
  .mono {
    font-family: var(--mono);
    font-size: 12px;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 7px;
    cursor: pointer;
  }
  .check input {
    width: 15px;
    height: 15px;
    margin: 0;
    accent-color: var(--accent);
  }
</style>
