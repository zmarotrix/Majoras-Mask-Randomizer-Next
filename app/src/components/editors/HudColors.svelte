<script lang="ts">
  // HUD colours (MMR 2.0's HUD Configuration window): buttons, hearts,
  // magic, clock, minimap, rupees and D-Pad, grouped like the screen.
  import ColorSwatch from "./ColorSwatch.svelte";
  import { app } from "../../lib/state.svelte";
  import { copy, randomColor } from "../../lib/colors";

  const KEY = "Cosmetic.HudColors";
  const def = $derived(app.names.settings.get(KEY)!);
  const value = $derived((app.values[KEY] ?? def.default) as Record<string, string>);
  const labels = $derived(new Map((def.fields ?? []).map((f) => [f.value, f.label])));

  const GROUPS: [string, string[]][] = [
    ["Buttons", ["ButtonA", "ButtonB", "ButtonC", "ButtonStart"]],
    ["Hearts and magic", ["Heart", "HeartDD", "Magic", "MagicInf"]],
    ["Clock", ["ClockEmblem", "ClockEmblemInverted1", "ClockEmblemInverted2", "ClockEmblemSun", "ClockSun", "ClockMoon"]],
    ["Minimap", ["Map", "MapEntranceCursor", "MapPlayerCursor"]],
    ["Rupees", ["RupeeIcon1", "RupeeIcon2", "RupeeIcon3", "RupeeIcon4"]],
    ["Other", ["DPad"]],
  ];

  // Hearts / magic colours are chosen randomly unless "Customized".
  const random = $derived({
    Heart: app.values["Cosmetic.HeartsSelection"] !== "Customized",
    HeartDD: app.values["Cosmetic.HeartsSelection"] !== "Customized",
    Magic: app.values["Cosmetic.MagicSelection"] !== "Customized",
    MagicInf: app.values["Cosmetic.MagicSelection"] !== "Customized",
  } as Record<string, boolean>);

  function set(part: string, rgb: string) {
    app.setValue(KEY, { ...copy(value), [part]: rgb });
  }
  function reset() {
    app.setValue(KEY, copy(def.default));
  }
  function randomizeAll() {
    const next = copy(value);
    for (const f of def.fields ?? []) if (!random[f.value]) next[f.value] = randomColor();
    app.setValue(KEY, next);
  }
  const short = (part: string) => {
    const s = (labels.get(part) ?? part).replace(/^(Clock|Minimap|Rupees): /, "");
    return s[0].toUpperCase() + s.slice(1);
  };
</script>

<div class="hud" onmouseenter={() => (app.help = KEY)} role="group" aria-label="HUD colours">
  {#each GROUPS as [title, parts] (title)}
    <div class="group">
      <span class="title small">{title}</span>
      <div class="parts">
        {#each parts as part (part)}
          <span class="part" class:off={random[part]} title={random[part] ? "Chosen randomly (see Hearts / Magic)" : undefined}>
            <ColorSwatch value={value[part]} label={labels.get(part) ?? part} disabled={random[part]} onchange={(c) => set(part, c)} />
            <span class="small">{short(part)}</span>
          </span>
        {/each}
      </div>
    </div>
  {/each}
  <div class="actions">
    <button class="btn small" onclick={randomizeAll}>Random colours</button>
    <button class="btn small ghost" onclick={reset}>Reset to default</button>
  </div>
</div>

<style>
  .hud {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .group {
    display: grid;
    grid-template-columns: 120px 1fr;
    gap: 8px;
    align-items: start;
  }
  @media (max-width: 760px) {
    .group {
      grid-template-columns: 1fr;
    }
  }
  .title {
    padding-top: 4px;
    color: var(--text-2);
    font-weight: 600;
  }
  .parts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 16px;
  }
  .part {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .part.off .small {
    opacity: 0.5;
  }
  .actions {
    display: flex;
    gap: 8px;
  }
</style>
