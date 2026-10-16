<script lang="ts">
  // Choose enabled tricks, grouped by area, with their descriptions and
  // links to videos/guides.
  import Modal from "./Modal.svelte";
  import { backend } from "../lib/backend";
  import { app, toasts } from "../lib/state.svelte";
  import type { Entry } from "../lib/types";

  interface Props {
    value: string[];
    onsave: (value: string[]) => void;
    onclose: () => void;
  }
  let { value, onsave, onclose }: Props = $props();

  // svelte-ignore state_referenced_locally (the picker edits a copy of the initial value)
  let selected = $state(new Set(value));
  let query = $state("");
  let tricks = $state<Entry[] | null>(null);
  let failed = $state<string | null>(null);

  const logicId = $derived(app.values.LogicMode === "Glitched" ? "glitched" : "casual");

  $effect(() => {
    const id = logicId;
    failed = null;
    const timer = setTimeout(() => (failed ??= "The tricks didn't load. Close this and try again."), 10_000);
    backend
      .logicTricks(id)
      .then((list) => (tricks = list))
      .catch((e) => {
        failed = String(e);
        toasts.error(e);
      })
      .finally(() => clearTimeout(timer));
    return () => clearTimeout(timer);
  });

  const groups = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const map = new Map<string, Entry[]>();
    for (const t of tricks ?? []) {
      if (q && !`${t.id} ${t.trick?.description ?? ""} ${t.trick?.category ?? ""}`.toLowerCase().includes(q)) continue;
      const k = t.trick?.category ?? "Other";
      let list = map.get(k);
      if (!list) map.set(k, (list = []));
      list.push(t);
    }
    return [...map.entries()].sort(([a], [b]) => a.localeCompare(b));
  });

  /** Tricks enabled in settings that the chosen logic doesn't have. */
  const unknown = $derived(tricks ? value.filter((v) => !tricks!.some((t) => t.id === v)) : []);

  function toggle(id: string) {
    const next = new Set(selected);
    next.has(id) ? next.delete(id) : next.add(id);
    selected = next;
  }

  function setAll(list: Entry[], on: boolean) {
    const next = new Set(selected);
    for (const t of list) on ? next.add(t.id) : next.delete(t.id);
    selected = next;
  }
</script>

<Modal
  title="Enabled tricks"
  subtitle="Tricks are techniques the logic may expect you to know. Only enable ones you're comfortable doing."
  wide
  {onclose}
>
  <div class="toolbar">
    <div class="search grow"><input class="input" placeholder="Search tricks and descriptions…" bind:value={query} /></div>
    <span class="small muted"><strong>{selected.size}</strong> enabled · from {logicId === "glitched" ? "Glitched" : "Casual"} logic</span>
  </div>

  {#if unknown.length}
    <p class="warn small">
      {unknown.length} enabled trick{unknown.length === 1 ? " doesn't" : "s don't"} exist in this logic and will be ignored: {unknown.join(", ")}
    </p>
  {/if}

  {#if failed && !tricks}
    <p class="err">{failed}</p>
  {:else if !tricks}
    <p class="muted">Loading tricks…</p>
  {:else}
    {#each groups as [category, list] (category)}
      {@const on = list.filter((t) => selected.has(t.id)).length}
      <section>
        <div class="cat-head">
          <h3>{category}</h3>
          <span class="badge" class:accent={on > 0}>{on}/{list.length}</span>
          <span class="spacer"></span>
          <button class="btn ghost small" onclick={() => setAll(list, on < list.length)}>{on < list.length ? "Enable all" : "Disable all"}</button>
        </div>
        {#if category === "PROCEED WITH CAUTION"}
          <p class="small warn">These tricks are risky or very difficult.</p>
        {/if}
        {#each list as t (t.id)}
          <label class="trick" class:on={selected.has(t.id)}>
            <input type="checkbox" checked={selected.has(t.id)} onchange={() => toggle(t.id)} />
            <span class="text">
              <span class="name">{t.id}</span>
              {#if t.trick?.description}<span class="small muted">{t.trick.description}</span>{/if}
            </span>
            {#if t.trick?.url}
              <button class="btn ghost small" onclick={(e) => (e.preventDefault(), backend.openLink(t.trick!.url!))}>Watch ↗</button>
            {/if}
          </label>
        {/each}
      </section>
    {:else}
      <p class="muted">No tricks match “{query}”.</p>
    {/each}
  {/if}

  {#snippet footer()}
    <button class="btn" onclick={onclose}>Cancel</button>
    <button class="btn primary" onclick={() => onsave([...selected])}>Apply</button>
  {/snippet}
</Modal>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    position: sticky;
    top: -14px;
    background: var(--surface);
    padding: 2px 0 10px;
    z-index: 1;
  }
  .grow {
    flex: 1;
  }
  section {
    margin-bottom: 14px;
  }
  .cat-head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 0;
    border-bottom: 1px solid var(--border);
    margin-bottom: 4px;
  }
  .trick {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 7px 8px;
    border-radius: var(--radius-sm);
  }
  .trick:hover {
    background: var(--surface-2);
  }
  .trick.on {
    background: var(--accent-soft);
  }
  .trick input {
    margin-top: 3px;
  }
  .text {
    flex: 1;
    display: flex;
    flex-direction: column;
  }
  .name {
    font-weight: 600;
  }
  .warn {
    color: var(--warn);
    margin-bottom: 8px;
  }
  .err {
    color: var(--bad);
  }
</style>
