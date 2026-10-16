<script lang="ts" module>
  import type { RefKind } from "../../lib/regionDoc.svelte";

  export const KIND_COLORS: Record<RefKind, string> = {
    item: "#5aa7f2",
    region: "#62c254",
    helper: "#a58bff",
    trick: "#ff8a4f",
    setting: "#2fc4a2",
    event: "#ee78b0",
    location: "#b8aed6",
    unknown: "#ff6b78",
  };

  export const KIND_NAMES: Record<RefKind, string> = {
    item: "Item",
    region: "Region",
    helper: "Helper",
    trick: "Trick",
    setting: "Setting",
    event: "Event",
    location: "Check",
    unknown: "Not found",
  };
</script>

<script lang="ts">
  // One thing a rule refers to, coloured by kind. Click to open it.
  import Icon from "../Icon.svelte";
  import { doc } from "../../lib/regionDoc.svelte";

  let { id, onremove, small = false }: { id: string; onremove?: () => void; small?: boolean } = $props();

  const kind = $derived(doc.kindOf(id));
  const name = $derived(doc.refName(id));
  const canOpen = $derived(kind === "region" || kind === "helper" || kind === "trick" || kind === "setting" || kind === "event");
</script>

<span class="tag" class:small class:unknown={kind === "unknown"} style:--k={KIND_COLORS[kind]} title="{KIND_NAMES[kind]}: {id}">
  {#if canOpen}
    <button class="name" onclick={(e) => (e.stopPropagation(), doc.follow(id))}>{name}</button>
  {:else}
    <span class="name">{name}</span>
  {/if}
  {#if onremove}
    <button class="x" aria-label="Remove {name}" onclick={(e) => (e.stopPropagation(), onremove())}><Icon name="x" size={11} /></button>
  {/if}
</span>

<style>
  .tag {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    max-width: 100%;
    padding: 1px 8px;
    border-radius: 999px;
    border: 1px solid color-mix(in srgb, var(--k) 45%, transparent);
    background: color-mix(in srgb, var(--k) 16%, transparent);
    color: color-mix(in srgb, var(--k) 70%, var(--text));
    font-size: 12.5px;
    line-height: 1.6;
    white-space: nowrap;
  }
  .tag.small {
    font-size: 11.5px;
    padding: 0 7px;
  }
  .tag.unknown {
    border-style: dashed;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    padding: 0;
    border: none;
    background: none;
    color: inherit;
    font: inherit;
  }
  button.name:hover {
    text-decoration: underline;
  }
  .x {
    display: inline-grid;
    place-items: center;
    width: 15px;
    height: 15px;
    margin-right: -4px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: transparent;
    color: inherit;
    opacity: 0.6;
  }
  .x:hover {
    opacity: 1;
    background: color-mix(in srgb, var(--k) 30%, transparent);
  }
</style>
