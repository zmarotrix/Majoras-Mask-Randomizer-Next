<script lang="ts">
  // A group box of the classic layout, optionally with inner tabs.
  import ClassicControl from "./ClassicControl.svelte";
  import ClassicCustom from "./ClassicCustom.svelte";
  import type { ClassicGroup, ClassicItem } from "../../lib/classic";

  let { group }: { group: ClassicGroup } = $props();

  let tab = $state(0);
  const items = $derived<ClassicItem[]>(group.tabs ? group.tabs[tab].items : (group.items ?? []));
  const cols = $derived((group.tabs ? group.tabs[tab].cols : group.cols) ?? 1);
</script>

{#snippet body()}
  <div class="items" style:--cols={cols}>
    {#each items as item, i (i)}
      {#if "note" in item}
        <p class="note" class:warn={item.warn}>{item.note}</p>
      {:else if "custom" in item}
        <div class="span"><ClassicCustom kind={item.custom} label={item.label} form={item.form} /></div>
      {:else if "flags" in item}
        <ClassicControl {item} />
      {:else}
        <div><ClassicControl {item} /></div>
      {/if}
    {/each}
  </div>
{/snippet}

{#if group.tabs}
  <div class="tabbed" style:grid-area={group.area}>
    <div class="strip" role="tablist">
      {#each group.tabs as t, i (t.title)}
        <button role="tab" aria-selected={tab === i} class:on={tab === i} onclick={() => (tab = i)}>{t.title}</button>
      {/each}
    </div>
    <div class="page">{@render body()}</div>
  </div>
{:else if group.title}
  <fieldset style:grid-area={group.area}>
    <legend>{group.title}</legend>
    {@render body()}
  </fieldset>
{:else}
  <div class="plain" style:grid-area={group.area}>{@render body()}</div>
{/if}

<style>
  fieldset {
    margin: 0;
    padding: 6px 12px 10px;
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
  }
  legend {
    padding: 0 6px;
    margin-left: -4px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }
  .items {
    display: grid;
    grid-template-columns: repeat(var(--cols), minmax(0, 1fr));
    gap: 0 14px;
    align-items: start;
  }
  @media (max-width: 760px) {
    .items {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .span,
  .note {
    grid-column: 1 / -1;
  }
  .note {
    margin: 6px 0 2px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-2);
  }
  .note.warn {
    margin: 0;
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    background: var(--warn-soft);
    color: var(--warn);
    font-weight: 500;
    text-align: center;
  }
  .plain {
    min-width: 0;
  }
  .tabbed {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .strip {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
  }
  .strip button {
    padding: 5px 10px;
    border: 1px solid var(--border);
    border-bottom: none;
    border-radius: var(--radius-sm) var(--radius-sm) 0 0;
    background: var(--surface-2);
    color: var(--text-2);
    font-size: 13px;
  }
  .strip button:hover {
    color: var(--text);
  }
  .strip button.on {
    position: relative;
    z-index: 1;
    margin-bottom: -1px;
    padding-bottom: 6px;
    background: var(--surface);
    color: var(--text);
    font-weight: 600;
  }
  .page {
    flex: 1;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 0 var(--radius-sm) var(--radius-sm) var(--radius-sm);
    background: var(--surface);
  }
</style>
