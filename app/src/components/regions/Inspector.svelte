<script lang="ts">
  // Right column: edit the selected exit, check, event, region or helper.
  import RuleBuilder from "./RuleBuilder.svelte";
  import RefTag from "./RefTag.svelte";
  import Picker from "./Picker.svelte";
  import TimeEditor from "../logic/TimeEditor.svelte";
  import Icon from "../Icon.svelte";
  import { doc, entryRule, REGION_PREFIX, type Target } from "../../lib/regionDoc.svelte";
  import { humanize } from "../../lib/names";
  import type { Time } from "../../lib/types";

  const t = $derived(doc.target);
  const info = $derived(t ? doc.ruleOf(t) : null);
  const spot = $derived(t ? doc.spotOf(t) : undefined);
  const exit = $derived(t?.kind === "exit" ? doc.region(t.region)?.exits?.[t.index] : undefined);
  const region = $derived(t?.kind === "region" ? doc.region(t.region) : undefined);
  const entry = $derived(t?.kind === "entry" ? doc.entryById.get(t.id) : undefined);

  const title = $derived.by(() => {
    if (!t) return "";
    if (t.kind === "exit") return `${t.region} → ${exit?.to ?? ""}`;
    if (t.kind === "location") return doc.spotName(spot?.id ?? "");
    if (t.kind === "event") return humanize(spot?.id ?? "");
    if (t.kind === "region") return t.region;
    return t.kind === "entry" ? humanize(t.id) : "";
  });
  const subtitle = $derived(
    !t ? "" : t.kind === "exit" ? "Exit" : t.kind === "location" ? `Check in ${t.region}` : t.kind === "event" ? `Event in ${t.region}` : t.kind === "region" ? "Region" : "Helper",
  );
  const ruleHelp = $derived(
    !t
      ? ""
      : t.kind === "exit"
        ? "What you need to go this way."
        : t.kind === "entry"
          ? "What this helper stands for."
          : "What you need once you're in this region.",
  );

  let changingTarget = $state(false);
  let renaming = $state<string | null>(null);

  const time = $derived<Time>((t?.kind === "region" ? region?.time : t?.kind === "entry" ? entry?.time : spot?.time) ?? {});
  const setTime = (next: Time) => t && doc.setTime(t, next);
  const flag = (key: "no_return" | "opening_hours", on: boolean) => setTime({ ...time, [key]: on || undefined });

  function editSpotList(key: "before" | "temporary_requires", list: string[]) {
    if (!t || (t.kind !== "location" && t.kind !== "event")) return;
    const target = t;
    doc.edit("Edit time rules", (f) => {
      const r = f.regions.find((x) => x.name === target.region);
      const s = (target.kind === "location" ? r?.locations : r?.events)?.[target.index];
      if (s) s[key] = list.length ? list : undefined;
    });
  }
  let addingList = $state<"before" | "temporary_requires" | null>(null);

  function retarget(to: string) {
    changingTarget = false;
    if (t?.kind !== "exit") return;
    const target = t;
    doc.edit("Change exit", (f) => {
      const e = f.regions.find((x) => x.name === target.region)?.exits?.[target.index];
      if (e) e.to = to;
    });
  }
  function rename() {
    if (t?.kind !== "region" || renaming === null) return;
    if (!doc.renameRegion(t.region, renaming.trim())) alert("That name is empty or already used.");
    else doc.target = { kind: "region", region: renaming.trim() };
    renaming = null;
  }
  function del() {
    if (t && confirm(`Delete ${title}?`)) doc.remove(t);
  }
</script>

<aside class="inspector scroll">
  {#if !t || !info?.exists}
    <div class="empty">
      <Icon name="info" size={18} />
      <p><strong>Select something to edit it.</strong></p>
      <p class="small muted">Click an exit, check or event on the page. Its requirements appear here as blocks you can change.</p>
      <p class="small faint">Colours: <RefTag id="ItemBow" small /> item · <span class="key" style:--k="#62c254">region</span> · <span class="key" style:--k="#a58bff">helper</span> · <span class="key" style:--k="#ff8a4f">trick</span></p>
    </div>
  {:else}
    <header>
      <span class="faint small">{subtitle}</span>
      <h2>{title}</h2>
    </header>

    {#if t.kind === "exit" && exit}
      <section>
        <span class="label">Leads to</span>
        {#if changingTarget}
          <Picker options={doc.regionOptions} placeholder="Region…" onpick={retarget} oncancel={() => (changingTarget = false)} />
        {:else}
          <div class="row"><RefTag id={REGION_PREFIX + exit.to} /> <button class="link" onclick={() => (changingTarget = true)}>Change</button></div>
        {/if}
      </section>
    {/if}

    {#if t.kind === "region" && region}
      <section>
        <span class="label">Name</span>
        {#if renaming !== null}
          <div class="row">
            <input class="input" bind:value={renaming} onkeydown={(e) => e.key === "Enter" && rename()} />
            <button class="btn small primary" onclick={rename}>Rename</button>
          </div>
        {:else}
          <div class="row"><strong>{region.name}</strong> <button class="link" onclick={() => (renaming = region.name)}>Rename</button></div>
        {/if}
        {#if doc.file?.start !== region.name}
          <button class="link" onclick={() => doc.edit("Set start region", (f) => (f.start = region.name))}>Make this the starting region</button>
        {/if}
      </section>
    {/if}

    {#if t.kind !== "region" && !(entry && (entry.trick || entry.setting) && !entry.requires && !entry.any_of)}
      <section>
        <span class="label">Requirements</span>
        <p class="small faint">{ruleHelp}</p>
        <RuleBuilder rule={t.kind === "entry" && entry ? entryRule(entry) : info.rule} onchange={(r) => doc.setRule(t as Target, r)} />
      </section>
    {/if}

    {#if t.kind !== "exit"}
      <section>
        <span class="label">Time of day</span>
        <TimeEditor value={time} onchange={setTime} />
        {#if t.kind === "location" || t.kind === "event" || t.kind === "region"}
          <details class="adv">
            <summary class="small">More time rules</summary>
            <label class="check small">
              <input type="checkbox" checked={!!time.no_return} onchange={(e) => flag("no_return", e.currentTarget.checked)} />
              No way back this cycle (what's found here only helps after the Song of Time)
            </label>
            <label class="check small">
              <input type="checkbox" checked={!!time.opening_hours} onchange={(e) => flag("opening_hours", e.currentTarget.checked)} />
              These times are opening hours (must be open when collected)
            </label>
            {#if spot}
              {#each [["before", "Must happen before (same cycle)"], ["temporary_requires", "Also needs, if what's found here doesn't last"]] as const as [key, label] (key)}
                <div class="sub">
                  <span class="small muted">{label}</span>
                  <div class="row wrap">
                    {#each spot[key] ?? [] as id (id)}
                      <RefTag {id} small onremove={() => editSpotList(key, (spot[key] ?? []).filter((x) => x !== id))} />
                    {/each}
                    <button class="link" onclick={() => (addingList = key)}>+ Add</button>
                  </div>
                  {#if addingList === key}
                    <Picker options={doc.refOptions} onpick={(id) => ((addingList = null), editSpotList(key, [...(spot[key] ?? []), id]))} oncancel={() => (addingList = null)} />
                  {/if}
                </div>
              {/each}
            {/if}
          </details>
        {/if}
      </section>
    {/if}

    {#if t.kind === "event" || t.kind === "entry"}
      {@const used = doc.usedBy.get(t.kind === "entry" ? t.id : (spot?.id ?? "")) ?? []}
      <section>
        <span class="label">Used by {used.length}</span>
        {#each used.slice(0, 30) as u, i (i)}
          <button class="use" onclick={() => doc.go(u.page, u.target)}>{u.label}</button>
        {/each}
      </section>
    {/if}

    <section class="danger">
      <button class="btn small danger" onclick={del}><Icon name="x" size={13} /> Delete {t.kind === "exit" ? "exit" : t.kind === "region" ? "region" : t.kind === "entry" ? "helper" : t.kind}</button>
    </section>
  {/if}
</aside>

<style>
  .inspector {
    display: flex;
    flex-direction: column;
    gap: 16px;
    height: 100%;
    min-height: 0;
    padding: 16px;
    border-left: 1px solid var(--border);
    background: color-mix(in srgb, var(--surface) 80%, transparent);
  }
  .empty {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 30px;
    color: var(--text-2);
    text-align: center;
    align-items: center;
  }
  .key {
    color: var(--k);
  }
  h2 {
    font-size: 16px;
    word-break: break-word;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .label {
    font-size: 11.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-3);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .row.wrap {
    flex-wrap: wrap;
  }
  .link {
    padding: 0;
    border: none;
    background: none;
    color: var(--accent);
    font-size: 12.5px;
    text-align: left;
  }
  .adv summary {
    cursor: pointer;
    color: var(--text-2);
  }
  .adv {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .check {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    margin-top: 6px;
  }
  .check input {
    margin-top: 2px;
    accent-color: var(--sec);
  }
  .sub {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 8px;
  }
  .use {
    padding: 3px 6px;
    border: none;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-2);
    font-size: 12.5px;
    text-align: left;
  }
  .use:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .danger {
    margin-top: auto;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }
</style>
