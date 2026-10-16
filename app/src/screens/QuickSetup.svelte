<script lang="ts">
  // A few plain-language questions for first-time players. Each answer sets
  // a handful of settings; the current answer is detected from the values,
  // so this page stays in sync with changes made elsewhere.
  import Icon from "../components/Icon.svelte";
  import { app } from "../lib/state.svelte";
  import { summarize } from "../lib/summary";
  import type { Values } from "../lib/types";

  interface Answer {
    id: string;
    label: string;
    help: string;
    values: Values;
  }
  interface Question {
    id: string;
    title: string;
    icon: string;
    answers: Answer[];
  }

  const d = $derived(app.schema!.defaults);
  const presetList = (id: string) => (app.presets.find((p) => p.id === id)?.settings.CustomItemList as string[] | undefined) ?? [];
  const allLocations = $derived(app.game!.items.filter((i) => i.location).map((i) => i.id));

  const SPEEDUPS = ["SpeedupBeavers", "SpeedupDampe", "SpeedupDogRace", "SpeedupLabFish", "SpeedupBank", "SpeedupBabyCuccos"];

  const questions: Question[] = $derived([
    {
      id: "logic",
      title: "How experienced are you?",
      icon: "branch",
      answers: [
        { id: "casual", label: "New to this", help: "Everything can be reached the normal way. Best for a first seed.", values: { LogicMode: "Casual", EnabledTricks: [] } },
        {
          id: "tricks",
          label: "I know some tricks",
          help: "The game may expect a few common techniques, like opening chests without the Lens of Truth.",
          values: { LogicMode: "Casual", EnabledTricks: d.EnabledTricks },
        },
        { id: "glitched", label: "Glitches are fine", help: "The game may expect glitches. For experienced players.", values: { LogicMode: "Glitched" } },
      ],
    },
    {
      id: "pool",
      title: "How much should be shuffled?",
      icon: "chest",
      answers: [
        { id: "small", label: "The important stuff", help: `Masks, songs, upgrades and other key items (${(d.CustomItemList as string[]).length} checks).`, values: { CustomItemList: d.CustomItemList } },
        { id: "modern", label: "A bit more", help: `Adds more rewards and shop items (${presetList("modern").length} checks, as in the Modern preset).`, values: { CustomItemList: presetList("modern") } },
        { id: "advanced", label: "A lot", help: `Even more checks across the world (${presetList("advanced").length} checks).`, values: { CustomItemList: presetList("advanced") } },
        { id: "all", label: "Everything", help: `Every pot, rupee and blade of grass (${allLocations.length} checks). A very long game.`, values: { CustomItemList: allLocations } },
      ],
    },
    {
      id: "hints",
      title: "Do you want hints?",
      icon: "info",
      answers: [
        {
          id: "clear",
          label: "Yes, clear ones",
          help: "Gossip stones name items and places plainly, and you don't need the Mask of Truth to read them.",
          values: { GossipHintStyle: "Competitive", ClearHints: true, FreeHints: true },
        },
        {
          id: "cryptic",
          label: "Yes, but cryptic",
          help: "Hints use riddles instead of names, and you need the Mask of Truth.",
          values: { GossipHintStyle: "Competitive", ClearHints: false, FreeHints: false },
        },
        { id: "none", label: "No hints", help: "Gossip stones say what they say in the original game.", values: { GossipHintStyle: "Default", GaroHintStyle: "Default" } },
      ],
    },
    {
      id: "speed",
      title: "Skip the slow parts?",
      icon: "play",
      answers: [
        {
          id: "fast",
          label: "Yes, speed things up",
          help: "Quick text, shorter cutscenes and easier versions of slow minigames.",
          values: {
            QuickTextEnabled: true,
            "ShortenCutsceneSettings.General": d["ShortenCutsceneSettings.General"],
            "ShortenCutsceneSettings.BossIntros": d["ShortenCutsceneSettings.BossIntros"],
            ...Object.fromEntries(SPEEDUPS.map((k) => [k, d[k]])),
          },
        },
        {
          id: "original",
          label: "Keep the original pace",
          help: "Text, cutscenes and minigames play like the original game.",
          values: {
            QuickTextEnabled: false,
            "ShortenCutsceneSettings.General": [],
            "ShortenCutsceneSettings.BossIntros": [],
            ...Object.fromEntries(SPEEDUPS.map((k) => [k, false])),
          },
        },
      ],
    },
    {
      id: "challenge",
      title: "Want an extra challenge?",
      icon: "sword",
      answers: [
        { id: "normal", label: "No thanks", help: "Normal damage and health drops.", values: { DamageMode: "Default", FewerHealthDrops: false, TrapAmount: "None" } },
        { id: "harder", label: "A bit harder", help: "Link takes double damage, and a few ice traps hide in the item pool.", values: { DamageMode: "Double", FewerHealthDrops: false, TrapAmount: "Normal" } },
        {
          id: "hardest",
          label: "Much harder",
          help: "Quadruple damage, fewer health drops and extra ice traps.",
          values: { DamageMode: "Quadruple", FewerHealthDrops: true, TrapAmount: "Extra" },
        },
      ],
    },
  ]);

  function same(a: unknown, b: unknown): boolean {
    if (Array.isArray(a) && Array.isArray(b)) {
      if (a.length !== b.length) return false;
      const s = new Set(a.map(String));
      return b.every((x) => s.has(String(x)));
    }
    return a === b;
  }

  function chosen(q: Question): string | null {
    return q.answers.find((a) => Object.entries(a.values).every(([k, v]) => same(app.values[k], v)))?.id ?? null;
  }

  function pick(a: Answer) {
    for (const [k, v] of Object.entries(a.values)) app.values[k] = structuredClone($state.snapshot(v));
    app.presetId = null;
    app.checkSettings();
  }
</script>

<div class="page scroll">
  <header>
    <h1>Quick setup</h1>
    <p class="muted">Pick a preset, or answer a few questions. You can fine-tune anything later in the other sections.</p>
    <p class="small classic-hint">
      Coming from MMR 1.16? <button class="link" onclick={() => app.setLayout("classic")}>Use the classic layout</button>, with the same tabs and
      boxes as the old window. You can switch back any time.
    </p>
  </header>

  <section class="q">
    <div class="q-head">
      <span class="n"><Icon name="sliders" size={14} /></span>
      <h2>Start from a preset</h2>
    </div>
    <p class="small muted lead">The fastest way to get going. Picking one sets every option; you can look through or change them in the sections on the left.</p>
    <div class="presets">
      {#each app.presets as p (p.id)}
        <button class="preset" class:on={app.presetId === p.id} onclick={() => app.applyPreset(p)}>
          <span class="a-label">
            {#if app.presetId === p.id}<Icon name="check" size={15} />{/if}
            {p.name}
          </span>
          {#if p.description}<span class="desc">{p.description}</span>{/if}
          <span class="small muted">{summarize({ ...app.schema!.defaults, ...p.settings }, app.schema!).join(" · ")}</span>
          {#if app.presetId === p.id && p.highlights?.length}
            <ul class="highlights small">
              {#each p.highlights as h}<li>{h}</li>{/each}
            </ul>
          {/if}
        </button>
      {/each}
    </div>
  </section>

  <div class="divider"><span>or answer a few questions</span></div>

  {#each questions as q, i (q.id)}
    {@const current = chosen(q)}
    <section class="q">
      <div class="q-head">
        <span class="n">{i + 1}</span>
        <h2>{q.title}</h2>
        {#if !current}<span class="badge">Custom</span>{/if}
      </div>
      <div class="answers" role="radiogroup" aria-label={q.title}>
        {#each q.answers as a (a.id)}
          <button class="answer" role="radio" aria-checked={current === a.id} class:on={current === a.id} onclick={() => pick(a)}>
            <span class="a-label">
              {#if current === a.id}<Icon name="check" size={15} />{/if}
              {a.label}
            </span>
            <span class="small muted">{a.help}</span>
          </button>
        {/each}
      </div>
    </section>
  {/each}


  <div class="next card">
    <Icon name="cartridge" size={20} />
    <p>
      <strong>That's it.</strong> Choose your ROM in the top bar (or drop it on this window), then press <strong>Generate</strong>.
    </p>
  </div>
</div>

<style>
  .page {
    height: 100%;
    padding: 24px 28px 60px;
    display: flex;
    flex-direction: column;
    gap: 22px;
  }
  /* The page scrolls at the window's edge; its content stays readable. */
  .page > :global(*) {
    width: 100%;
    max-width: 980px;
  }
  header {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .q {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .q-head {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .n {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 700;
    font-size: 13px;
    flex: none;
  }
  h2 {
    font-size: 16px;
  }
  .answers,
  .presets {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 8px;
  }
  .answer,
  .preset {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    text-align: left;
  }
  .answer:hover,
  .preset:hover {
    border-color: var(--accent);
  }
  .answer.on,
  .preset.on {
    border-color: var(--accent);
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px var(--accent);
  }
  .a-label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
  }
  .on .a-label {
    color: var(--accent);
  }
  .lead {
    margin-top: -4px;
  }
  .desc {
    color: var(--text);
  }
  .highlights {
    margin: 6px 0 0;
    padding-left: 18px;
    color: var(--text-2);
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .presets {
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  }
  .divider {
    display: flex;
    align-items: center;
    gap: 12px;
    color: var(--text-3);
    font-size: 13px;
  }
  .divider::before,
  .divider::after {
    content: "";
    flex: 1;
    height: 1px;
    background: var(--border);
  }
  .next {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 16px;
    box-shadow: none;
    background: var(--surface-2);
  }
  .classic-hint {
    margin: 6px 0 0;
    color: var(--text-2);
  }
  .link {
    padding: 0;
    border: none;
    background: none;
    color: var(--accent);
    font: inherit;
    text-decoration: underline;
    cursor: pointer;
  }
</style>
