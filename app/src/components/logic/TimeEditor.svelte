<script lang="ts">
  import { PERIODS, type Period, type Time } from "../../lib/types";

  let { value, readonly = false, onchange }: { value: Time; readonly?: boolean; onchange: (t: Time) => void } = $props();

  type PeriodKey = "available" | "needed" | "setup";
  const rows: { key: PeriodKey; label: string; help: string }[] = [
    { key: "available", label: "Available", help: "When this can be done. Leave empty if it can be done at any time." },
    { key: "needed", label: "Needed", help: "When having this matters. Locations that need it must be available at one of these times." },
    { key: "setup", label: "Setup", help: "When the setup for this happens, if that's different from when it's available." },
  ];

  const LABELS: Record<Period, string> = { Day1: "D1", Night1: "N1", Day2: "D2", Night2: "N2", Day3: "D3", Night3: "N3" };
  const FULL: Record<Period, string> = { Day1: "Day 1", Night1: "Night 1", Day2: "Day 2", Night2: "Night 2", Day3: "Day 3", Night3: "Night 3" };

  function toggle(key: PeriodKey, p: Period) {
    const cur = value[key] ?? [];
    const next = cur.includes(p) ? cur.filter((x) => x !== p) : PERIODS.filter((x) => x === p || cur.includes(x));
    onchange({ ...value, [key]: next });
  }
</script>

<div class="time">
  {#each rows as r (r.key)}
    <div class="row-label" title={r.help}>{r.label}</div>
    <div class="periods">
      {#each PERIODS as p (p)}
        {@const on = value[r.key]?.includes(p) ?? false}
        <button class="p" class:on class:night={p.startsWith("Night")} disabled={readonly} onclick={() => toggle(r.key, p)} aria-pressed={on} title={FULL[p]}>
          {LABELS[p]}
        </button>
      {/each}
      {#if !value[r.key]?.length}<span class="small faint any">any</span>{/if}
    </div>
  {/each}
</div>

<style>
  .time {
    display: grid;
    grid-template-columns: 74px 1fr;
    gap: 6px 10px;
    align-items: center;
  }
  .row-label {
    font-size: 13px;
    color: var(--text-2);
    cursor: help;
  }
  .periods {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
    align-items: center;
  }
  .p {
    padding: 2px 7px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    font-size: 12.5px;
  }
  .p.on {
    background: var(--gold-soft);
    border-color: var(--gold);
    color: var(--gold);
    font-weight: 600;
  }
  .p.night.on {
    background: var(--accent-soft);
    border-color: var(--accent);
    color: var(--accent);
  }
  .any {
    margin-left: 6px;
  }
</style>
