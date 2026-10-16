<script lang="ts">
  // The classic layout: MMR 1.16's window (menu bar, settings tabs, ROM /
  // outputs / seed panel, status line) in this app's style, for players
  // coming from the old randomizer.
  import ClassicGroup from "../components/classic/ClassicGroup.svelte";
  import ClassicItemPool from "../components/classic/ClassicItemPool.svelte";
  import ClassicTools from "../components/classic/ClassicTools.svelte";
  import { openTool } from "../components/classic/tools.svelte";
  import LogicEditor from "./LogicEditor.svelte";
  import Icon from "../components/Icon.svelte";
  import * as actions from "../lib/actions.svelte";
  import { app } from "../lib/state.svelte";
  import { CLASSIC_TABS } from "../lib/classic";

  const tabId = $derived(CLASSIC_TABS.some((t) => t.id === app.classicTab) ? app.classicTab : "main");
  const tab = $derived(CLASSIC_TABS.find((t) => t.id === tabId)!);
  const choose = (id: string) => app.setClassicTab(id);

  let menu = $state<string | null>(null);
  let outputTab = $state<"output" | "patch">("output");

  const status = $derived.by(() => {
    if (!app.help) return "Ready...";
    const def = app.names.settings.get(app.help);
    return def?.description ?? "Ready...";
  });

  const run = (f: () => unknown) => () => {
    menu = null;
    f();
  };

  const MENUS: { title: string; items: ({ label: string; act: () => unknown; disabled?: boolean } | "-")[] }[] = [
    {
      title: "File",
      items: [
        { label: "Open settings…", act: actions.openSettingsFile },
        { label: "Save settings…", act: actions.saveSettingsFile },
        "-",
        { label: "Copy settings string", act: actions.copySettingsString },
        { label: "Paste settings string…", act: () => (actions.dialogs.pasting = true) },
        "-",
        { label: "Apply a patch (.mmrn)…", act: actions.applyPatch },
        "-",
        { label: "Reset to defaults", act: actions.resetSettings },
      ],
    },
    {
      title: "Customize",
      items: [
        { label: "HUD colors…", act: () => openTool("hud") },
        { label: "D-Pad…", act: () => openTool("dpad") },
        "-",
        { label: "Random starting items…", act: () => openTool("random-starting") },
        { label: "Hint priorities…", act: () => openTool("hints") },
      ],
    },
    {
      title: "Presets",
      items: [],
    },
    {
      title: "Tools",
      items: [
        { label: "Logic Editor", act: actions.openLogicEditor },
        { label: "Seed tracker…", act: actions.openTracker },
      ],
    },
    {
      title: "View",
      items: [
        { label: "Switch to the guided layout", act: () => app.setLayout("guided") },
        "-",
        { label: "Theme: match the system", act: () => app.setTheme("system") },
        { label: "Theme: light", act: () => app.setTheme("light") },
        { label: "Theme: dark", act: () => app.setTheme("dark") },
        "-",
        { label: "Moving background on / off", act: () => app.setMotion(!app.motion) },
      ],
    },
    {
      title: "Help",
      items: [{ label: "Manual (every setting and trick)", act: actions.openManual }],
    },
  ];
</script>

<svelte:window onclick={(e) => menu && !(e.target as HTMLElement).closest(".menubar") && (menu = null)} />

<div class="classic">
  <nav class="menubar" aria-label="Menu">
    {#each MENUS as m (m.title)}
      <div class="menu-wrap">
        <button class="top" class:open={menu === m.title} aria-expanded={menu === m.title} onclick={() => (menu = menu === m.title ? null : m.title)}>
          {m.title}
        </button>
        {#if menu === m.title}
          <div class="dropdown card" role="menu">
            {#if m.title === "Presets"}
              {#each app.presets as p (p.id)}
                <button role="menuitem" class:sel={app.presetId === p.id} title={p.description} onclick={run(() => app.applyPreset(p))}>{p.name}</button>
              {/each}
            {:else}
              {#each m.items as it, i (i)}
                {#if it === "-"}<hr />{:else}<button role="menuitem" disabled={it.disabled} onclick={run(it.act)}>{it.label}</button>{/if}
              {/each}
            {/if}
          </div>
        {/if}
      </div>
    {/each}
    <span class="spacer"></span>
    <button class="layout-btn small" onclick={() => app.setLayout("guided")} title="The newer layout, with presets, step-by-step setup and explanations">
      <Icon name="sparkles" size={13} /> Guided layout
    </button>
  </nav>

  {#if app.screen === "logic-editor"}
    <div class="back">
      <button class="btn small" onclick={() => app.go("quick")}><Icon name="arrow-left" size={13} /> Back to settings</button>
    </div>
    <div class="tool"><LogicEditor /></div>
  {:else}
    <div class="settings">
      <div class="tabs" role="tablist">
        {#each CLASSIC_TABS as t (t.id)}
          <button role="tab" aria-selected={tabId === t.id} class:on={tabId === t.id} onclick={() => choose(t.id)}>{t.title}</button>
        {/each}
      </div>
      <div class="tab-page scroll" style:--areas={tab.areas.map((a) => `"${a}"`).join(" ")} style:--columns={tab.columns}>
        {#if tab.id === "items"}
          <ClassicItemPool />
        {:else}
          {#each tab.groups as g (g.area)}<ClassicGroup group={g} />{/each}
        {/if}
      </div>
    </div>

    <div class="bottom">
      <p class="rom-note small muted">ROM must be Majora's Mask (U). .z64, .n64 and .v64 files all work.</p>
      <div class="rom-row">
        <button class="btn" onclick={actions.chooseRom}>Open ROM</button>
        <input
          class="input path"
          class:bad={app.rom && !app.rom.check.ok}
          readonly
          value={app.rom?.path ?? ""}
          placeholder="No ROM chosen (you can also drop it on the window)"
          title={app.rom?.check.message}
          aria-label="ROM file"
        />
      </div>
      <div class="out-row">
        <fieldset class="outputs">
          <legend>Outputs</legend>
          <label><input type="checkbox" checked={app.outputs.rom} onchange={(e) => app.setOutput("rom", e.currentTarget.checked)} /> N64 ROM .z64</label>
          <label><input type="checkbox" checked={app.outputs.spoiler} onchange={(e) => app.setOutput("spoiler", e.currentTarget.checked)} /> Spoiler log</label>
          <label><input type="checkbox" checked={app.outputs.tracker} onchange={(e) => app.setOutput("tracker", e.currentTarget.checked)} /> Tracker .html</label>
          <label
            ><input type="checkbox" checked={app.values.DrawHash === true} onchange={(e) => app.setValue("DrawHash", e.currentTarget.checked)} /> Hash Icons .png</label
          >
          <label><input type="checkbox" checked={app.outputs.patch} onchange={(e) => app.setOutput("patch", e.currentTarget.checked)} /> Patch .mmrn</label>
        </fieldset>
        <div class="gen">
          <div class="strip" role="tablist">
            <button role="tab" aria-selected={outputTab === "output"} class:on={outputTab === "output"} onclick={() => (outputTab = "output")}>Output settings</button>
            <button role="tab" aria-selected={outputTab === "patch"} class:on={outputTab === "patch"} onclick={() => (outputTab = "patch")}>Patch settings</button>
          </div>
          <div class="gen-page">
            {#if outputTab === "output"}
              <label class="seed">
                Random seed:
                <input class="input" bind:value={app.seed} placeholder="Random" aria-label="Seed" />
              </label>
              <button class="btn small" onclick={actions.rollSeed}>Re-Roll Seed</button>
              <button class="btn primary randomize" onclick={actions.generate}>Randomize</button>
            {:else}
              <span class="small muted grow">Got a patch (.mmrn) from someone? Turn it into a ROM with your own clean ROM.</span>
              <button class="btn primary randomize" onclick={actions.applyPatch}>Apply Patch…</button>
            {/if}
          </div>
        </div>
      </div>
    </div>
  {/if}

  <footer class="status small" aria-live="polite">{status}</footer>
</div>

<ClassicTools />

<style>
  .classic {
    display: grid;
    grid-template-rows: auto 1fr auto auto;
    grid-template-columns: minmax(0, 1fr);
    height: 100%;
    min-height: 0;
  }
  .menubar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px;
    padding: 3px 8px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }
  .menu-wrap {
    position: relative;
  }
  .top {
    padding: 4px 10px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
  }
  .top:hover,
  .top.open {
    background: var(--surface-3);
  }
  .dropdown {
    position: absolute;
    top: calc(100% + 2px);
    left: 0;
    z-index: 50;
    display: flex;
    flex-direction: column;
    min-width: 220px;
    max-height: 70vh;
    overflow: auto;
    padding: 4px;
    box-shadow: var(--shadow-lg);
  }
  .dropdown button {
    padding: 6px 10px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    text-align: left;
  }
  .dropdown button:hover:not(:disabled) {
    background: var(--accent-soft);
  }
  .dropdown button.sel {
    font-weight: 600;
    color: var(--accent);
  }
  .dropdown hr {
    width: 100%;
    margin: 4px 0;
    border: none;
    border-top: 1px solid var(--border);
  }
  .spacer {
    flex: 1;
  }
  .layout-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    color: var(--text-2);
  }
  .layout-btn:hover {
    color: var(--accent);
    border-color: var(--accent);
  }

  .settings {
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 8px 10px 0;
  }
  .tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
  }
  .tabs button {
    padding: 6px 12px;
    border: 1px solid var(--border);
    border-bottom: none;
    border-radius: var(--radius-sm) var(--radius-sm) 0 0;
    background: var(--surface-2);
    color: var(--text-2);
  }
  .tabs button:hover {
    color: var(--text);
  }
  .tabs button.on {
    position: relative;
    z-index: 1;
    margin-bottom: -1px;
    padding-bottom: 7px;
    background: var(--bg);
    color: var(--text);
    font-weight: 600;
  }
  .tab-page {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-areas: var(--areas);
    grid-template-columns: var(--columns);
    align-content: start;
    gap: 10px;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: 0 var(--radius-sm) 0 0;
    background: var(--bg);
  }
  @media (max-width: 760px) {
    .tab-page {
      display: flex;
      flex-direction: column;
    }
  }

  .bottom {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 6px 10px 8px;
    border-top: 1px solid var(--border);
    background: var(--surface-2);
  }
  .rom-note {
    margin: 0;
    text-align: center;
  }
  .rom-row {
    display: flex;
    gap: 8px;
  }
  .path {
    flex: 1;
    min-width: 0;
  }
  .path.bad {
    border-color: var(--bad);
  }
  .out-row {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 10px;
    align-items: stretch;
  }
  @media (max-width: 760px) {
    .out-row {
      grid-template-columns: 1fr;
    }
    /* Phones: the settings get the room; the ROM hint is in the box. */
    .rom-note {
      display: none;
    }
  }
  .outputs {
    display: grid;
    grid-template-columns: auto auto;
    gap: 2px 16px;
    margin: 0;
    padding: 4px 12px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
  }
  .outputs legend {
    padding: 0 6px;
    font-size: 13px;
    font-weight: 600;
  }
  .outputs label {
    display: flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
  }
  .outputs input {
    accent-color: var(--accent);
  }
  .gen {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .strip {
    display: flex;
    gap: 2px;
  }
  .strip button {
    padding: 4px 10px;
    border: 1px solid var(--border);
    border-bottom: none;
    border-radius: var(--radius-sm) var(--radius-sm) 0 0;
    background: var(--surface-2);
    color: var(--text-2);
    font-size: 13px;
  }
  .strip button.on {
    position: relative;
    z-index: 1;
    margin-bottom: -1px;
    padding-bottom: 5px;
    background: var(--surface);
    color: var(--text);
    font-weight: 600;
  }
  .gen-page {
    flex: 1;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 0 var(--radius-sm) var(--radius-sm) var(--radius-sm);
    background: var(--surface);
  }
  .seed {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 220px;
    white-space: nowrap;
  }
  .seed .input,
  .grow {
    flex: 1;
    min-width: 0;
  }
  .randomize {
    min-width: 120px;
    min-height: 44px;
    font-size: 15px;
  }
  .status {
    padding: 4px 12px;
    border-top: 1px solid var(--border);
    background: var(--surface);
    color: var(--text-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .back {
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }
  .tool {
    min-height: 0;
    overflow: hidden;
  }
  .classic:has(.tool) {
    grid-template-rows: auto auto 1fr auto;
  }
  @media (max-width: 760px) {
    .seed {
      flex-basis: 100%;
      min-width: 0;
    }
    .randomize {
      flex: 1;
    }
  }
</style>
