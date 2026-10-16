<script lang="ts">
  // One control of the classic layout: a checkbox, a labelled dropdown or a
  // number box, like MMR 1.16's window. Hovering shows the explanation in
  // the status bar (and as a tooltip).
  import { app } from "../../lib/state.svelte";
  import { addSpaces, type ClassicItem } from "../../lib/classic";
  import type { SettingDef } from "../../lib/types";

  let { item }: { item: Extract<ClassicItem, { s: string } | { flag: string } | { flags: string }> } = $props();

  const key = $derived("s" in item ? item.s : "flag" in item ? item.flag : item.flags);
  const def = $derived(app.names.settings.get(key) as SettingDef | undefined);
  const value = $derived(app.values[key]);
  const inactive = $derived(app.inactive.get(key));
  const label = $derived(("label" in item && item.label) || app.label(key));
  const tip = $derived([def?.description, inactive && `${inactive.message}.`].filter(Boolean).join("\n\n") || undefined);

  const list = $derived(Array.isArray(value) ? (value as string[]) : []);
  function toggleFlag(flag: string, on: boolean) {
    const next = list.filter((f) => f !== flag);
    if (on) next.push(flag);
    app.setValue(key, next);
  }

  /** Small whole-number ranges read better as a dropdown (e.g. "4 (Default)"). */
  const asDropdown = $derived(def?.type === "number" && def.integer && !def.nullable && (def.max ?? 99) - (def.min ?? 0) <= 8);
  const range = $derived(asDropdown ? Array.from({ length: def!.max! - def!.min! + 1 }, (_, i) => def!.min! + i) : []);

  const hover = () => (app.help = key);
</script>

{#if !def}
  <span class="missing small faint" title="This option doesn't exist in this version">{label}</span>
{:else if "flag" in item}
  <label class="check" class:off={!!inactive} title={tip} onmouseenter={hover}>
    <input type="checkbox" checked={list.includes(item.value)} onchange={(e) => toggleFlag(item.value, e.currentTarget.checked)} />
    {label}
  </label>
{:else if "flags" in item}
  {#each def.options ?? [] as o (o.value)}
    <label class="check" class:off={!!inactive} title={o.description ?? tip} onmouseenter={hover}>
      <input type="checkbox" checked={list.includes(o.value)} onchange={(e) => toggleFlag(o.value, e.currentTarget.checked)} />
      {addSpaces(o.value)}
    </label>
  {/each}
{:else if def.type === "bool"}
  <label class="check" class:off={!!inactive} title={tip} onmouseenter={hover}>
    <input type="checkbox" checked={value === true} onchange={(e) => app.setValue(key, e.currentTarget.checked)} />
    {label}
  </label>
{:else if def.type === "enum" || asDropdown}
  <label class="field" class:off={!!inactive} title={tip} onmouseenter={hover}>
    <span>{label}</span>
    {#if asDropdown}
      <select class="input" value={String(value)} onchange={(e) => app.setValue(key, Number(e.currentTarget.value))}>
        {#each range as n (n)}<option value={String(n)}>{n}{n === def.default ? " (Default)" : ""}</option>{/each}
      </select>
    {:else}
      <select class="input" value={value as string} onchange={(e) => app.setValue(key, e.currentTarget.value)}>
        {#each def.options ?? [] as o (o.value)}<option value={o.value}>{app.optionLabel(key, o.value)}</option>{/each}
      </select>
    {/if}
  </label>
{:else if def.type === "number"}
  <label class="field" class:off={!!inactive} title={tip} onmouseenter={hover}>
    <span>{label}</span>
    <input
      class="input num"
      type="number"
      min={def.min}
      max={def.max}
      step={def.integer ? 1 : "any"}
      value={value as number}
      onchange={(e) => app.setValue(key, Number(e.currentTarget.value))}
    />
  </label>
{:else}
  <span class="small faint" title={tip}>{label}</span>
{/if}

<style>
  .check {
    display: flex;
    align-items: center;
    gap: 7px;
    min-height: 26px;
    padding: 1px 0;
    cursor: pointer;
    line-height: 1.2;
  }
  .check input {
    width: 15px;
    height: 15px;
    margin: 0;
    flex: none;
    accent-color: var(--accent);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 2px 0 4px;
  }
  .field > span {
    font-size: 13px;
    color: var(--text-2);
  }
  .field .input {
    padding: 4px 8px;
    max-width: 240px;
  }
  .field .num {
    width: 110px;
  }
  .off {
    opacity: 0.5;
  }
  .missing {
    text-decoration: line-through;
  }
</style>
