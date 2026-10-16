<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import TopBar from "./components/TopBar.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import SectionPage from "./screens/SectionPage.svelte";
  import QuickSetup from "./screens/QuickSetup.svelte";
  import ItemPool from "./screens/ItemPool.svelte";
  import LogicEditor from "./screens/LogicEditor.svelte";
  import Dialogs from "./components/Dialogs.svelte";
  import Classic from "./screens/Classic.svelte";
  import LayoutChoice from "./components/LayoutChoice.svelte";
  import LairBackground from "./components/LairBackground.svelte";
  import { app, isEditorWindow, toasts } from "./lib/state.svelte";
  import { doc } from "./lib/regionDoc.svelte";
  import { isDesktop } from "./lib/backend";
  import { checkRom } from "./lib/actions.svelte";

  let dragging = $state(false);

  onMount(() => {
    app.init();
    if (!isDesktop) return;
    // Drop a ROM anywhere on the window.
    const unlisten = getCurrentWebview().onDragDropEvent((e) => {
      const p = e.payload;
      if (p.type === "enter" || p.type === "over") dragging = true;
      else if (p.type === "leave") dragging = false;
      else if (p.type === "drop") {
        dragging = false;
        if (p.paths[0]) checkRom(p.paths[0]);
      }
    });
    return () => void unlisten.then((f) => f());
  });

  function onKey(e: KeyboardEvent) {
    if (!(e.ctrlKey || e.metaKey) || (app.screen !== "logic-editor" && !isEditorWindow)) return;
    if ((e.target as HTMLElement).closest("input, textarea")) return;
    if (e.key === "z" && !e.shiftKey) {
      doc.undo();
      e.preventDefault();
    } else if (e.key === "y" || (e.key === "z" && e.shiftKey)) {
      doc.redo();
      e.preventDefault();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<LairBackground />

{#if app.loadError}
  <div class="center">
    <h2>Couldn't start</h2>
    <p class="muted">{app.loadError}</p>
  </div>
{:else if !app.ready}
  <div class="center muted">Loading…</div>
{:else if isEditorWindow}
  <div class="editor-window"><LogicEditor /></div>
{:else if app.layout === "classic"}
  <Classic />
{:else}
  <div class="shell">
    <TopBar />
    <div class="body">
      <Sidebar />
      <main>
        {#if app.screen === "logic-editor"}
          <LogicEditor />
        {:else if app.screen === "items"}
          <ItemPool />
        {:else if app.screen === "quick"}
          <QuickSetup />
        {:else}
          <SectionPage />
        {/if}
      </main>
    </div>
  </div>
{/if}

<Dialogs />

{#if app.ready && app.chooseLayout && !isEditorWindow}
  <LayoutChoice />
{/if}

{#if dragging}
  <div class="drop">Drop your Majora's Mask ROM</div>
{/if}

<div class="toasts" aria-live="polite">
  {#each toasts.list as t (t.id)}
    <div class="toast" class:error={t.kind === "error"}>{t.text}</div>
  {/each}
</div>

<style>
  .editor-window {
    height: 100%;
    overflow: hidden;
  }
  .shell {
    display: grid;
    grid-template-rows: auto 1fr;
    height: 100%;
  }
  .body {
    display: grid;
    grid-template-columns: 220px 1fr;
    min-height: 0;
  }
  @media (max-width: 760px) {
    .body {
      grid-template-columns: 1fr;
    }
  }
  main {
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }
  .center {
    display: grid;
    place-content: center;
    gap: 8px;
    height: 100%;
    text-align: center;
  }
  .drop {
    position: fixed;
    inset: 12px;
    z-index: 90;
    display: grid;
    place-items: center;
    border: 3px dashed var(--accent);
    border-radius: 16px;
    background: color-mix(in srgb, var(--accent-soft) 85%, transparent);
    color: var(--accent);
    font-size: 20px;
    font-weight: 600;
    pointer-events: none;
  }
  .toasts {
    position: fixed;
    right: 18px;
    bottom: 18px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 100;
    max-width: 420px;
  }
  .toast {
    padding: 10px 14px;
    border-radius: var(--radius-sm);
    background: var(--text);
    color: var(--bg);
    box-shadow: var(--shadow-lg);
    animation: in 0.15s ease-out;
  }
  .toast.error {
    background: var(--bad);
    color: #fff;
  }
  @keyframes in {
    from {
      transform: translateY(8px);
      opacity: 0;
    }
  }
</style>
