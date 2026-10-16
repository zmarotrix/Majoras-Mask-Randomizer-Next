<script lang="ts">
  // Random starting item groups: "start with N random items from this list".
  import Modal from "../Modal.svelte";
  import PoolEditor from "../pool/PoolEditor.svelte";
  import Icon from "../Icon.svelte";
  import { app } from "../../lib/state.svelte";
  import { copy } from "../../lib/colors";
  import { startingItemChecks } from "../../lib/pool";

  const KEY = "RandomStartingItemGroups";
  type Group = { Items: string[]; Amount: number };

  const groups = $derived((app.values[KEY] as Group[] | null) ?? []);
  const checks = $derived(startingItemChecks(app.game!));
  const name = (id: string) => app.names.items.get(id)?.name ?? id;
  let editing = $state<number | null>(null);

  function save(next: Group[]) {
    app.setValue(KEY, next);
  }
  function update(i: number, edit: (g: Group) => void) {
    const next = copy(groups);
    edit(next[i]);
    next[i].Amount = Math.max(1, Math.min(next[i].Amount, Math.max(1, next[i].Items.length)));
    save(next);
  }
  function add() {
    save([...copy(groups), { Items: [], Amount: 1 }]);
    editing = groups.length;
  }
  function remove(i: number) {
    save(groups.filter((_, j) => j !== i));
  }
</script>

<div class="groups" onmouseenter={() => (app.help = KEY)} role="group" aria-label="Random starting item groups">
  {#each groups as g, i (i)}
    <div class="group">
      <span class="amount">
        Start with
        <input
          class="input"
          type="number"
          min="1"
          max={Math.max(1, g.Items.length)}
          value={g.Amount}
          onchange={(e) => update(i, (x) => (x.Amount = Number(e.currentTarget.value)))}
          aria-label="How many"
        />
        of
      </span>
      <span class="items small" title={g.Items.map(name).join(", ")}>
        {#if g.Items.length}{g.Items.slice(0, 4).map(name).join(", ")}{g.Items.length > 4 ? ` and ${g.Items.length - 4} more` : ""}{:else}<em class="faint"
            >no items yet</em
          >{/if}
      </span>
      <button class="btn small" onclick={() => (editing = i)}>Edit items</button>
      <button class="icon-btn" aria-label="Remove group" onclick={() => remove(i)}><Icon name="x" size={14} /></button>
      {#if g.Items.length === 0}<p class="err small">Choose at least one item, or remove the group.</p>{/if}
    </div>
  {/each}
  <button class="btn small add" onclick={add}><Icon name="star" size={13} /> Add a random group</button>
</div>

{#if editing !== null && groups[editing]}
  <Modal title="Random starting items" subtitle="One or more of these will be chosen at random." wide onclose={() => (editing = null)}>
    <PoolEditor
      id="starting-group"
      kind="items"
      {checks}
      value={groups[editing].Items}
      onchange={(v) => update(editing!, (x) => (x.Items = v))}
      views={["browse", "lists"]}
      labels={{ on: "In the group", off: "Not in the group", add: "Add", remove: "Remove", noun: "items" }}
    />
    {#snippet footer()}
      <button class="btn primary" onclick={() => (editing = null)}>Done</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .groups {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .group {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
  }
  .amount {
    display: flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
  }
  .amount input {
    width: 56px;
    padding: 3px 6px;
  }
  .items {
    flex: 1;
    min-width: 120px;
    color: var(--text-2);
  }
  .err {
    width: 100%;
    margin: 0;
    color: var(--bad);
  }
  .add {
    align-self: flex-start;
  }
</style>
