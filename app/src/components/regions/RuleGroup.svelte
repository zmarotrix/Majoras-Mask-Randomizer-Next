<script lang="ts">
  // One block of the rule builder ("all of" or "any one of").
  import RefTag from "./RefTag.svelte";
  import Picker from "./Picker.svelte";
  import Self from "./RuleGroup.svelte";
  import Icon from "../Icon.svelte";
  import { doc } from "../../lib/regionDoc.svelte";
  import { appendAt, children, isAll, isAny, isCount, isRef, replaceAt, type RulePath } from "../../lib/rules";
  import type { Rule } from "../../lib/types";

  interface Props {
    node: Rule;
    path: RulePath;
    root: Rule;
    onchange: (r: Rule) => void;
  }
  let { node, path, root, onchange }: Props = $props();

  let picking = $state(false);
  const kids = $derived(children(node) ?? []);
  const any = $derived(isAny(node));
  const depth = $derived(path.length);

  const set = (next: Rule | null, at: RulePath = path) => onchange(replaceAt(root, at, next));
  function flip() {
    set(any ? { all: kids } : { any: kids });
  }
  function addRef(id: string) {
    picking = false;
    onchange(appendAt(root, path, id));
  }
  function addGroup() {
    onchange(appendAt(root, path, any ? { all: [] } : { any: [] }));
  }
  function setCount(i: number, n: number) {
    const c = kids[i];
    if (isCount(c)) set({ count: Math.max(1, Math.min(c.of.length || 1, n)), of: c.of }, [...path, i]);
  }
</script>

<div class="group" class:any class:root={depth === 0} style:--d={depth}>
  <div class="head">
    <button class="kind" onclick={flip} title="Switch between needing all of these and needing any one of them">
      {any ? "Any one of" : "All of"}
      <Icon name="chevron-down" size={12} />
    </button>
    {#if kids.length === 0}<span class="faint small">{depth === 0 ? "Nothing needed (always)" : "empty"}</span>{/if}
    {#if depth > 0}
      <button class="icon-btn remove" aria-label="Remove this group" title="Remove this group" onclick={() => set(null)}><Icon name="x" size={13} /></button>
    {/if}
  </div>
  <div class="kids">
    {#each kids as kid, i (i)}
      {#if isRef(kid)}
        <RefTag id={kid} onremove={() => set(null, [...path, i])} />
      {:else if isCount(kid)}
        <span class="count">
          <input class="input" type="number" min="1" max={kid.of.length} value={kid.count} onchange={(e) => setCount(i, Number(e.currentTarget.value))} aria-label="How many" />
          of
          {#each kid.of as id (id)}<RefTag {id} small />{/each}
          <button class="icon-btn" aria-label="Remove" onclick={() => set(null, [...path, i])}><Icon name="x" size={12} /></button>
        </span>
      {:else}
        <Self node={kid} path={[...path, i]} {root} {onchange} />
      {/if}
    {/each}
  </div>
  <div class="add">
    {#if picking}
      <div class="picker-wrap">
        <Picker options={doc.refOptions} placeholder="Item, region, helper, trick…" onpick={addRef} oncancel={() => (picking = false)} />
      </div>
    {:else}
      <button class="link" onclick={() => (picking = true)}><Icon name="star" size={12} /> Add requirement</button>
      <button class="link" onclick={addGroup}>+ {any ? "“all of” group" : "“any one of” group"}</button>
    {/if}
  </div>
</div>

<style>
  .group {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px 10px;
    border: 1px solid var(--border-strong);
    border-left: 3px solid var(--text-3);
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--surface-2) 70%, transparent);
  }
  .group.any {
    border-left-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 7%, transparent);
  }
  .group.root {
    border-color: var(--border);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .kind {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    background: var(--surface);
    color: var(--text-2);
    font-size: 11.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .any > .head .kind {
    color: var(--accent);
    border-color: color-mix(in srgb, var(--accent) 50%, transparent);
  }
  .remove {
    margin-left: auto;
  }
  .kids {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 6px;
  }
  .kids > :global(.group) {
    flex-basis: 100%;
  }
  .count {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--text-2);
  }
  .count input {
    width: 48px;
    padding: 1px 4px;
  }
  .add {
    display: flex;
    gap: 12px;
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 0;
    border: none;
    background: none;
    color: var(--accent);
    font-size: 12.5px;
  }
  .link:hover {
    text-decoration: underline;
  }
  .picker-wrap {
    width: 100%;
  }
</style>
