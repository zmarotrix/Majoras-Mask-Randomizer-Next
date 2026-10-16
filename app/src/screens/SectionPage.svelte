<script lang="ts">
  // A settings section (from data/layout.json), or search results across all
  // sections, with the explanation of the pointed-at option docked below.
  import SettingGroup from "../components/settings/SettingGroup.svelte";
  import GroupForm from "../components/settings/GroupForm.svelte";
  import Icon from "../components/Icon.svelte";
  import { sectionColor } from "../lib/sections";
  import HelpPanel from "../components/settings/HelpPanel.svelte";
  import { app } from "../lib/state.svelte";
  import type { LayoutSection, SettingDef } from "../lib/types";

  const layout = $derived(app.schema!.layout);
  const searching = $derived(app.screen === "search");
  const section = $derived(layout.sections.find((s) => s.id === app.screen));

  function matches(def: SettingDef, words: string[]): boolean {
    const hay = [app.label(def.key), def.label, def.description ?? "", ...(def.options ?? []).flatMap((o) => [o.label, o.description ?? ""])]
      .join(" ")
      .toLowerCase();
    return words.every((w) => hay.includes(w));
  }

  /** Search results: the sections and groups containing matches. */
  const results = $derived.by(() => {
    if (!searching) return [];
    const words = app.query.toLowerCase().split(/\s+/).filter(Boolean);
    const hits = new Set(app.schema!.settings.filter((d) => matches(d, words)).map((d) => d.key));
    const out: { section: LayoutSection; groups: { group: LayoutSection["groups"][number]; only: Set<string> }[] }[] = [];
    for (const s of layout.sections) {
      const groups = s.groups
        .map((g) => ({ group: g, only: new Set([...g.settings, ...(g.more?.settings ?? [])].filter((k) => hits.has(k))) }))
        .filter((g) => g.only.size);
      if (groups.length) out.push({ section: s, groups });
    }
    return out;
  });
  const poolHit = $derived(searching && /pool|shuffle|junk|starting|location|check/i.test(app.query));
  const keysOf = (g: LayoutSection["groups"][number]) => [...g.settings, ...(g.more?.settings ?? [])];
  const changedIn = (g: LayoutSection["groups"][number]) => keysOf(g).filter((k) => app.isChanged(k)).length;
  const group = $derived(section ? (section.groups.find((g) => g.id === app.groupTab[section.id]) ?? section.groups[0]) : undefined);
  const chooseGroup = (id: string) => {
    app.groupTab[section!.id] = id;
    app.help = null;
  };
  const changedHere = $derived(
    section ? section.groups.flatMap((g) => [...g.settings, ...(g.more?.settings ?? [])]).filter((k) => app.isChanged(k)).length : 0,
  );
</script>

<div class="page" style:--sec={section ? sectionColor(section.id) : undefined}>
  <div class="content scroll">
    <div class="main">
      {#if searching}
        <header>
          <h1>Results for “{app.query.trim()}”</h1>
        </header>
        {#if poolHit}
          <button class="card jump" onclick={() => app.go("items")}>
            Shuffled locations, junk locations and starting items are on the <strong>Item pool</strong> page →
          </button>
        {/if}
        {#each results as r (r.section.id)}
          <h2 class="group-title">{r.section.label}</h2>
          {#each r.groups as g (g.group.id)}
            <SettingGroup group={g.group} only={g.only} />
          {/each}
        {:else}
          {#if !poolHit}<p class="muted">Nothing found. Try other words.</p>{/if}
        {/each}
      {:else if section}
        <header class="head">
          <span class="tile big"><Icon name={section.icon} size={21} /></span>
          <div>
            <h1>{section.label}</h1>
            <p class="muted">
              {section.description}
              {#if changedHere}<span class="changed"> · {changedHere} changed</span>{/if}
            </p>
          </div>
        </header>
        {#if group}
          <div class="panel card">
            <div class="tabs" role="tablist" aria-label="{section.label} groups">
              {#each section.groups as g (g.id)}
                {@const n = changedIn(g)}
                <button role="tab" aria-selected={g.id === group.id} class:on={g.id === group.id} onclick={() => chooseGroup(g.id)}>
                  <Icon name={g.icon} size={15} />
                  {g.title}
                  {#if n}<span class="count" title="{n} changed from the default">{n}</span>{/if}
                </button>
              {/each}
            </div>
            <div class="body" role="tabpanel">
              <p class="muted small desc">{group.description}</p>
              {#key group.id}<GroupForm {group} />{/key}
            </div>
          </div>
        {/if}
      {/if}
    </div>
  </div>
  <HelpPanel />
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .content {
    flex: 1;
    min-height: 0;
    padding: 24px 28px 32px;
  }
  .main {
    max-width: 1600px;
  }
  header {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 16px;
  }
  .head {
    flex-direction: row;
    align-items: center;
    gap: 14px;
  }
  .changed {
    color: var(--gold);
  }
  .panel {
    position: relative;
    padding: 0;
    overflow: hidden;
    border-top: 2px solid var(--sec);
    box-shadow:
      var(--shadow),
      0 -1px 24px -8px color-mix(in srgb, var(--sec) 60%, transparent);
  }
  .tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
    padding: 6px 10px 0;
    border-bottom: 1px solid var(--border);
    background: linear-gradient(180deg, color-mix(in srgb, var(--sec) 10%, var(--surface-2)), var(--surface-2));
  }
  .tabs button {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    margin-bottom: -1px;
    padding: 9px 12px 10px;
    border: none;
    border-bottom: 2px solid transparent;
    background: none;
    color: var(--text-2);
    font-size: 13.5px;
    font-weight: 500;
  }
  .tabs button:hover {
    color: var(--text);
  }
  .tabs button.on {
    border-bottom-color: var(--sec);
    color: var(--text);
    text-shadow: 0 0 12px color-mix(in srgb, var(--sec) 50%, transparent);
  }
  .tabs button.on :global(svg) {
    color: var(--sec);
  }
  .count {
    display: inline-grid;
    place-items: center;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: 999px;
    background: var(--gold);
    color: #2a1d05;
    font-size: 11px;
    font-weight: 700;
  }
  .body {
    padding: 16px 20px 22px;
  }
  .desc {
    margin: 0 0 16px;
  }
  .group-title {
    margin: 16px 0 8px;
    font-size: 12.5px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-3);
  }
  .jump {
    display: block;
    width: 100%;
    margin-bottom: 12px;
    padding: 12px 16px;
    text-align: left;
    box-shadow: none;
  }
  .jump:hover {
    border-color: var(--accent);
  }
  @media (max-width: 640px) {
    .content {
      padding: 16px 16px 24px;
    }
  }
</style>
