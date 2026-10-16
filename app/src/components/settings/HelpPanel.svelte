<script lang="ts">
  // Explains whichever setting is pointed at, in a bar docked to the bottom
  // of the page: its name and explanation, then one line about the choice
  // that matters (the one pointed at, else the current one), and the
  // default with a reset button. Can be collapsed to one line.
  import Icon from "../Icon.svelte";
  import { app } from "../../lib/state.svelte";

  const def = $derived(app.help ? app.names.settings.get(app.help) : undefined);
  const value = $derived(def ? app.values[def.key] : undefined);
  const inactive = $derived(def ? app.inactive.get(def.key) : undefined);
  const changed = $derived(def ? app.isChanged(def.key) : false);

  function initialCollapsed() {
    try {
      return localStorage.getItem("mmr.helpCollapsed") === "1";
    } catch {
      return false;
    }
  }
  let collapsed = $state(initialCollapsed());
  function toggle() {
    collapsed = !collapsed;
    try {
      localStorage.setItem("mmr.helpCollapsed", collapsed ? "1" : "0");
    } catch {}
  }

  function describe(v: unknown): string {
    if (!def) return "";
    if (def.type === "bool") return v ? "On" : "Off";
    if (v === null) return "Automatic";
    if (Array.isArray(v)) {
      if (!v.length) return "None";
      return v.map((x) => app.optionLabel(def!.key, String(x))).join(", ");
    }
    if (typeof v === "object") return "Standard";
    return app.optionLabel(def.key, String(v));
  }

  /** One line about a choice: pointed at, else the current enum choice. */
  const choice = $derived.by(() => {
    if (!def?.options?.length) return null;
    const pointed = def.options.find((o) => o.value === app.helpOption);
    if (pointed) return { prefix: "", name: app.optionLabel(def.key, pointed.value), text: pointed.description };
    if (def.type === "enum") {
      const o = def.options.find((o) => o.value === value);
      if (o) return { prefix: "Selected:", name: app.optionLabel(def.key, o.value), text: o.description };
    }
    if (def.type === "flags") {
      return { prefix: "Selected:", name: describe(value), text: "Point at one to see what it does." };
    }
    return null;
  });
  const showDefault = $derived(!!def && def.type !== "custom" && def.type !== "tricks");
</script>

<aside class="dock" class:collapsed aria-label="About this option" aria-live="polite">
  <div class="text">
    <div class="title">
      <Icon name="info" size={15} />
      {#if def}
        <strong>{app.label(def.key)}</strong>
        {#if collapsed && def.description}<span class="one-line muted">{def.description}</span>{/if}
      {:else}
        <strong>Point at any option</strong>
        <span class="muted">to see what it does. <span class="dot"></span> marks options you've changed.</span>
      {/if}
    </div>
    {#if !collapsed && def}
      {#if def.description}<p class="desc">{def.description}</p>{/if}
      {#if choice}
        <p class="choice">
          {#if choice.prefix}<span class="faint">{choice.prefix}</span>{/if}
          <strong>{choice.name}</strong>{#if choice.text}<span class="muted">{` — ${choice.text}`}</span>{/if}
        </p>
      {/if}
      {#if inactive}<p class="note small"><Icon name="alert" size={13} /> {inactive.message}.</p>{/if}
    {/if}
  </div>

  <div class="side">
    {#if showDefault && !collapsed}
      <span class="small muted">Default: {describe(app.schema!.defaults[def!.key])}</span>
      {#if changed}
        <button class="btn small" onclick={() => app.setValue(def!.key, structuredClone(app.schema!.defaults[def!.key]))}>
          <Icon name="reset" size={13} /> Reset
        </button>
      {/if}
    {/if}
    <button class="icon-btn" aria-label={collapsed ? "Show explanations" : "Hide explanations"} title={collapsed ? "Show more" : "Show less"} onclick={toggle}>
      <Icon name="chevron-down" size={16} />
    </button>
  </div>
</aside>

<style>
  .dock {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: start;
    gap: 24px;
    min-height: 92px;
    padding: 12px 20px 14px;
    border-top: 1px solid color-mix(in srgb, var(--sec) 40%, var(--border));
    background: color-mix(in srgb, var(--surface) 82%, transparent);
    backdrop-filter: blur(14px);
    box-shadow: 0 -10px 30px rgba(3, 0, 12, 0.35);
  }
  .dock.collapsed {
    align-items: center;
    min-height: 0;
    padding: 8px 20px;
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
    max-width: 980px;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .title > :global(svg) {
    color: var(--sec);
  }
  .one-line {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .desc,
  .choice {
    margin: 0;
    line-height: 1.5;
  }
  .desc {
    color: var(--text-2);
  }
  .choice {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .choice .faint {
    margin-right: 6px;
  }
  .note {
    display: flex;
    gap: 6px;
    margin: 0;
    color: var(--info);
  }
  .side {
    display: flex;
    align-items: center;
    gap: 10px;
    white-space: nowrap;
  }
  .side .icon-btn :global(svg) {
    transition: transform 0.15s;
  }
  .collapsed .side .icon-btn :global(svg) {
    transform: rotate(180deg);
  }
</style>
