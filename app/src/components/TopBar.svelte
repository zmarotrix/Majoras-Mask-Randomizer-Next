<script lang="ts">
  // Always-visible bar: everything about the seed you're about to make.
  import Icon from "./Icon.svelte";
  import * as actions from "../lib/actions.svelte";
  import { app } from "../lib/state.svelte";
  import { summarize } from "../lib/summary";

  let menu = $state<"preset" | null>(null);

  const changedCount = $derived(app.schema ? app.schema.settings.filter((s) => app.isChanged(s.key)).length : 0);
  const presetOf = (id: string | null) => app.presets.find((p) => p.id === id)?.name;
  const presetName = $derived(
    presetOf(app.presetId) ?? presetOf(app.basePresetId) ?? (changedCount === 0 ? "Default settings" : "Custom settings"),
  );
  const edited = $derived(!app.presetId && !!presetOf(app.basePresetId));

  const THEMES = { system: ["monitor", "Theme: match the system"], light: ["sun", "Theme: light"], dark: ["moon", "Theme: dark"] } as const;
  const NEXT = { system: "light", light: "dark", dark: "system" } as const;

  /** Close the menu, then run `f`. */
  const then = (f: () => unknown) => () => {
    menu = null;
    f();
  };
</script>

<svelte:window onclick={(e) => menu && !(e.target as HTMLElement).closest(".menu-wrap") && (menu = null)} />

<header class="top">
  <button class="icon-btn menu-btn" aria-label="Menu" onclick={() => (app.navOpen = !app.navOpen)}><Icon name="list" size={18} /></button>
  <div class="brand">
    <img src="/logo.png" alt="" width="28" height="28" />
    <span class="brand-text">Majora's Mask Randomizer Next</span>
  </div>

  <button
    class="pill rom"
    class:ok={app.rom?.check.ok}
    class:bad={app.rom && !app.rom.check.ok}
    onclick={actions.chooseRom}
    title={app.rom ? `${app.rom.path}\n${app.rom.check.message}` : "Choose or drop your ROM"}
  >
    <Icon name={app.rom?.check.ok ? "check" : app.rom ? "alert" : "cartridge"} size={15} />
    {#if app.rom?.check.ok}
      Majora's Mask (USA)
    {:else if app.rom}
      ROM problem
    {:else}
      Choose ROM
    {/if}
  </button>

  <div class="menu-wrap">
    <button class="pill" onclick={() => (menu = menu ? null : "preset")} aria-expanded={menu === "preset"}>
      <Icon name="sliders" size={15} />
      {presetName}
      {#if edited}<span class="faint">· edited</span>{/if}
      <Icon name="chevron-down" size={13} />
    </button>
    {#if menu === "preset"}
      <div class="menu card">
        <div class="menu-title small faint">Presets</div>
        {#each app.presets as p (p.id)}
          <button class="preset" class:sel={app.presetId === p.id} onclick={() => ((menu = null), app.applyPreset(p))}>
            <span class="pname">{p.name}</span>
            {#if p.description}<span class="small">{p.description}</span>{/if}
            <span class="small muted">{summarize({ ...app.schema!.defaults, ...p.settings }, app.schema!).join(" · ")}</span>
          </button>
        {/each}
        <hr />
        <button onclick={then(actions.copySettingsString)}><Icon name="copy" size={14} /> Copy settings string</button>
        <button onclick={then(() => (actions.dialogs.pasting = true))}><Icon name="download" size={14} /> Paste settings string…</button>
        <button onclick={then(actions.openSettingsFile)}><Icon name="file" size={14} /> Open settings file…</button>
        <button onclick={then(actions.saveSettingsFile)}><Icon name="upload" size={14} /> Save settings file…</button>
        <hr />
        <div class="menu-title small faint">When generating, also write</div>
        <button onclick={() => app.setOutput("tracker", !app.outputs.tracker)}><Icon name="file" size={14} /> Tracker (spoiler-free): {app.outputs.tracker ? "yes" : "no"}</button>
        <button onclick={() => app.setOutput("spoiler", !app.outputs.spoiler)}><Icon name="file" size={14} /> Spoiler log: {app.outputs.spoiler ? "yes" : "no"}</button>
        <button onclick={() => app.setOutput("patch", !app.outputs.patch)}><Icon name="file" size={14} /> Patch file (.mmrn): {app.outputs.patch ? "yes" : "no"}</button>
        <button onclick={then(actions.applyPatch)}><Icon name="download" size={14} /> Apply a patch (.mmrn)…</button>
        <hr />
        <button onclick={then(() => app.setLayout("classic"))}><Icon name="columns" size={14} /> Classic layout (like the old randomizer)</button>
        <button onclick={then(() => app.setMotion(!app.motion))}>
          <Icon name="sparkles" size={14} /> Moving background: {app.motion ? "on" : "off"}
        </button>
        <hr />
        <button onclick={then(actions.resetSettings)}><Icon name="reset" size={14} /> Reset to defaults</button>
      </div>
    {/if}
  </div>

  <span class="spacer"></span>

  <button class="icon-btn theme" title="{THEMES[app.theme][1]} (click to change)" aria-label={THEMES[app.theme][1]} onclick={() => app.setTheme(NEXT[app.theme])}>
    <Icon name={THEMES[app.theme][0]} size={17} />
  </button>

  <div class="seed">
    <input class="input" placeholder="Random seed" bind:value={app.seed} aria-label="Seed" />
    <button class="icon-btn" title="Roll a random seed" onclick={actions.rollSeed}>
      <Icon name="dice" size={16} />
    </button>
  </div>
  <button class="btn primary" onclick={actions.generate} title={app.rom?.check.ok ? "Generate a seed" : "Choose a ROM first"}>
    <Icon name="play" size={14} /> Generate
  </button>
</header>

<style>
  .top {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    background: color-mix(in srgb, var(--surface) 75%, transparent);
    backdrop-filter: blur(12px);
    border-bottom: 1px solid var(--border);
    box-shadow: 0 6px 24px rgba(3, 0, 12, 0.25);
    position: relative;
    z-index: 5;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-right: 10px;
    font-weight: 700;
    white-space: nowrap;
  }
  .menu-btn {
    display: none;
  }
  @media (max-width: 760px) {
    .top {
      flex-wrap: wrap;
    }
    .menu-btn {
      display: inline-grid;
    }
    .brand-text {
      display: none;
    }
    .seed {
      width: 120px;
    }
  }
  @media (max-width: 1000px) {
    .brand-text {
      display: none;
    }
  }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 6px 12px;
    border-radius: 999px;
    border: 1px solid var(--border-strong);
    background: var(--surface);
    font-weight: 500;
    white-space: nowrap;
  }
  .pill:hover {
    background: var(--surface-2);
  }
  .rom.ok {
    border-color: var(--good);
    color: var(--good);
    background: var(--good-soft);
  }
  .rom.bad {
    border-color: var(--bad);
    color: var(--bad);
    background: var(--bad-soft);
  }
  .menu-wrap {
    position: relative;
  }
  .menu {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    z-index: 40;
    width: 400px;
    max-height: 70vh;
    overflow: auto;
    padding: 6px;
    box-shadow: var(--shadow-lg);
  }
  .menu-title {
    padding: 4px 8px;
  }
  .menu button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 7px 10px;
    border: none;
    border-radius: 6px;
    background: none;
    text-align: left;
  }
  .menu button:hover {
    background: var(--surface-2);
  }
  .menu .preset {
    flex-direction: column;
    align-items: flex-start;
    gap: 1px;
  }
  .menu .preset.sel {
    background: var(--accent-soft);
  }
  .pname {
    font-weight: 600;
  }
  .menu hr {
    border: none;
    border-top: 1px solid var(--border);
    margin: 6px 0;
  }
  .seed {
    position: relative;
    width: 170px;
  }
  .seed .input {
    padding-right: 34px;
  }
  .seed .icon-btn {
    position: absolute;
    right: 4px;
    top: 50%;
    transform: translateY(-50%);
  }
</style>
