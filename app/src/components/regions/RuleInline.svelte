<script lang="ts">
  // A rule as one compact sentence (fallback for very large rules): "always", tags, "one of: …", "all of: …",
  // "2 of: …". Long rules are shortened ("+3 more").
  import RefTag from "./RefTag.svelte";
  import Self from "./RuleInline.svelte";
  import { isAlways, isAll, isAny, isRef } from "../../lib/rules";
  import type { Rule } from "../../lib/types";

  let { rule, max = 6, nested = false }: { rule: Rule | undefined; max?: number; nested?: boolean } = $props();

  const parts = $derived.by(() => {
    if (rule === undefined || isRef(rule)) return [];
    if (isAll(rule)) return rule.all;
    if (isAny(rule)) return rule.any;
    return [];
  });
  const shown = $derived(parts.slice(0, max));
  const more = $derived(parts.length - shown.length);
</script>

{#if isAlways(rule)}
  {#if !nested}<span class="faint">always</span>{/if}
{:else if isRef(rule!)}
  <RefTag id={rule} small />
{:else if isAll(rule!) || isAny(rule!)}
  <span class="group" class:nested>
    {#if isAny(rule!)}<span class="op">{nested ? "(one of" : "one of"}</span>{:else if nested}<span class="op">(</span>{/if}
    {#each shown as part, i (i)}
      {#if i > 0 && isAny(rule!)}<span class="op">or</span>{/if}
      <Self rule={part} max={3} nested />
    {/each}
    {#if more > 0}<span class="faint">+{more} more</span>{/if}
    {#if nested}<span class="op">)</span>{/if}
  </span>
{:else if "count" in rule!}
  <span class="group">
    <span class="op">{rule.count} of</span>
    {#each rule.of.slice(0, max) as id (id)}<RefTag {id} small />{/each}
    {#if rule.of.length > max}<span class="faint">+{rule.of.length - max} more</span>{/if}
  </span>
{/if}

<style>
  .group {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
  }
  .op {
    font-size: 11.5px;
    color: var(--text-3);
  }
</style>
