<script lang="ts">
  import PoolEditor from "../components/pool/PoolEditor.svelte";
  import SettingGroup from "../components/settings/SettingGroup.svelte";
  import { app } from "../lib/state.svelte";
  import { locationChecks, shuffledChecks, startingItemChecks } from "../lib/pool";

  type Tab = "shuffled" | "junk" | "starting";
  let tab = $state<Tab>("shuffled");

  const locations = $derived(locationChecks(app.game!));
  const starting = $derived(startingItemChecks(app.game!));
  const list = (key: string) => (app.values[key] as string[] | undefined) ?? [];
  // Only shuffled checks can be junk locations.
  const junkable = $derived(shuffledChecks(app.game!, list("CustomItemList")));
  const junkIds = $derived(new Set(junkable.map((c) => c.id)));
  const ignoredJunk = $derived(list("CustomJunkLocations").filter((id) => !junkIds.has(id)));

  const rules = $derived(app.schema!.layout.sections.find((s) => s.id === "items")?.groups ?? []);

  const tabs: { id: Tab; label: string; key: string }[] = [
    { id: "shuffled", label: "Shuffled locations", key: "CustomItemList" },
    { id: "junk", label: "Junk locations", key: "CustomJunkLocations" },
    { id: "starting", label: "Starting items", key: "CustomStartingItemList" },
  ];
</script>

<div class="page scroll">
  <header>
    <h1>Item pool</h1>
    <p class="muted">Choose which checks are shuffled, which can only hold junk, and what you start with.</p>
  </header>

  <div class="tabs" role="tablist">
    {#each tabs as t (t.id)}
      <button role="tab" aria-selected={tab === t.id} class:on={tab === t.id} onclick={() => (tab = t.id)}>
        {t.label}
        <span class="n">{t.id === "junk" ? list(t.key).length - ignoredJunk.length : list(t.key).length}</span>
        {#if app.isChanged(t.key)}<span class="dot" title="Changed from default"></span>{/if}
      </button>
    {/each}
  </div>

  {#if tab === "shuffled"}
    <div class="rules">
      {#each rules as group (group.id)}<SettingGroup {group} />{/each}
    </div>
    <p class="intro muted">Shuffled checks can hold any item. Everything else keeps its normal (vanilla) item.</p>
    <section class="card pool">
    <PoolEditor
      id="shuffled"
      kind="locations"
      checks={locations}
      value={list("CustomItemList")}
      onchange={(v) => app.setValue("CustomItemList", v)}
      views={["simple", "grid", "browse", "lists"]}
      labels={{ on: "Shuffled", off: "Vanilla", add: "Shuffle", remove: "Make vanilla", noun: "checks" }}
    />
    </section>
  {:else if tab === "junk"}
    <p class="intro muted">Junk locations never hold an item you need to beat the game. Only shuffled checks are listed.</p>
    {#if ignoredJunk.length}
      <p class="small faint">
        {ignoredJunk.length} other junk location{ignoredJunk.length === 1 ? " isn't" : "s aren't"} shuffled, so {ignoredJunk.length === 1
          ? "it has"
          : "they have"} no effect.
        <button class="link" onclick={() => app.setValue("CustomJunkLocations", list("CustomJunkLocations").filter((id) => junkIds.has(id)))}>Remove</button>
      </p>
    {/if}
    <section class="card pool">
    <PoolEditor
      id="junk"
      kind="locations"
      checks={junkable}
      value={list("CustomJunkLocations")}
      onchange={(v) => app.setValue("CustomJunkLocations", v)}
      views={["lists"]}
      labels={{ on: "Junk", off: "Normal", add: "Make junk", remove: "Make normal", noun: "checks" }}
    />
    </section>
  {:else}
    <p class="intro muted">You'll have these from the start of the game.</p>
    <section class="card pool">
    <PoolEditor
      id="starting"
      kind="items"
      checks={starting}
      value={list("CustomStartingItemList")}
      onchange={(v) => app.setValue("CustomStartingItemList", v)}
      views={["browse", "lists"]}
      labels={{ on: "Starting with", off: "Not starting with", add: "Start with", remove: "Remove", noun: "items" }}
    />
    </section>
  {/if}
</div>

<style>
  .page {
    height: 100%;
    padding: 24px 32px 60px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  header {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .tabs {
    display: flex;
    gap: 4px;
    border-bottom: 1px solid var(--border);
  }
  .tabs button {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 9px 14px;
    border: none;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    background: none;
    color: var(--text-2);
    font-weight: 500;
  }
  .tabs button:hover {
    color: var(--text);
  }
  .tabs button.on {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }
  .n {
    font-size: 12px;
    padding: 0 7px;
    border-radius: 999px;
    background: var(--surface-3);
    color: var(--text-2);
  }
  .rules {
    max-width: 760px;
  }
  .rules :global(.group) {
    margin-bottom: 0;
  }
  .intro {
    margin-top: -4px;
  }
  .link {
    padding: 0;
    border: none;
    background: none;
    color: var(--accent);
    font: inherit;
    text-decoration: underline;
    cursor: pointer;
  }
  .pool {
    padding: 16px 18px 18px;
  }
</style>
