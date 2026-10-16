<script lang="ts">
  // Build a rule from blocks: "all of" / "any one of" groups that hold
  // things (items, regions, helpers, tricks...) and more groups.
  import RuleGroup from "./RuleGroup.svelte";
  import { isAll, isAny } from "../../lib/rules";
  import type { Rule } from "../../lib/types";

  let { rule, onchange }: { rule: Rule | undefined; onchange: (r: Rule) => void } = $props();

  /** The top is always a group, so there's somewhere to add things. */
  const root = $derived<Rule>(rule === undefined ? { all: [] } : isAll(rule) || isAny(rule) ? rule : { all: [rule] });
</script>

<RuleGroup node={root} path={[]} {root} {onchange} />
