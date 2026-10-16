<script lang="ts">
  // A rule as an organised list: what's always needed, then each
  // alternative way on its own line ("one of"). Tags are in a fixed order
  // (regions, items, helpers, settings, tricks). Very large rules fall back
  // to a compact sentence.
  import RefTag from "./RefTag.svelte";
  import RuleInline from "./RuleInline.svelte";
  import { doc, dnf, type RefKind } from "../../lib/regionDoc.svelte";
  import { isAlways } from "../../lib/rules";
  import type { Rule } from "../../lib/types";

  let { rule, max = 5 }: { rule: Rule | undefined; max?: number } = $props();

  const ORDER: Record<RefKind, number> = { region: 0, event: 1, item: 2, location: 3, helper: 4, setting: 5, trick: 6, unknown: 7 };
  const sortIds = (ids: string[]) => [...ids].sort((a, b) => ORDER[doc.kindOf(a)] - ORDER[doc.kindOf(b)] || doc.refName(a).localeCompare(doc.refName(b)));

  /** Too many alternatives to list (rules are usually small). */
  const LIMIT = 40;

  const shape = $derived.by(() => {
    if (rule === undefined || isAlways(rule)) return { kind: "always" as const };
    let alts: string[][];
    try {
      alts = dnf(rule);
    } catch {
      return { kind: "inline" as const };
    }
    if (alts.length > LIMIT) return { kind: "inline" as const };
    if (alts.length === 0) return { kind: "never" as const };
    const common = alts[0].filter((id) => alts.every((a) => a.includes(id)));
    let rest = alts.map((a) => a.filter((id) => !common.includes(id)));
    // An alternative that needs nothing beyond the common part makes the rest redundant.
    if (rest.some((a) => a.length === 0)) rest = [];
    // Drop duplicate alternatives.
    const seen = new Set<string>();
    rest = rest.filter((a) => {
      const k = [...a].sort().join("|");
      return seen.has(k) ? false : (seen.add(k), true);
    });
    if (rest.length === 1) return { kind: "list" as const, common: sortIds([...common, ...rest[0]]), alts: [] as string[][] };
    return { kind: "list" as const, common: sortIds(common), alts: rest.map(sortIds) };
  });
</script>

{#if shape.kind === "always"}
  <span class="faint">nothing needed</span>
{:else if shape.kind === "never"}
  <span class="never">impossible</span>
{:else if shape.kind === "inline"}
  <RuleInline {rule} />
{:else}
  <div class="req">
    {#if shape.common.length}
      <div class="line">
        <span class="lead">{shape.alts.length ? "needs" : "needs"}</span>
        <span class="tags">{#each shape.common as id (id)}<RefTag {id} small />{/each}</span>
      </div>
    {/if}
    {#if shape.alts.length}
      <div class="line">
        <span class="lead">{shape.common.length ? "and one of" : "one of"}</span>
        <ol class="alts">
          {#each shape.alts.slice(0, max) as alt, i (i)}
            <li>
              {#each alt as id, j (id)}
                {#if j > 0}<span class="plus">+</span>{/if}
                <RefTag {id} small />
              {/each}
            </li>
          {/each}
          {#if shape.alts.length > max}<li class="more faint">+{shape.alts.length - max} more ways</li>{/if}
        </ol>
      </div>
    {/if}
  </div>
{/if}

<style>
  .req {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .line {
    display: grid;
    grid-template-columns: 76px minmax(0, 1fr);
    align-items: start;
    gap: 8px;
  }
  .lead {
    padding-top: 2px;
    font-size: 11.5px;
    color: var(--text-3);
    white-space: nowrap;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .alts {
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin: 0;
    padding-left: 18px;
  }
  .alts li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
  }
  .alts li::marker {
    font-size: 11px;
    color: var(--text-3);
  }
  .alts li.more {
    list-style: none;
    font-size: 12px;
  }
  .plus {
    font-size: 11px;
    color: var(--text-3);
  }
  .never {
    color: var(--bad);
  }
</style>
