<script lang="ts">
  // The logic editor: navigator | region / helper page | inspector.
  // Logic is edited as regions (places) connected by exits; each check
  // says what it needs once you're there.
  import Navigator from "../components/regions/Navigator.svelte";
  import RegionPage from "../components/regions/RegionPage.svelte";
  import EntryPage from "../components/regions/EntryPage.svelte";
  import Inspector from "../components/regions/Inspector.svelte";
  import Guide from "../components/regions/Guide.svelte";
  import Icon from "../components/Icon.svelte";
  import { backend } from "../lib/backend";
  import { doc } from "../lib/regionDoc.svelte";
  import { sectionColor } from "../lib/sections";

  let menu = $state<"open" | "save" | "problems" | null>(null);
  let hideConversion = $state(false);
  let guideOpen = $state(false);

  const name = $derived(
    doc.source?.kind === "bundled" ? doc.source.name : doc.source?.kind === "file" ? doc.source.path.split(/[\\/]/).pop() : "No logic open",
  );
  const errors = $derived(doc.issues.filter((i) => i.severity === "error"));
  const warnings = $derived(doc.issues.filter((i) => i.severity === "warning"));

  const FILTERS = [{ name: "Logic", extensions: ["json", "txt"] }];
  async function openFile() {
    menu = null;
    const path = await backend.pickFile("Open logic (any MMR format)", FILTERS);
    if (path) doc.open(path);
  }
  function openBundled(id: string) {
    menu = null;
    doc.open(`bundled:${id}`);
  }
  async function saveAs(oldFormat = false) {
    menu = null;
    const path = await backend.pickSavePath(oldFormat ? "Export for older MMR versions" : "Save logic", [{ name: "Logic", extensions: ["json"] }], oldFormat ? "logic-old-format.json" : "logic.json");
    if (path) doc.save(path, oldFormat);
  }
  let guideStart = $state<"beginner" | "veteran" | undefined>();
  function openGuide(id: "beginner" | "veteran") {
    guideStart = id;
    guideOpen = true;
  }
  function save() {
    if (doc.source?.kind === "file" && doc.conversion === null) doc.save(doc.source.path);
    else saveAs();
  }
</script>

<svelte:window onclick={(e) => menu && !(e.target as HTMLElement).closest(".menu-wrap") && (menu = null)} />

<div class="editor" style:--sec={sectionColor("logic-editor")}>
  <header class="toolbar">
    <span class="tile"><Icon name="branch" size={16} /></span>
    <div class="titles">
      <strong>{name}{#if doc.dirty}<span class="dot" title="Unsaved changes"></span>{/if}</strong>
      {#if doc.file}<span class="faint small">{doc.file.regions.length} regions · {doc.file.entries.length} helpers, tricks and settings</span>{/if}
    </div>
    <span class="spacer"></span>

    <div class="menu-wrap">
      <button class="btn small" onclick={() => (menu = menu === "open" ? null : "open")}><Icon name="file" size={13} /> Open <Icon name="chevron-down" size={12} /></button>
      {#if menu === "open"}
        <div class="menu card">
          <button onclick={() => openBundled("casual")}>Casual logic (built in)</button>
          <button onclick={() => openBundled("glitched")}>Glitched logic (built in)</button>
          <hr />
          <button onclick={openFile}>Open a file…<span class="faint small">Any MMR logic file. Older ones are converted to regions and checked.</span></button>
        </div>
      {/if}
    </div>
    <button class="btn small" disabled={!doc.undoLabel} title={doc.undoLabel ? `Undo: ${doc.undoLabel} (Ctrl+Z)` : "Nothing to undo"} onclick={() => doc.undo()}>Undo</button>
    <button class="btn small" disabled={!doc.redoLabel} title={doc.redoLabel ? `Redo: ${doc.redoLabel} (Ctrl+Y)` : "Nothing to redo"} onclick={() => doc.redo()}>Redo</button>

    <div class="menu-wrap">
      <button class="btn small" class:bad={errors.length} disabled={!doc.file} onclick={() => (menu = menu === "problems" ? null : "problems")}>
        <Icon name={errors.length || warnings.length ? "alert" : "check"} size={13} />
        {errors.length ? `${errors.length} problems` : warnings.length ? `${warnings.length} warnings` : "No problems"}
      </button>
      {#if menu === "problems"}
        <div class="menu card problems">
          {#each [...errors, ...warnings].slice(0, 100) as p, i (i)}
            <button onclick={() => ((menu = null), p.entry && doc.follow(p.entry))}>
              <span class:err={p.severity === "error"}>{p.message}</span>
              {#if p.entry}<span class="faint small">{p.entry}</span>{/if}
            </button>
          {:else}
            <p class="small muted pad">Everything checks out.</p>
          {/each}
        </div>
      {/if}
    </div>

    <button class="btn small" class:on={guideOpen} onclick={() => ((guideStart = undefined), (guideOpen = !guideOpen))}><Icon name="help" size={13} /> Guide</button>

    <div class="menu-wrap">
      <button class="btn small primary" disabled={!doc.file} onclick={save}><Icon name="upload" size={13} /> Save</button>
      <button class="btn small" disabled={!doc.file} aria-label="More save options" onclick={() => (menu = menu === "save" ? null : "save")}><Icon name="chevron-down" size={12} /></button>
      {#if menu === "save"}
        <div class="menu card right">
          <button onclick={() => saveAs()}>Save as…</button>
          <button onclick={() => saveAs(true)}>Export for older MMR versions…<span class="faint small">The old single-list format. Regions are flattened.</span></button>
        </div>
      {/if}
    </div>
  </header>

  {#if doc.conversion && !hideConversion}
    <div class="banner" class:bad={!doc.conversion.equivalent}>
      <Icon name={doc.conversion.equivalent ? "check" : "alert"} size={15} />
      {#if doc.conversion.equivalent}
        Converted from the {doc.conversion.format === "old" ? "older MMR" : "single-list"} format into {doc.conversion.regions} regions. It behaves exactly like the original
        (checked {doc.conversion.entries} rules in {doc.conversion.scenarios} test situations). Save to keep it in the new format.
      {:else}
        Converted into {doc.conversion.regions} regions, but {doc.conversion.mismatched.length} rules behave differently from the original, e.g.
        {doc.conversion.mismatched.slice(0, 3).join(", ")}. Please check them before saving.
      {/if}
      <button class="icon-btn" aria-label="Dismiss" onclick={() => (hideConversion = true)}><Icon name="x" size={14} /></button>
    </div>
  {/if}

  {#if guideOpen}
    <Guide initial={guideStart} onclose={() => (guideOpen = false)} />
  {/if}

  {#if !doc.file}
    <div class="start">
      <div class="card start-card">
        <span class="tile big"><Icon name="branch" size={22} /></span>
        <h1>Logic editor</h1>
        <p class="muted">
          Logic says what you need to reach each check. Here it's organised as <strong>regions</strong>: places connected by exits. Each exit says what you need
          to go that way, and each check says what you need once you're there.
        </p>
        <div class="choices">
          <button class="btn primary" disabled={doc.loading} onclick={() => openBundled("casual")}>Open Casual logic</button>
          <button class="btn" disabled={doc.loading} onclick={() => openBundled("glitched")}>Open Glitched logic</button>
          <button class="btn" disabled={doc.loading} onclick={openFile}>Open or import a file…</button>
        </div>
        <p class="small faint">Logic files from older MMR versions are converted to regions automatically, and checked to behave exactly the same.</p>
        <div class="guides">
          <span class="small muted">New to this?</span>
          <button class="link" onclick={() => openGuide("beginner")}>Read the beginner's guide</button>
          <span class="faint">·</span>
          <button class="link" onclick={() => openGuide("veteran")}>Used the old logic editor? What's changed</button>
        </div>
        {#if doc.loading}<p class="small muted">Opening…</p>{/if}
      </div>
    </div>
  {:else}
    <div class="cols">
      <Navigator />
      <main class="scroll">
        {#if doc.page?.kind === "region"}
          <RegionPage name={doc.page.name} />
        {:else if doc.page?.kind === "entry"}
          <EntryPage id={doc.page.id} />
        {/if}
      </main>
      <Inspector />
    </div>
  {/if}
</div>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .toolbar {
    position: relative;
    z-index: 5;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    border-bottom: 1px solid var(--border);
    background: color-mix(in srgb, var(--surface) 78%, transparent);
    backdrop-filter: blur(12px);
  }
  .titles {
    display: flex;
    flex-direction: column;
    line-height: 1.25;
  }
  .titles .dot {
    margin-left: 6px;
    vertical-align: middle;
  }
  .menu-wrap {
    position: relative;
    display: flex;
    gap: 2px;
  }
  .menu {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    z-index: 30;
    display: flex;
    flex-direction: column;
    min-width: 280px;
    max-height: 60vh;
    overflow: auto;
    padding: 4px;
    box-shadow: var(--shadow-lg);
  }
  .menu.right {
    left: auto;
    right: 0;
  }
  .menu.problems {
    min-width: 420px;
  }
  .menu button {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 7px 10px;
    border: none;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text);
    text-align: left;
  }
  .menu button:hover {
    background: var(--surface-2);
  }
  .menu hr {
    width: 100%;
    margin: 4px 0;
    border: none;
    border-top: 1px solid var(--border);
  }
  .err {
    color: var(--bad);
  }
  .pad {
    padding: 8px 10px;
  }
  .btn.bad {
    color: var(--bad);
    border-color: color-mix(in srgb, var(--bad) 50%, transparent);
  }
  .banner {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 16px;
    background: var(--good-soft);
    color: var(--good);
    font-size: 13px;
  }
  .banner.bad {
    background: var(--warn-soft);
    color: var(--warn);
  }
  .banner .icon-btn {
    margin-left: auto;
  }
  .editor {
    position: relative;
  }
  .btn.on {
    border-color: var(--sec);
    color: var(--text);
  }
  .guides {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    align-items: center;
    gap: 6px;
    margin-top: 4px;
  }
  .link {
    padding: 0;
    border: none;
    background: none;
    color: var(--accent);
    font-size: 13px;
  }
  .link:hover {
    text-decoration: underline;
  }
  .cols {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 270px minmax(0, 1fr) 360px;
  }
  main {
    min-height: 0;
  }
  .start {
    flex: 1;
    display: grid;
    place-items: center;
    padding: 24px;
  }
  .start-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    max-width: 560px;
    padding: 32px;
    text-align: center;
  }
  .choices {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 8px;
    margin-top: 6px;
  }
</style>
