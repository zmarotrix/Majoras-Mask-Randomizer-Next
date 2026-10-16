<script lang="ts">
  // A group's settings as a tidy form: choices lined up in columns, then
  // bigger editors (lists of options, colours...), then on/off options as a
  // checkbox grid. Related extras follow under a small heading.
  import FormField from "./FormField.svelte";
  import { MERGED } from "../editors/StructuredEditor.svelte";
  import { app } from "../../lib/state.svelte";
  import type { LayoutGroup, SettingDef } from "../../lib/types";

  let { group, only = null }: { group: LayoutGroup; only?: Set<string> | null } = $props();

  const defs = (keys: string[]) =>
    keys
      .map((k) => app.names.settings.get(k))
      // Companions are edited inside their partner's editor.
      .filter((d): d is SettingDef => !!d && !d.companion_of && !MERGED.has(d.key) && (!only || only.has(d.key)));

  function split(list: SettingDef[]) {
    return {
      choices: list.filter((d) => ["enum", "number", "tricks", "file"].includes(d.type)),
      wide: list.filter((d) => d.type === "flags" || d.type === "custom"),
      toggles: list.filter((d) => d.type === "bool"),
    };
  }
  const main = $derived(split(defs(group.settings)));
  const more = $derived(group.more ? split(defs(group.more.settings)) : null);
  const moreCount = $derived(more ? more.choices.length + more.wide.length + more.toggles.length : 0);
</script>

{#snippet block(parts: ReturnType<typeof split>)}
  {#if parts.choices.length}
    <div class="choices">
      {#each parts.choices as def (def.key)}<FormField {def} />{/each}
    </div>
  {/if}
  {#each parts.wide as def (def.key)}
    <div class="wide"><FormField {def} /></div>
  {/each}
  {#if parts.toggles.length}
    <div class="toggles">
      {#each parts.toggles as def (def.key)}<FormField {def} />{/each}
    </div>
  {/if}
{/snippet}

<div class="form">
  {@render block(main)}
  {#if more && moreCount}
    <h4 class="sub">{group.more!.label}</h4>
    {@render block(more)}
  {/if}
</div>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .choices {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 14px 18px;
  }
  .toggles {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 4px 18px;
  }
  .sub {
    margin: 4px 0 -6px;
    padding-top: 14px;
    border-top: 1px solid var(--border);
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-3);
  }
</style>
