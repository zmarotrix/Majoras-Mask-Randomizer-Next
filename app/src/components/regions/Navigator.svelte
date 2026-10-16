<script lang="ts">
  // Left column: regions grouped by area, helpers, tricks and settings.
  import Icon from "../Icon.svelte";
  import { doc, REGION_PREFIX, type NavTab } from "../../lib/regionDoc.svelte";
  import { humanize } from "../../lib/names";

  const TABS: { id: NavTab; label: string }[] = [
    { id: "regions", label: "Regions" },
    { id: "helpers", label: "Helpers" },
    { id: "tricks", label: "Tricks" },
    { id: "settings", label: "Settings" },
  ];

  let open = $state<Record<string, boolean>>({});

  const words = $derived(doc.query.toLowerCase().split(/\s+/).filter(Boolean));
  const match = (text: string) => words.every((w) => text.toLowerCase().includes(w));

  /** A region matches by its name, or by any check or event inside it. */
  function regionMatches(r: (typeof doc.regions)[number]) {
    if (!words.length) return true;
    const text = [r.name, ...(r.locations ?? []).map((s) => `${doc.spotName(s.id)} ${s.id}`), ...(r.events ?? []).map((s) => humanize(s.id))].join(" ");
    return match(text);
  }

  const groups = $derived(doc.groups.map(([g, list]) => [g, list.filter(regionMatches)] as const).filter(([, list]) => list.length));
  const entries = $derived(
    (doc.file?.entries ?? []).filter((e) => {
      const k = doc.kindOf(e.id);
      const want = doc.nav === "tricks" ? "trick" : doc.nav === "settings" ? "setting" : null;
      if (want ? k !== want : k === "trick" || k === "setting") return false;
      return match(`${e.id} ${e.trick?.description ?? ""} ${e.note ?? ""}`);
    }),
  );
  const counts = $derived({
    regions: doc.regions.length,
    helpers: (doc.file?.entries ?? []).filter((e) => !["trick", "setting"].includes(doc.kindOf(e.id))).length,
    tricks: (doc.file?.entries ?? []).filter((e) => e.trick).length,
    settings: (doc.file?.entries ?? []).filter((e) => e.setting).length,
  });

  const currentGroup = $derived(doc.page?.kind === "region" ? doc.groups.find(([, l]) => l.some((r) => r.name === (doc.page as { name: string }).name))?.[0] : undefined);
  const isOpen = (g: string) => words.length > 0 || (open[g] ?? g === currentGroup);
  const problems = (id: string) => doc.issues.filter((i) => i.entry === id && i.severity !== "info").length;

  const NEW_LABEL: Record<NavTab, string> = { regions: "New region", helpers: "New helper", tricks: "New trick", settings: "New setting rule" };
  function create() {
    const what = NEW_LABEL[doc.nav].slice(4);
    const name = prompt(`Name of the ${what}:`)?.trim();
    if (!name) return;
    if (doc.nav === "regions") {
      if (doc.regionByName.has(name)) return alert("There's already a region with that name.");
      return doc.addRegion(name);
    }
    if (doc.entryById.has(name) || doc.kindOf(name) !== "unknown") return alert("That name is already used by something else.");
    if (doc.nav === "tricks") doc.addEntry({ id: name, trick: {} });
    else if (doc.nav === "settings") doc.addEntry({ id: name, setting: { setting: "CloseCows" } });
    else doc.addEntry({ id: name });
  }
</script>

<nav class="nav">
  <div class="search">
    <input class="input" placeholder="Find a region, check, helper…" bind:value={doc.query} />
  </div>
  <div class="tabs" role="tablist">
    {#each TABS as t (t.id)}
      <button role="tab" aria-selected={doc.nav === t.id} class:on={doc.nav === t.id} onclick={() => (doc.nav = t.id)}>
        <span class="tl">{t.label}</span><span class="n">{counts[t.id]}</span>
      </button>
    {/each}
  </div>

  <button class="new" onclick={create}>+ {NEW_LABEL[doc.nav]}</button>

  <div class="list scroll">
    {#if doc.nav === "regions"}
      {#each groups as [g, list] (g)}
        <button class="group" onclick={() => (open[g] = !isOpen(g))} aria-expanded={isOpen(g)}>
          <span class="chev" class:rot={isOpen(g)}><Icon name="chevron-right" size={13} /></span>
          <span class="gname">{g}</span>
          <span class="faint small">{list.length}</span>
        </button>
        {#if isOpen(g)}
          {#each list as r (r.name)}
            {@const n = problems(REGION_PREFIX + r.name)}
            <button
              class="item"
              class:on={doc.page?.kind === "region" && doc.page.name === r.name}
              onclick={() => doc.go({ kind: "region", name: r.name })}
              title={r.name}
            >
              <span class="iname">{r.name}</span>
              {#if n}<span class="badge bad">{n}</span>{/if}
              <span class="faint small">{(r.locations ?? []).length || ""}</span>
            </button>
          {/each}
        {/if}
      {:else}
        <p class="faint small pad">No regions match.</p>
      {/each}
    {:else}
      {#each entries as e (e.id)}
        {@const n = problems(e.id)}
        <button class="item" class:on={doc.page?.kind === "entry" && doc.page.id === e.id} onclick={() => doc.go({ kind: "entry", id: e.id })} title={e.trick?.description ?? e.id}>
          <span class="iname">{humanize(e.id)}</span>
          {#if n}<span class="badge bad">{n}</span>{/if}
        </button>
      {:else}
        <p class="faint small pad">Nothing matches.</p>
      {/each}
    {/if}
  </div>
</nav>

<style>
  .nav {
    display: flex;
    flex-direction: column;
    min-height: 0;
    height: 100%;
    border-right: 1px solid var(--border);
    background: color-mix(in srgb, var(--surface) 80%, transparent);
  }
  .search {
    padding: 12px 10px 8px;
  }
  .tabs {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 4px;
    padding: 0 10px 10px;
    border-bottom: 1px solid var(--border);
  }
  .tabs button {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0;
    padding: 5px 2px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text-2);
    font-size: 12px;
    font-weight: 500;
    line-height: 1.2;
  }
  .tabs button:hover {
    color: var(--text);
  }
  .tabs button.on {
    border-color: var(--sec);
    background: color-mix(in srgb, var(--sec) 18%, transparent);
    color: var(--text);
  }
  .n {
    font-size: 10.5px;
    color: var(--text-3);
  }
  .new {
    margin: 8px 10px 0;
    padding: 5px 8px;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-sm);
    background: none;
    color: var(--accent);
    font-size: 12.5px;
    text-align: left;
  }
  .new:hover {
    border-color: var(--accent);
  }
  .list {
    flex: 1;
    min-height: 0;
    padding: 6px;
  }
  .group {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    margin-top: 4px;
    padding: 4px 6px;
    border: none;
    background: none;
    color: var(--text-2);
    font-size: 12px;
    font-weight: 600;
    text-align: left;
  }
  .gname {
    flex: 1;
  }
  .chev {
    display: inline-grid;
    transition: transform 0.12s;
  }
  .chev.rot {
    transform: rotate(90deg);
  }
  .item {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 4px 8px 4px 22px;
    border: none;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-2);
    font-size: 13px;
    text-align: left;
  }
  .item:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .item.on {
    background: color-mix(in srgb, var(--sec) 20%, transparent);
    color: var(--text);
  }
  .iname {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pad {
    padding: 8px;
  }
</style>
