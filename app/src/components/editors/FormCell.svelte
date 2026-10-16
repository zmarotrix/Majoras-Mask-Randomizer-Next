<script lang="ts">
  // One form's tunic colour, energy colours or instrument. Colours show the
  // game's defaults until you pick one (which makes it custom); the reset
  // button goes back to the default, the dice picks random colours.
  import ColorSwatch from "./ColorSwatch.svelte";
  import Icon from "../Icon.svelte";
  import { app } from "../../lib/state.svelte";
  import { copy, formName, randomColor } from "../../lib/colors";

  let { part, form }: { part: "tunic" | "energy" | "instruments"; form: string } = $props();

  const KEYS = {
    tunic: { value: "Cosmetic.TunicColors", use: "Cosmetic.UseTunicColors" },
    energy: { value: "Cosmetic.EnergyColors", use: "Cosmetic.UseEnergyColors" },
    instruments: { value: "Cosmetic.Instruments", use: null },
  } as const;

  const keys = $derived(KEYS[part]);
  const def = $derived(app.names.settings.get(keys.value)!);
  const defaults = $derived(def.default as Record<string, unknown>);
  const value = $derived((app.values[keys.value] ?? def.default) as Record<string, unknown>);
  const custom = $derived(keys.use ? ((app.values[keys.use] ?? {}) as Record<string, boolean>)[form] === true : false);
  /** What the game will use: the custom colour, or the default. */
  const shown = $derived(custom ? value[form] : defaults[form]);
  const name = $derived(formName(form));

  function setCustom(on: boolean) {
    app.setValue(keys.use!, { ...copy((app.values[keys.use!] ?? {}) as object), [form]: on });
  }
  function setValue(v: unknown) {
    app.setValue(keys.value, { ...copy(value), [form]: v });
    if (keys.use && !custom) setCustom(true);
  }
  function setColorAt(i: number, rgb: string) {
    const list = [...((shown as string[]) ?? [])];
    list[i] = rgb;
    setValue(list);
  }
  function randomize() {
    setValue(part === "tunic" ? randomColor() : ((shown as string[]) ?? []).map(() => randomColor()));
  }
</script>

{#if part === "instruments"}
  {#if form in defaults}
    <select class="input" value={value[form] as string} onchange={(e) => setValue(e.currentTarget.value)} aria-label="{name} instrument">
      {#each def.options ?? [] as o (o.value)}
        <option value={o.value}>{o.label.replace(/([a-z])([A-Z])/g, "$1 $2")}{o.value === defaults[form] ? " (default)" : ""}</option>
      {/each}
    </select>
  {:else}
    <span class="faint small">—</span>
  {/if}
{:else}
  <span class="cell" class:custom>
    {#if part === "tunic"}
      <ColorSwatch value={shown} label="{name} tunic" onchange={setValue} />
    {:else}
      {#each (shown as string[]) ?? [] as c, i (i)}
        <ColorSwatch value={c} label="{name} energy {i + 1}" onchange={(rgb) => setColorAt(i, rgb)} />
      {/each}
    {/if}
    <button class="icon-btn" title="Random colour" aria-label="Random {name} {part} colour" onclick={randomize}><Icon name="dice" size={16} /></button>
    {#if custom}
      <button class="icon-btn" title="Back to the default colour" aria-label="Reset {name} {part} colour" onclick={() => setCustom(false)}>
        <Icon name="reset" size={15} />
      </button>
    {:else}
      <span class="tag small">default</span>
    {/if}
  </span>
{/if}

<style>
  .cell {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .tag {
    color: var(--text-3);
  }
  .custom :global(.swatch span) {
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.25),
      0 0 0 2px var(--accent-soft);
  }
  select.input {
    width: 100%;
    max-width: 200px;
    padding: 5px 8px;
  }
</style>
