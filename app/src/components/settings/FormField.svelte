<script lang="ts">
  // One setting in the tidy form layout: a checkbox for on/off options,
  // otherwise a label above its control. Explanations show in the help
  // panel on hover or focus (and as a tooltip).
  import TricksPicker from "../TricksPicker.svelte";
  import StructuredEditor from "../editors/StructuredEditor.svelte";
  import Icon from "../Icon.svelte";
  import { backend } from "../../lib/backend";
  import { app } from "../../lib/state.svelte";
  import type { SettingDef } from "../../lib/types";

  let { def }: { def: SettingDef } = $props();

  let picking = $state(false);

  const value = $derived(app.values[def.key]);
  const changed = $derived(app.isChanged(def.key));
  const inactive = $derived(app.inactive.get(def.key));
  const issue = $derived(app.settingsIssues.find((i) => i.key === def.key));
  const label = $derived(app.label(def.key));
  const options = $derived((def.options ?? []).map((o) => ({ ...o, label: app.optionLabel(def.key, o.value) })));
  const tip = $derived([def.description, inactive && `${inactive.message}.`].filter(Boolean).join("\n\n") || undefined);
  const list = $derived(Array.isArray(value) ? (value as string[]) : []);

  const set = (v: unknown) => app.setValue(def.key, v);
  function toggleFlag(flag: string, on: boolean) {
    const next = list.filter((f) => f !== flag);
    if (on) next.push(flag);
    set(next);
  }
  async function browse() {
    const path = await backend.pickFile(label, [{ name: "Logic file", extensions: ["json", "txt"] }]);
    if (path) set(path);
  }
  const focus = () => {
    if (app.help !== def.key) app.helpOption = null;
    app.help = def.key;
  };
</script>

{#snippet name()}
  <span class="name">{label}{#if changed}<span class="dot" title="Changed from default"></span>{/if}</span>
{/snippet}

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="field {def.type}" class:off={!!inactive} class:active={app.help === def.key} title={tip} onmouseenter={focus} onfocusin={focus}>
  {#if def.type === "bool"}
    <label class="check">
      <input type="checkbox" checked={value === true} onchange={(e) => set(e.currentTarget.checked)} />
      {@render name()}
    </label>
  {:else}
    {@render name()}
    {#if def.type === "enum"}
      <select class="input" value={value as string} onchange={(e) => set(e.currentTarget.value)} aria-label={label}>
        {#each options as o (o.value)}<option value={o.value}>{o.label}</option>{/each}
      </select>
    {:else if def.type === "number"}
      <span class="num-row">
        {#if def.nullable}
          <label class="auto small">
            <input type="checkbox" checked={value === null} onchange={(e) => set(e.currentTarget.checked ? null : (def.min ?? 0))} />
            Automatic
          </label>
        {/if}
        {#if value !== null}
          <input
            class="input num"
            type="number"
            min={def.min}
            max={def.max}
            step={def.integer ? 1 : "any"}
            value={value as number}
            onchange={(e) => set(Number(e.currentTarget.value))}
            aria-label={label}
          />
        {/if}
      </span>
    {:else if def.type === "tricks"}
      <button class="btn" onclick={() => (picking = true)}>{list.length} enabled · Choose…</button>
    {:else if def.type === "file"}
      <span class="file">
        <span class="path small muted" title={String(value || "")}>{value ? String(value).split(/[\\/]/).pop() : "None"}</span>
        <button class="btn small" onclick={browse}>Browse…</button>
      </span>
    {:else if def.type === "flags"}
      <span class="chips">
        {#each options as o (o.value)}
          {@const on = list.includes(o.value)}
          <label
            class="chip"
            class:on
            onmouseenter={() => ((app.help = def.key), (app.helpOption = o.value))}
            onmouseleave={() => (app.helpOption = null)}
            onfocusin={() => ((app.help = def.key), (app.helpOption = o.value))}
          >
            <input type="checkbox" checked={on} onchange={(e) => toggleFlag(o.value, e.currentTarget.checked)} />
            {#if on}<Icon name="check" size={12} />{/if}
            {o.label}
          </label>
        {/each}
      </span>
    {:else if def.type === "custom"}
      <div class="structured"><StructuredEditor {def} /></div>
    {/if}
  {/if}
  {#if issue}<p class="err small">{issue.message}</p>{/if}
</div>

{#if picking}
  <TricksPicker value={list} onsave={set} onclose={() => (picking = false)} />
{/if}

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    min-width: 0;
  }
  .field.bool {
    justify-content: center;
  }
  .name {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: var(--text-2);
  }
  .bool .name {
    font-size: 14px;
    color: var(--text);
  }
  .active > .name,
  .active .check .name {
    color: var(--text);
  }
  .check {
    display: flex;
    align-items: center;
    gap: 9px;
    min-height: 30px;
    cursor: pointer;
  }
  .check input,
  .auto input {
    width: 16px;
    height: 16px;
    margin: 0;
    flex: none;
    accent-color: var(--sec);
  }
  select.input {
    width: 100%;
    padding: 6px 8px;
  }
  .num-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .num {
    width: 96px;
    padding: 6px 8px;
  }
  .auto {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text-2);
  }
  .file {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    font-size: 13px;
    cursor: pointer;
  }
  .chip input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .chip:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .chip:hover {
    border-color: color-mix(in srgb, var(--sec) 50%, var(--border));
  }
  .chip.on {
    border-color: var(--sec);
    background: color-mix(in srgb, var(--sec) 20%, transparent);
    color: var(--text);
    box-shadow: 0 0 12px -4px var(--sec);
  }
  .structured {
    padding-top: 2px;
  }
  .off {
    opacity: 0.45;
  }
  .err {
    margin: 0;
    color: var(--bad);
  }
</style>
