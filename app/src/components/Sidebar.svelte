<script lang="ts">
  import Icon from "./Icon.svelte";
  import { app } from "../lib/state.svelte";
  import { sectionColor } from "../lib/sections";
  import { dialogs, openLogicEditor, openManual, openTracker } from "../lib/actions.svelte";

  const sections = $derived(app.schema!.layout.sections);
  const POOL_KEYS = ["CustomItemList", "CustomJunkLocations", "CustomStartingItemList"];

  function keysOf(id: string): string[] {
    const s = sections.find((x) => x.id === id);
    const keys = s ? s.groups.flatMap((g) => [...g.settings, ...(g.more?.settings ?? [])]) : [];
    return id === "items" ? [...keys, ...POOL_KEYS] : keys;
  }
  const changedIn = (id: string) => keysOf(id).filter((k) => app.isChanged(k)).length;
  const issuesIn = (id: string) => {
    const keys = new Set(keysOf(id));
    return app.settingsIssues.filter((i) => keys.has(i.key)).length;
  };

  function search(q: string) {
    app.query = q;
    if (q.trim()) app.screen = "search";
    else if (app.screen === "search") app.screen = "quick";
  }
</script>

<nav class="side" class:open={app.navOpen}>
  <div class="search">
    <input class="input" placeholder="Search settings…" value={app.query} oninput={(e) => search(e.currentTarget.value)} />
  </div>

  {#each sections as s, i (s.id)}
    {#if i === 1}<div class="label">Settings</div>{/if}
    {@const n = changedIn(s.id)}
    {@const bad = issuesIn(s.id)}
    <button
      class="item"
      class:on={app.screen === s.id}
      class:first={i === 0}
      style:--sec={sectionColor(s.id)}
      onclick={() => app.go(s.id)}
      title={s.description}
    >
      <span class="tile" class:dim={app.screen !== s.id}><Icon name={s.icon} size={15} /></span>
      <span class="name">{s.label}</span>
      {#if bad}<span class="badge bad">{bad}</span>{/if}
      {#if n && s.id !== "quick"}<span class="dot" title="{n} changed from default"></span>{/if}
    </button>
  {/each}

  <div class="label">Tools</div>
  <button class="item" class:on={app.screen === "logic-editor"} style:--sec={sectionColor("logic-editor")} onclick={openLogicEditor} title="Opens in its own window">
    <span class="tile" class:dim={app.screen !== "logic-editor"}><Icon name="branch" size={15} /></span>
    <span class="name">Logic editor</span>
  </button>
  <button
    class="item"
    style:--sec={sectionColor("tracker")}
    onclick={openTracker}
    title={dialogs.lastTracker ? "Open the last seed's tracker in your browser" : "Open a seed's tracker (.html) in your browser. Every seed comes with one."}
  >
    <span class="tile dim"><Icon name="map" size={15} /></span>
    <span class="name">Seed tracker</span>
  </button>
  <button class="item" style:--sec={sectionColor("manual")} onclick={openManual} title="Every setting, preset and trick, explained (opens in your browser)">
    <span class="tile dim"><Icon name="help" size={15} /></span>
    <span class="name">Manual</span>
  </button>

  <div class="spacer"></div>
  <div class="faint small version">MMR Next preview</div>
</nav>
{#if app.navOpen}
  <button class="scrim" aria-label="Close menu" onclick={() => (app.navOpen = false)}></button>
{/if}

<style>
  .side {
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 12px 10px;
    background: color-mix(in srgb, var(--surface) 70%, transparent);
    backdrop-filter: blur(10px);
    border-right: 1px solid var(--border);
    overflow: auto;
  }
  .search {
    margin-bottom: 8px;
  }
  .label {
    padding: 14px 10px 4px;
    font-size: 11.5px;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-3);
  }
  .item {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 5px 10px 5px 8px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    font-weight: 500;
    text-align: left;
  }
  .item .name {
    flex: 1;
  }
  .item:hover:not(:disabled) {
    background: var(--surface-2);
    color: var(--text);
  }
  .item.on {
    background: linear-gradient(90deg, color-mix(in srgb, var(--sec) 24%, transparent), color-mix(in srgb, var(--sec) 6%, transparent));
    color: var(--text);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--sec) 30%, transparent);
  }
  .item.on::before {
    content: "";
    position: absolute;
    left: -10px;
    top: 7px;
    bottom: 7px;
    width: 3px;
    border-radius: 0 3px 3px 0;
    background: var(--sec);
    box-shadow: 0 0 10px var(--sec);
  }
  .tile {
    width: 26px;
    height: 26px;
    border-radius: 7px;
  }
  .item.first:not(.on) {
    color: var(--gold);
  }
  .version {
    padding: 8px 10px 0;
  }
  .scrim {
    display: none;
  }
  /* Narrow screens: the sidebar slides in over the content. */
  @media (max-width: 760px) {
    .side {
      position: fixed;
      top: 0;
      bottom: 0;
      left: 0;
      z-index: 60;
      width: 260px;
      transform: translateX(-100%);
      transition: transform 0.18s ease-out;
      box-shadow: var(--shadow-lg);
    }
    .side.open {
      transform: none;
    }
    .scrim {
      display: block;
      position: fixed;
      inset: 0;
      z-index: 55;
      border: none;
      background: rgba(10, 6, 24, 0.4);
    }
  }
</style>
