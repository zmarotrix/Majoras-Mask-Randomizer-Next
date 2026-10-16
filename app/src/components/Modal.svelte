<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    title: string;
    subtitle?: string;
    wide?: boolean;
    onclose: () => void;
    children: Snippet;
    footer?: Snippet;
  }
  let { title, subtitle, wide = false, onclose, children, footer }: Props = $props();

  let dialog: HTMLDialogElement;
  $effect(() => {
    dialog.showModal();
  });
</script>

<dialog
  bind:this={dialog}
  class:wide
  onclose={onclose}
  onclick={(e) => {
    if (e.target === dialog) dialog.close();
  }}
>
  <div class="frame">
    <header>
      <div>
        <h2>{title}</h2>
        {#if subtitle}<p class="muted small">{subtitle}</p>{/if}
      </div>
      <button class="icon-btn" aria-label="Close" onclick={() => dialog.close()}>✕</button>
    </header>
    <div class="body">
      {@render children()}
    </div>
    {#if footer}
      <footer>{@render footer()}</footer>
    {/if}
  </div>
</dialog>

<style>
  dialog {
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 14px;
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-lg);
    width: min(620px, calc(100vw - 48px));
    max-height: calc(100vh - 60px);
  }
  dialog.wide {
    width: min(980px, calc(100vw - 48px));
  }
  dialog::backdrop {
    background: rgba(10, 6, 24, 0.45);
  }
  .frame {
    display: flex;
    flex-direction: column;
    max-height: calc(100vh - 62px);
  }
  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding: 16px 18px 12px;
    border-bottom: 1px solid var(--border);
  }
  .body {
    padding: 14px 18px;
    overflow: auto;
    min-height: 0;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 18px;
    border-top: 1px solid var(--border);
  }
</style>
