<script lang="ts">
  // First launch: choose the guided (new) or classic (like the old
  // randomizer) layout. Either can be switched later.
  import Icon from "./Icon.svelte";
  import { app, type Layout } from "../lib/state.svelte";

  const CHOICES: { id: Layout; title: string; tag: string; text: string; points: string[]; icon: string }[] = [
    {
      id: "guided",
      title: "Guided",
      tag: "New",
      icon: "sparkles",
      text: "Start from a preset or answer a few questions. Settings are grouped by topic, with explanations as you go.",
      points: ["Best if you're new to randomizers", "Presets and step-by-step setup", "Plain-English help for every option"],
    },
    {
      id: "classic",
      title: "Classic",
      tag: "Familiar",
      icon: "columns",
      text: "The same tabs and boxes as MMR 1.16, so everything is where you remember it.",
      points: ["Best if you've used MMR before", "Main Settings, Gimmicks, Comfort… tabs", "Everything on one screen"],
    },
  ];
</script>

<div class="backdrop">
  <div class="panel card" role="dialog" aria-modal="true" aria-labelledby="choose-title">
    <img src="/logo.png" alt="" width="56" height="56" />
    <h1 id="choose-title">Welcome to Majora's Mask Randomizer Next</h1>
    <p class="muted">Which layout would you like? You can switch any time.</p>
    <div class="choices">
      {#each CHOICES as c (c.id)}
        <button class="choice" onclick={() => app.setLayout(c.id)}>
          <span class="head">
            <span class="icon"><Icon name={c.icon} size={20} /></span>
            <span class="title">{c.title}</span>
            <span class="tag small">{c.tag}</span>
          </span>
          <span class="text">{c.text}</span>
          <ul class="small muted">
            {#each c.points as p (p)}<li>{p}</li>{/each}
          </ul>
          <span class="go">Use the {c.title.toLowerCase()} layout <Icon name="arrow-right" size={14} /></span>
        </button>
      {/each}
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: grid;
    place-items: center;
    padding: 16px;
    overflow: auto;
    background: color-mix(in srgb, var(--bg) 70%, transparent);
    backdrop-filter: blur(6px);
  }
  .panel {
    width: min(760px, 100%);
    padding: 28px;
    text-align: center;
    box-shadow: var(--shadow-lg);
  }
  h1 {
    margin: 10px 0 4px;
    font-size: 22px;
  }
  .panel > p {
    margin: 0 0 20px;
  }
  .choices {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
  }
  @media (max-width: 640px) {
    .choices {
      grid-template-columns: 1fr;
    }
  }
  .choice {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 18px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-2);
    color: var(--text);
    text-align: left;
    cursor: pointer;
    transition:
      border-color 0.15s,
      transform 0.15s;
  }
  .choice:hover,
  .choice:focus-visible {
    border-color: var(--accent);
    transform: translateY(-2px);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .icon {
    display: inline-grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: 10px;
    background: var(--accent-soft);
    color: var(--accent);
  }
  .title {
    font-size: 17px;
    font-weight: 700;
  }
  .tag {
    margin-left: auto;
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--surface-3);
    color: var(--text-2);
  }
  .text {
    line-height: 1.45;
  }
  ul {
    margin: 0;
    padding-left: 18px;
  }
  .go {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-top: auto;
    color: var(--accent);
    font-weight: 600;
  }
</style>
