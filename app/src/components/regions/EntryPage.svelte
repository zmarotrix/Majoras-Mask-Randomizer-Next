<script lang="ts">
  // A helper, trick or setting rule: what it is and where it's used.
  import RuleView from "./RuleView.svelte";
  import ConditionEditor from "../logic/ConditionEditor.svelte";
  import { KIND_COLORS, KIND_NAMES } from "./RefTag.svelte";
  import { doc, entryRule } from "../../lib/regionDoc.svelte";
  import { humanize } from "../../lib/names";
  import { backend } from "../../lib/backend";

  let { id }: { id: string } = $props();

  const entry = $derived(doc.entryById.get(id));
  const kind = $derived(doc.kindOf(id));
  const used = $derived(doc.usedBy.get(id) ?? []);
  const problems = $derived(doc.issues.filter((i) => i.entry === id && i.severity !== "info"));
</script>

{#if entry}
  <div class="page">
    <header>
      <span class="kind" style:--k={KIND_COLORS[kind]}>{KIND_NAMES[kind]}</span>
      <h1>{humanize(id)}</h1>
      {#if humanize(id) !== id}<p class="faint small mono">{id}</p>{/if}
    </header>

    {#if problems.length}
      <div class="problems">{#each problems as p, i (i)}<p>{p.message}</p>{/each}</div>
    {/if}

    {#if entry.trick}
      <section class="card fields">
        <label>
          <span>What the trick is</span>
          <textarea
            class="input"
            rows="3"
            value={entry.trick.description ?? ""}
            onchange={(e) => doc.updateEntry(id, "Edit trick", (x) => (x.trick!.description = e.currentTarget.value || undefined))}
          ></textarea>
        </label>
        <div class="two">
          <label>
            <span>Category (area)</span>
            <input class="input" value={entry.trick.category ?? ""} onchange={(e) => doc.updateEntry(id, "Edit trick", (x) => (x.trick!.category = e.currentTarget.value || undefined))} />
          </label>
          <label>
            <span>Video or guide link</span>
            <span class="url">
              <input class="input" value={entry.trick.url ?? ""} onchange={(e) => doc.updateEntry(id, "Edit trick", (x) => (x.trick!.url = e.currentTarget.value || undefined))} />
              {#if entry.trick.url}<button class="btn small" onclick={() => backend.openLink(entry.trick!.url!)}>Open</button>{/if}
            </span>
          </label>
        </div>
      </section>
    {/if}

    {#if entry.setting}
      <section class="card fields">
        <span class="small muted">Satisfied when</span>
        <ConditionEditor value={entry.setting} onchange={(c) => doc.updateEntry(id, "Edit setting rule", (x) => (x.setting = c))} />
      </section>
    {/if}

    {#if !entry.trick && !entry.setting}
      <section>
        <h2>Requires</h2>
        <div class="row" role="button" tabindex="0" onclick={() => (doc.target = { kind: "entry", id })} onkeydown={(e) => e.key === "Enter" && (doc.target = { kind: "entry", id })}>
          <RuleView rule={entryRule(entry)} max={12} />
        </div>
      </section>
    {:else if entry.requires?.length || entry.any_of?.length}
      <section>
        <h2>Also requires</h2>
        <div class="row" role="button" tabindex="0" onclick={() => (doc.target = { kind: "entry", id })} onkeydown={(e) => e.key === "Enter" && (doc.target = { kind: "entry", id })}>
          <RuleView rule={entryRule(entry)} max={12} />
        </div>
      </section>
    {/if}

    <section>
      <h2>Used by <span class="faint">{used.length}</span></h2>
      {#each used.slice(0, 200) as u, i (i)}
        <button class="use" onclick={() => doc.go(u.page, u.target)}>{u.label}</button>
      {:else}
        <p class="faint small">Nothing uses this yet.</p>
      {/each}
    </section>

    <section>
      <h2>Note</h2>
      <textarea
        class="input"
        rows="2"
        placeholder="Notes for other logic writers (not used by the randomizer)"
        value={entry.note ?? ""}
        onchange={(e) => doc.updateEntry(id, "Edit note", (x) => (x.note = e.currentTarget.value || undefined))}
      ></textarea>
    </section>
  </div>
{/if}

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding: 20px 24px 40px;
    max-width: 900px;
  }
  .kind {
    display: inline-block;
    margin-bottom: 4px;
    padding: 1px 9px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--k) 18%, transparent);
    color: var(--k);
    font-size: 11.5px;
    font-weight: 600;
  }
  h1 {
    font-size: 21px;
  }
  h2 {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-2);
    margin-bottom: 6px;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .fields {
    gap: 12px;
    padding: 14px 16px;
  }
  .fields label {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .fields label > span:first-child {
    font-size: 12.5px;
    color: var(--text-2);
  }
  .two {
    display: grid;
    grid-template-columns: 1fr 2fr;
    gap: 12px;
  }
  .url {
    display: flex;
    gap: 6px;
  }
  .row {
    cursor: pointer;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    text-align: left;
  }
  .use {
    padding: 4px 8px;
    border: none;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-2);
    font-size: 13px;
    text-align: left;
  }
  .use:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .problems {
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    background: var(--warn-soft);
    color: var(--warn);
    font-size: 13px;
  }
</style>
