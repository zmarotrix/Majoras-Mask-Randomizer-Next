<script lang="ts">
  // Builds a setting condition from dropdowns instead of C# code.
  import { app } from "../../lib/state.svelte";
  import { describeCondition } from "../../lib/names";
  import type { SettingCondition } from "../../lib/types";

  let { value, onchange }: { value: SettingCondition; onchange: (c: SettingCondition) => void } = $props();

  type Op = "on" | "off" | "is" | "is_not" | "has" | "has_not" | "randomized" | "not_randomized";

  // Flatten the condition into (setting, op, operand) for the form.
  const parsed = $derived.by(() => {
    let c = value;
    let neg = false;
    while ("not" in c) {
      neg = !neg;
      c = c.not;
    }
    if ("randomized" in c) return { setting: "CustomItemList", op: (neg ? "not_randomized" : "randomized") as Op, operand: c.randomized };
    if (c.is !== undefined) return { setting: c.setting, op: (neg ? "is_not" : "is") as Op, operand: String(c.is) };
    if (c.has !== undefined) return { setting: c.setting, op: (neg ? "has_not" : "has") as Op, operand: c.has };
    return { setting: c.setting, op: (neg ? "off" : "on") as Op, operand: "" };
  });

  const settings = $derived(app.schema!.settings.filter((s) => ["bool", "enum", "flags"].includes(s.type) || s.key === "CustomItemList"));
  const def = $derived(app.names.settings.get(parsed.setting));

  function opsFor(type: string | undefined): { op: Op; label: string }[] {
    if (type === "bool") return [{ op: "on", label: "is on" }, { op: "off", label: "is off" }];
    if (type === "enum") return [{ op: "is", label: "is" }, { op: "is_not", label: "is not" }];
    if (type === "flags") return [{ op: "has", label: "includes" }, { op: "has_not", label: "doesn't include" }];
    return [{ op: "randomized", label: "includes location" }, { op: "not_randomized", label: "doesn't include location" }];
  }

  function build(setting: string, op: Op, operand: string): SettingCondition {
    const wrap = (c: SettingCondition, neg: boolean): SettingCondition => (neg ? { not: c } : c);
    switch (op) {
      case "on":
      case "off":
        return wrap({ setting }, op === "off");
      case "is":
      case "is_not":
        return wrap({ setting, is: operand }, op === "is_not");
      case "has":
      case "has_not":
        return wrap({ setting, has: operand }, op === "has_not");
      default:
        return wrap({ randomized: operand }, op === "not_randomized");
    }
  }

  function setSetting(key: string) {
    const d = app.names.settings.get(key);
    const op = opsFor(key === "CustomItemList" ? "item_list" : d?.type)[0].op;
    const operand = d?.options?.[0]?.value ?? (key === "CustomItemList" ? "ItemBow" : "");
    onchange(build(key, op, operand));
  }

  const locations = $derived(app.game!.items.filter((i) => i.location));
</script>

<div class="cond">
  <select class="input" value={parsed.setting} onchange={(e) => setSetting(e.currentTarget.value)} aria-label="Setting">
    {#each settings as s (s.key)}
      <option value={s.key}>{s.key === "CustomItemList" ? "Randomized locations" : s.label}</option>
    {/each}
  </select>
  <select
    class="input op"
    value={parsed.op}
    onchange={(e) => onchange(build(parsed.setting, e.currentTarget.value as Op, parsed.operand))}
    aria-label="Comparison"
  >
    {#each opsFor(parsed.setting === "CustomItemList" ? "item_list" : def?.type) as o (o.op)}
      <option value={o.op}>{o.label}</option>
    {/each}
  </select>
  {#if parsed.op === "randomized" || parsed.op === "not_randomized"}
    <select class="input" value={parsed.operand} onchange={(e) => onchange(build(parsed.setting, parsed.op, e.currentTarget.value))} aria-label="Location">
      {#each locations as l (l.id)}
        <option value={l.id}>{l.location}</option>
      {/each}
    </select>
  {:else if parsed.op !== "on" && parsed.op !== "off"}
    <select class="input" value={parsed.operand} onchange={(e) => onchange(build(parsed.setting, parsed.op, e.currentTarget.value))} aria-label="Value">
      {#each def?.options ?? [] as o (o.value)}
        <option value={o.value}>{o.label}</option>
      {/each}
    </select>
  {/if}
</div>
<p class="small muted read">Reads as: <strong>{describeCondition(value, app.names)}</strong></p>

<style>
  .cond {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .cond select {
    width: auto;
    max-width: 280px;
  }
  .read {
    margin-top: 6px;
  }
</style>
