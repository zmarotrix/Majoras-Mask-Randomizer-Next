<script lang="ts">
  // How likely each trap type is (relative weights; 0 = never).
  import { app } from "../../lib/state.svelte";

  const KEY = "TrapWeights";
  const def = $derived(app.names.settings.get(KEY)!);
  const weights = $derived((app.values[KEY] as Record<string, number> | null) ?? {});

  function set(trap: string, n: number) {
    const next: Record<string, number> = {};
    for (const o of def.options ?? []) next[o.value] = weights[o.value] ?? 0;
    next[trap] = Math.max(0, Math.min(1000, Math.round(n) || 0));
    app.setValue(KEY, next);
  }
</script>

<div class="weights" onmouseenter={() => (app.help = KEY)} role="group" aria-label="Trap weights">
  {#each def.options ?? [] as o (o.value)}
    <label title={o.description}>
      <span>{o.label}</span>
      <input class="input" type="number" min="0" max="1000" value={weights[o.value] ?? 0} onchange={(e) => set(o.value, Number(e.currentTarget.value))} />
    </label>
  {/each}
</div>

<style>
  .weights {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(110px, 1fr));
    gap: 6px 12px;
  }
  label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    font-size: 13px;
  }
  input {
    width: 64px;
    padding: 3px 6px;
  }
</style>
