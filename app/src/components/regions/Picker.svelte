<script lang="ts">
  // Search and pick one thing (an item, region, helper, trick, check...).
  // Arrow keys and Enter work; Escape cancels.
  import { KIND_COLORS, KIND_NAMES } from "./RefTag.svelte";
  import type { PickOption } from "../../lib/regionDoc.svelte";

  interface Props {
    options: PickOption[];
    placeholder?: string;
    exclude?: string[];
    onpick: (id: string) => void;
    oncancel: () => void;
  }
  let { options, placeholder = "Search…", exclude = [], onpick, oncancel }: Props = $props();

  let query = $state("");
  let active = $state(0);
  let input: HTMLInputElement;
  $effect(() => input.focus());

  const results = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const words = q.split(/\s+/).filter(Boolean);
    const skip = new Set(exclude);
    // Best first: name starts with the search, then name contains it, then
    // anything else that matches (ids, descriptions).
    const ranked: [number, PickOption][] = [];
    for (const o of options) {
      if (skip.has(o.id)) continue;
      const label = o.label.toLowerCase();
      const hay = `${label} ${o.id} ${o.detail}`.toLowerCase();
      if (!words.every((w) => hay.includes(w))) continue;
      const rank = !q ? 1 : label.startsWith(q) ? 0 : words.every((w) => label.includes(w)) ? 1 : 2;
      ranked.push([rank, o]);
    }
    ranked.sort((a, b) => a[0] - b[0]);
    return ranked.slice(0, 60).map(([, o]) => o);
  });

  function key(e: KeyboardEvent) {
    if (e.key === "ArrowDown") (active = Math.min(active + 1, results.length - 1)), e.preventDefault();
    else if (e.key === "ArrowUp") (active = Math.max(active - 1, 0)), e.preventDefault();
    else if (e.key === "Enter" && results[active]) onpick(results[active].id);
    else if (e.key === "Escape") oncancel();
  }
</script>

<div class="picker card">
  <input class="input" bind:this={input} bind:value={query} {placeholder} onkeydown={key} oninput={() => (active = 0)} />
  <ul role="listbox">
    {#each results as o, i (o.id)}
      <li>
        <button role="option" aria-selected={i === active} class:on={i === active} style:--k={KIND_COLORS[o.kind]} onmouseenter={() => (active = i)} onclick={() => onpick(o.id)}>
          <span class="dot"></span>
          <span class="label">{o.label}</span>
          <span class="kind">{KIND_NAMES[o.kind]}</span>
        </button>
      </li>
    {:else}
      <li class="empty faint small">Nothing matches.</li>
    {/each}
  </ul>
</div>

<style>
  .picker {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px;
    width: 100%;
    max-width: 420px;
    box-shadow: var(--shadow-lg);
  }
  ul {
    max-height: 260px;
    margin: 0;
    padding: 0;
    overflow: auto;
    list-style: none;
  }
  button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 5px 8px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    text-align: left;
  }
  button.on {
    background: color-mix(in srgb, var(--k) 18%, transparent);
  }
  .dot {
    width: 8px;
    height: 8px;
    flex: none;
    border-radius: 50%;
    background: var(--k);
  }
  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .kind {
    font-size: 11px;
    color: var(--text-3);
  }
  .empty {
    padding: 6px 8px;
  }
</style>
