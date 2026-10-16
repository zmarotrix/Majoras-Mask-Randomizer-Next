<script lang="ts">
  // The built-in guides: one for people new to logic, one for people who
  // used the old logic editor. Content lives in lib/guides.ts.
  import { untrack } from "svelte";
  import Icon from "../Icon.svelte";
  import { KIND_COLORS } from "./RefTag.svelte";
  import { GUIDES, type Guide } from "../../lib/guides";
  import type { RefKind } from "../../lib/regionDoc.svelte";

  let { initial, onclose }: { initial?: Guide["id"]; onclose: () => void } = $props();

  const KEY = "mmr.logicGuide";
  function remembered(): { guide: Guide["id"]; section: string } {
    try {
      const v = JSON.parse(localStorage.getItem(KEY) ?? "null");
      if (v && GUIDES.some((g) => g.id === v.guide)) return v;
    } catch {}
    return { guide: "beginner", section: GUIDES[0].sections[0].id };
  }
  // Opened for a particular guide: continue it if it was being read, else start it.
  const saved = remembered();
  const first = untrack(() => initial);
  const start = !first || first === saved.guide ? saved : { guide: first, section: "" };
  let guideId = $state<Guide["id"]>(start.guide);
  let sectionId = $state(start.section);
  let body: HTMLElement;

  const guide = $derived(GUIDES.find((g) => g.id === guideId)!);
  const index = $derived(Math.max(0, guide.sections.findIndex((s) => s.id === sectionId)));
  const section = $derived(guide.sections[index]);

  $effect(() => {
    try {
      localStorage.setItem(KEY, JSON.stringify({ guide: guideId, section: section.id }));
    } catch {}
  });

  function show(g: Guide["id"], s?: string) {
    guideId = g;
    sectionId = s ?? GUIDES.find((x) => x.id === g)!.sections[0].id;
    body?.scrollTo({ top: 0 });
  }

  // ---- A small markup renderer (see lib/guides.ts for the syntax) ----

  const esc = (t: string) => t.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

  function inline(text: string): string {
    return text
      .split("`")
      .map((part, i) => {
        if (i % 2) return `<code>${esc(part)}</code>`;
        return esc(part)
          .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
          .replace(/\*(.+?)\*/g, "<em>$1</em>")
          .replace(/\{(\w+):([^}]+)\}/g, (_, kind: string, label: string) => {
            const color = KIND_COLORS[kind as RefKind] ?? KIND_COLORS.unknown;
            return `<span class="gtag" style="--k:${color}">${label}</span>`;
          })
          .replace(/\[([^\]]+)\]\(#([\w-]+)\)/g, '<a href="#$2" data-section="$2">$1</a>');
      })
      .join("");
  }

  function render(src: string): string {
    const lines = src.trim().split("\n");
    const out: string[] = [];
    let i = 0;
    while (i < lines.length) {
      const line = lines[i];
      if (!line.trim()) {
        i++;
      } else if (line.startsWith("### ")) {
        out.push(`<h3>${inline(line.slice(4))}</h3>`);
        i++;
      } else if (/^(- |\d+\. )/.test(line)) {
        const ordered = !line.startsWith("- ");
        const items: string[] = [];
        while (i < lines.length && (ordered ? /^\d+\. /.test(lines[i]) : lines[i].startsWith("- "))) {
          items.push(`<li>${inline(lines[i].replace(/^(- |\d+\. )/, ""))}</li>`);
          i++;
        }
        out.push(ordered ? `<ol>${items.join("")}</ol>` : `<ul>${items.join("")}</ul>`);
      } else if (line.startsWith("> ")) {
        const parts: string[] = [];
        while (i < lines.length && lines[i].startsWith("> ")) parts.push(lines[i++].slice(2));
        out.push(`<p class="tip">${inline(parts.join(" "))}</p>`);
      } else if (line.startsWith("| ")) {
        const rows: string[][] = [];
        while (i < lines.length && lines[i].startsWith("| ")) {
          rows.push(lines[i++].replace(/^\|\s*|\s*\|?\s*$/g, "").split(/\s+\|\s+/));
        }
        const [head, ...rest] = rows;
        out.push(
          `<table><thead><tr>${head.map((c) => `<th>${inline(c)}</th>`).join("")}</tr></thead><tbody>${rest
            .map((r) => `<tr>${r.map((c) => `<td>${inline(c)}</td>`).join("")}</tr>`)
            .join("")}</tbody></table>`,
        );
      } else {
        const parts: string[] = [];
        while (i < lines.length && lines[i].trim() && !/^(### |- |\d+\. |> |\| )/.test(lines[i])) parts.push(lines[i++]);
        out.push(`<p>${inline(parts.join(" "))}</p>`);
      }
    }
    return out.join("");
  }

  const html = $derived(render(section.body));

  function click(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest("a[data-section]") as HTMLAnchorElement | null;
    if (!a) return;
    e.preventDefault();
    show(guideId, a.dataset.section);
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<div class="guide" role="dialog" aria-label="Logic editor guide">
  <aside class="toc scroll">
    <div class="tracks">
      {#each GUIDES as g (g.id)}
        <button class="track" class:on={g.id === guideId} onclick={() => show(g.id)}>
          <strong>{g.title}</strong>
          <span class="small faint">{g.blurb}</span>
        </button>
      {/each}
    </div>
    <ol class="sections">
      {#each guide.sections as s, i (s.id)}
        <li>
          <button class:on={s.id === section.id} onclick={() => show(guideId, s.id)}>
            <span class="num">{i + 1}</span>{s.title}
          </button>
        </li>
      {/each}
    </ol>
  </aside>

  <div class="main">
    <header>
      <span class="faint small">{guide.title} · {index + 1} of {guide.sections.length}</span>
      <button class="icon-btn" aria-label="Close the guide" title="Close (Esc)" onclick={onclose}><Icon name="x" size={16} /></button>
    </header>
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
    <article class="scroll" bind:this={body} onclick={click}>
      <h1>{section.title}</h1>
      {@html html}
      <nav class="pager">
        {#if index > 0}
          <button class="btn" onclick={() => show(guideId, guide.sections[index - 1].id)}><Icon name="arrow-left" size={14} /> {guide.sections[index - 1].title}</button>
        {/if}
        <span class="spacer"></span>
        {#if index < guide.sections.length - 1}
          <button class="btn primary" onclick={() => show(guideId, guide.sections[index + 1].id)}>{guide.sections[index + 1].title} <Icon name="arrow-right" size={14} /></button>
        {:else}
          <button class="btn primary" onclick={onclose}>Start editing</button>
        {/if}
      </nav>
    </article>
  </div>
</div>

<style>
  .guide {
    position: absolute;
    inset: 0;
    z-index: 20;
    display: grid;
    grid-template-columns: 290px minmax(0, 1fr);
    background: var(--bg);
  }
  .toc {
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-height: 0;
    padding: 16px 12px;
    border-right: 1px solid var(--border);
    background: color-mix(in srgb, var(--surface) 80%, transparent);
  }
  .tracks {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .track {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 9px 11px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    text-align: left;
  }
  .track.on {
    border-color: var(--sec);
    background: color-mix(in srgb, var(--sec) 16%, var(--surface));
  }
  .sections {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .sections button {
    display: flex;
    gap: 8px;
    width: 100%;
    padding: 6px 8px;
    border: none;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-2);
    font-size: 13px;
    text-align: left;
  }
  .sections button:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .sections button.on {
    background: color-mix(in srgb, var(--sec) 20%, transparent);
    color: var(--text);
  }
  .num {
    flex: none;
    width: 16px;
    color: var(--text-3);
    font-variant-numeric: tabular-nums;
  }
  .main {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .main header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 18px;
    border-bottom: 1px solid var(--border);
  }
  article {
    flex: 1;
    min-height: 0;
    padding: 26px 40px 40px;
    line-height: 1.6;
  }
  article > :global(*) {
    max-width: 760px;
  }
  h1 {
    margin-bottom: 14px;
    font-size: 24px;
  }
  article :global(h3) {
    margin: 22px 0 6px;
    font-size: 15px;
  }
  article :global(p) {
    margin: 0 0 12px;
    color: var(--text-2);
  }
  article :global(ul),
  article :global(ol) {
    margin: 0 0 12px;
    padding-left: 22px;
    color: var(--text-2);
  }
  article :global(li) {
    margin: 3px 0;
  }
  article :global(strong) {
    color: var(--text);
  }
  article :global(code) {
    padding: 1px 5px;
    border-radius: 4px;
    background: var(--surface-2);
    font-size: 0.9em;
  }
  article :global(a) {
    color: var(--accent);
  }
  article :global(.tip) {
    padding: 10px 14px;
    border-left: 3px solid var(--gold);
    border-radius: var(--radius-sm);
    background: var(--gold-soft);
    color: var(--text);
  }
  article :global(.gtag) {
    display: inline-block;
    padding: 0 7px;
    border: 1px solid color-mix(in srgb, var(--k) 45%, transparent);
    border-radius: 999px;
    background: color-mix(in srgb, var(--k) 16%, transparent);
    color: var(--k);
    font-size: 0.88em;
    font-weight: 500;
    line-height: 1.5;
    white-space: nowrap;
  }
  article :global(table) {
    width: 100%;
    margin: 0 0 14px;
    border-collapse: collapse;
    font-size: 13.5px;
  }
  article :global(th),
  article :global(td) {
    padding: 7px 10px;
    border-bottom: 1px solid var(--border);
    text-align: left;
    vertical-align: top;
  }
  article :global(th) {
    color: var(--text-3);
    font-size: 11.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  article :global(td) {
    color: var(--text-2);
  }
  article :global(td:first-child) {
    width: 38%;
    color: var(--text);
  }
  .pager {
    display: flex;
    gap: 10px;
    margin-top: 28px;
    padding-top: 16px;
    border-top: 1px solid var(--border);
  }
</style>
