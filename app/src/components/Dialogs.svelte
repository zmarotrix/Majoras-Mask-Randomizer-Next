<script lang="ts">
  // Dialogs opened by shared actions (lib/actions.svelte.ts).
  import Modal from "./Modal.svelte";
  import { dialogs, pasteSettingsString } from "../lib/actions.svelte";
  import { backend, isDesktop } from "../lib/backend";
  import { toasts } from "../lib/state.svelte";

  function fileName(path: string) {
    return path.split(/[\\/]/).pop() ?? path;
  }

  let pasted = $state("");

  async function load() {
    if (await pasteSettingsString(pasted)) {
      dialogs.pasting = false;
      pasted = "";
    }
  }
</script>

{#snippet file(label: string, path: string | null)}
  {#if path}
    <dt>{label}</dt>
    <dd>
      {#if backend.downloadUrl}
        <a class="download" href={backend.downloadUrl(path)} download={fileName(path)}>{fileName(path)}</a>
      {:else}
        {fileName(path)}
      {/if}
    </dd>
  {/if}
{/snippet}

{#if dialogs.pasting}
  <Modal title="Paste settings string" subtitle="Settings strings start with MMRN." onclose={() => (dialogs.pasting = false)}>
    <textarea class="input" rows="4" bind:value={pasted} placeholder="MMRN.…"></textarea>
    {#snippet footer()}
      <button class="btn" onclick={() => (dialogs.pasting = false)}>Cancel</button>
      <button class="btn primary" disabled={!pasted.trim()} onclick={load}>Load settings</button>
    {/snippet}
  </Modal>
{/if}

{#if dialogs.building}
  <Modal title="Building your seed…" subtitle="Placing items and patching the ROM. This takes a few seconds." onclose={() => {}}>
    <div class="progress"><span></span></div>
  </Modal>
{/if}

{#if dialogs.generated}
  {@const g = dialogs.generated}
  <Modal title="Your seed is ready" subtitle={`Seed ${g.seed} · built in ${(g.millis / 1000).toFixed(1)} s`} onclose={() => (dialogs.generated = null)}>
    {#if g.hashIcons.length}
      <div class="hash" title="These icons show on the file select screen: everyone on the same seed sees the same ones">
        {#each g.hashIcons as icon, i (i)}
          <figure><img src={icon.png} alt={icon.name} /><figcaption>{icon.name}</figcaption></figure>
        {/each}
      </div>
    {/if}
    <dl class="result">
      {@render file("ROM", g.romPath)}
      {@render file("Patch", g.patchPath)}
      {@render file("Tracker", g.trackerPath)}
      {@render file("Spoiler log", g.spoilerPath)}
    </dl>
    {#if !isDesktop}
      <p class="note small">Click a file to download it. Nothing leaves your browser: the seed was made right here.</p>
    {/if}
    {#each g.notes as note, i (i)}
      <p class="note small">{note}</p>
    {/each}
    {#snippet footer()}
      {#if g.files.length && isDesktop}
        <button class="btn" onclick={() => backend.revealFile(g.files[0]).catch((e) => toasts.error(e))}>Show in folder</button>
      {:else if g.romPath}
        <button class="btn" onclick={() => backend.revealFile(g.romPath!).catch((e) => toasts.error(e))}>Download ROM</button>
      {/if}
      {#if g.trackerPath}
        <button class="btn" onclick={() => backend.openFile(g.trackerPath!).catch((e) => toasts.error(e))}>Open tracker</button>
      {/if}
      <button class="btn primary" onclick={() => (dialogs.generated = null)}>Done</button>
    {/snippet}
  </Modal>
{/if}

{#if dialogs.patched}
  {@const p = dialogs.patched}
  <Modal title="Patched" subtitle={p.info?.seed ? `Seed ${p.info.seed}` : "Your ROM is ready"} onclose={() => (dialogs.patched = null)}>
    <dl class="result">
      <dt>ROM</dt>
      <dd>{fileName(p.romPath)}</dd>
      {#if p.info?.hashIcons?.length}<dt>Hash</dt><dd>{p.info.hashIcons.join(" · ")}</dd>{/if}
      {#if p.info?.version}<dt>Made by</dt><dd>MMR Next {p.info.version}</dd>{/if}
    </dl>
    {#snippet footer()}
      <button class="btn" onclick={() => backend.revealFile(p.romPath).catch((e) => toasts.error(e))}>{isDesktop ? "Show in folder" : "Download again"}</button>
      <button class="btn primary" onclick={() => (dialogs.patched = null)}>Done</button>
    {/snippet}
  </Modal>
{/if}

{#if dialogs.notice}
  <Modal title={dialogs.notice.title} onclose={() => (dialogs.notice = null)}>
    <p>{dialogs.notice.text}</p>
    {#snippet footer()}
      <button class="btn primary" onclick={() => (dialogs.notice = null)}>OK</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .progress {
    position: relative;
    height: 6px;
    overflow: hidden;
    border-radius: 999px;
    background: var(--surface-2);
  }
  .progress span {
    position: absolute;
    inset: 0 auto 0 0;
    width: 35%;
    border-radius: 999px;
    background: var(--accent);
    animation: slide 1.1s ease-in-out infinite;
  }
  @keyframes slide {
    from {
      left: -35%;
    }
    to {
      left: 100%;
    }
  }
  .hash {
    display: flex;
    gap: 14px;
    justify-content: center;
    margin: 4px 0 18px;
  }
  .hash figure {
    margin: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    width: 72px;
  }
  .hash img {
    width: 48px;
    height: 48px;
    image-rendering: pixelated;
    filter: drop-shadow(0 2px 6px rgb(0 0 0 / 0.35));
  }
  .hash figcaption {
    font-size: 11px;
    text-align: center;
    color: var(--text-muted, inherit);
    line-height: 1.2;
  }
  .result {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 6px 16px;
    margin: 0 0 8px;
  }
  .result dt {
    color: var(--text-3);
  }
  .result dd {
    margin: 0;
    word-break: break-all;
  }
  .note {
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    background: var(--warn-soft);
    color: var(--warn);
  }
  textarea {
    font-family: var(--mono);
    font-size: 12px;
  }
  .download {
    color: var(--accent);
    text-decoration: none;
    font-weight: 600;
  }
  .download:hover {
    text-decoration: underline;
  }
</style>
