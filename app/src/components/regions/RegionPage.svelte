<script lang="ts">
  // A region: how you get in, where you can go, and what's here.
  import RefTag from "./RefTag.svelte";
  import RuleView from "./RuleView.svelte";
  import Picker from "./Picker.svelte";
  import Icon from "../Icon.svelte";
  import { doc, REGION_PREFIX, type Target } from "../../lib/regionDoc.svelte";
  import { humanize } from "../../lib/names";
  import type { Spot, Time } from "../../lib/types";

  let { name }: { name: string } = $props();

  const region = $derived(doc.regionByName.get(name));
  const waysIn = $derived(doc.waysIn.get(name) ?? []);
  const group = $derived(doc.groups.find(([, l]) => l.some((r) => r.name === name))?.[0]);
  const isStart = $derived(doc.file?.start === name);
  const ids = $derived(new Set([REGION_PREFIX + name, ...(region?.locations ?? []).map((s) => s.id), ...(region?.events ?? []).map((s) => s.id)]));
  const problems = $derived(doc.issues.filter((i) => i.entry && ids.has(i.entry) && i.severity !== "info"));

  let adding = $state<"exit" | "location" | "event" | null>(null);
  let newEvent = $state("");

  const same = (a: Target | null, b: Target) => !!a && JSON.stringify(a) === JSON.stringify(b);
  const select = (t: Target) => (doc.target = t);

  const PERIODS: Record<string, string> = { Day1: "D1", Night1: "N1", Day2: "D2", Night2: "N2", Day3: "D3", Night3: "N3" };
  function timeTag(t: Time | undefined): string {
    if (!t) return "";
    const parts: string[] = [];
    if (t.available?.length) parts.push(t.available.map((p) => PERIODS[p]).join(" "));
    if (t.setup?.length) parts.push("set up " + t.setup.map((p) => PERIODS[p]).join(" "));
    if (t.no_return) parts.push("no return");
    if (t.opening_hours) parts.push("opening hours");
    return parts.join(" · ");
  }
  const spotTime = (s: Spot) => timeTag(s.time);

  function addEvent() {
    const id = newEvent.trim();
    if (!id) return;
    doc.addSpot(name, "event", id);
    newEvent = "";
    adding = null;
  }
  function addExit(to: string) {
    adding = null;
    doc.addExit(name, to);
  }
  function newRegion() {
    const n = prompt("Name of the new region (it will be connected from here):", `${name}: `);
    if (n?.trim()) {
      if (doc.regionByName.has(n.trim())) return alert("There's already a region with that name.");
      doc.addRegion(n.trim(), name);
    }
  }
</script>

{#if region}
  <div class="page">
    <header>
      <div class="crumb faint small">{group ?? ""}</div>
      <div class="title-row">
        <h1>{name}</h1>
        {#if isStart}<span class="badge gold">Start</span>{/if}
        {#if timeTag(region.time)}<span class="badge">{timeTag(region.time)}</span>{/if}
        <span class="spacer"></span>
        <button class="btn small" onclick={() => select({ kind: "region", region: name })}><Icon name="sliders" size={13} /> Region settings</button>
      </div>
    </header>

    {#if problems.length}
      <div class="problems">
        {#each problems as p, i (i)}<p class:err={p.severity === "error"}><Icon name="alert" size={13} /> {p.message}</p>{/each}
      </div>
    {/if}

    <section>
      <h2>Ways in <span class="faint">{waysIn.length}</span></h2>
      {#each waysIn as w (w.from + w.index)}
        <div class="row" role="button" tabindex="0" class:on={same(doc.target, { kind: "exit", region: w.from, index: w.index })} onclick={() => doc.go({ kind: "region", name: w.from }, { kind: "exit", region: w.from, index: w.index })} onkeydown={(e) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), e.currentTarget.click())}>
          <span class="what"><span class="lead">from</span> <RefTag id={REGION_PREFIX + w.from} small /></span>
          <RuleView rule={w.rule} />
        </div>
      {:else}
        <p class="faint small">{isStart ? "This is where the game starts." : "Nothing leads here yet."}</p>
      {/each}
    </section>

    <section>
      <h2>
        Ways out <span class="faint">{(region.exits ?? []).length}</span>
        <button class="add" onclick={() => (adding = adding === "exit" ? null : "exit")}>+ Add exit</button>
      </h2>
      {#if adding === "exit"}
        <div class="adder">
          <Picker options={doc.regionOptions} exclude={[name]} placeholder="Which region does it lead to?" onpick={addExit} oncancel={() => (adding = null)} />
          <button class="btn small" onclick={newRegion}>New region…</button>
        </div>
      {/if}
      {#each region.exits ?? [] as e, index (index)}
        {@const t = { kind: "exit", region: name, index } as Target}
        <div class="row" role="button" tabindex="0" class:on={same(doc.target, t)} onclick={() => select(t)} onkeydown={(e) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), e.currentTarget.click())}>
          <span class="what"><span class="lead">to</span> <RefTag id={REGION_PREFIX + e.to} small /></span>
          <RuleView rule={e.rule} />
        </div>
      {/each}
    </section>

    <section>
      <h2>
        Checks <span class="faint">{(region.locations ?? []).length}</span>
        <button class="add" onclick={() => (adding = adding === "location" ? null : "location")}>+ Add check</button>
      </h2>
      {#if adding === "location"}
        <div class="adder">
          <Picker options={doc.freeLocations} placeholder="Find a check that isn't in a region yet…" onpick={(id) => ((adding = null), doc.addSpot(name, "location", id))} oncancel={() => (adding = null)} />
        </div>
      {/if}
      {#each region.locations ?? [] as s, index (s.id)}
        {@const t = { kind: "location", region: name, index } as Target}
        <div class="row" role="button" tabindex="0" class:on={same(doc.target, t)} onclick={() => select(t)} onkeydown={(e) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), e.currentTarget.click())}>
          <span class="what">
            <span class="spot">{doc.spotName(s.id)}</span>
            {#if spotTime(s)}<span class="badge time">{spotTime(s)}</span>{/if}
          </span>
          <RuleView rule={s.rule} />
        </div>
      {/each}
    </section>

    <section>
      <h2>
        Events <span class="faint">{(region.events ?? []).length}</span>
        <button class="add" onclick={() => (adding = adding === "event" ? null : "event")}>+ Add event</button>
      </h2>
      {#if adding === "event"}
        <div class="adder">
          <input class="input" placeholder="Name of the event, e.g. Defeat the Gekko" bind:value={newEvent} onkeydown={(e) => e.key === "Enter" && addEvent()} />
          <button class="btn small primary" onclick={addEvent}>Add</button>
        </div>
      {/if}
      {#each region.events ?? [] as s, index (s.id)}
        {@const t = { kind: "event", region: name, index } as Target}
        <div class="row" role="button" tabindex="0" class:on={same(doc.target, t)} onclick={() => select(t)} onkeydown={(e) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), e.currentTarget.click())}>
          <span class="what">
            <span class="spot">{humanize(s.id)}</span>
            {#if spotTime(s)}<span class="badge time">{spotTime(s)}</span>{/if}
          </span>
          <RuleView rule={s.rule} />
        </div>
      {:else}
        <p class="faint small">Events are things that happen here that other rules can need (a boss beaten, water drained…).</p>
      {/each}
    </section>
  </div>
{:else}
  <p class="muted">This region doesn't exist.</p>
{/if}

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 18px;
    padding: 20px 24px 40px;
  }
  .crumb {
    margin-bottom: 2px;
  }
  .title-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  h1 {
    font-size: 21px;
  }
  .badge.time {
    font-weight: 500;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-2);
  }
  .add {
    margin-left: auto;
    padding: 0;
    border: none;
    background: none;
    color: var(--accent);
    font-size: 12.5px;
    font-weight: 500;
    text-transform: none;
    letter-spacing: 0;
  }
  .row {
    display: grid;
    grid-template-columns: minmax(150px, 240px) minmax(0, 1fr);
    align-items: start;
    gap: 6px 16px;
    width: 100%;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--sheen), var(--surface);
    color: var(--text);
    text-align: left;
  }
  .row {
    cursor: pointer;
  }
  .row:hover {
    border-color: var(--border-strong);
  }
  .row.on {
    border-color: var(--sec);
    box-shadow: 0 0 0 1px var(--sec), 0 0 16px -6px var(--sec);
  }
  .lead {
    font-size: 12px;
    color: var(--text-3);
  }
  .what {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 6px;
    min-width: 0;
  }
  .spot {
    font-weight: 500;
  }
  .adder {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }
  .adder .input {
    max-width: 420px;
  }
  .problems {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    background: var(--warn-soft);
    color: var(--warn);
    font-size: 13px;
  }
  .problems p {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .problems .err {
    color: var(--bad);
  }
</style>
