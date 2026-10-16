<script lang="ts">
  // D-Pad shortcuts: what each direction uses, laid out like the D-Pad,
  // plus on/off, which side it's shown on, and MMR 2.0's presets.
  import { app } from "../../lib/state.svelte";
  import { copy } from "../../lib/colors";

  const KEY = "Cosmetic.DPad";
  type DPadValue = { Pad: { Values: string[] }; State: string; Display: string };

  const def = $derived(app.names.settings.get(KEY)!);
  const value = $derived((app.values[KEY] ?? def.default) as DPadValue);
  const enabled = $derived(value.State === "Enabled");

  const SLOTS = ["Up", "Right", "Down", "Left"];
  const NAMES: Record<string, string> = { None: "Nothing", Ocarina: "Ocarina", DekuMask: "Deku Mask", GoronMask: "Goron Mask", ZoraMask: "Zora Mask" };
  // DPadPreset.All() in MMR 2.0.
  const PRESETS: [string, string[]][] = [
    ["Default", ["DekuMask", "ZoraMask", "Ocarina", "GoronMask"]],
    ["Virtual Console", ["None", "ZoraMask", "GoronMask", "DekuMask"]],
  ];

  function update(edit: (v: DPadValue) => void) {
    const next = copy(value);
    edit(next);
    app.setValue(KEY, next);
  }
  const setSlot = (i: number, item: string) => update((v) => (v.Pad.Values[i] = item));
</script>

<div class="dpad-editor" onmouseenter={() => (app.help = KEY)} role="group" aria-label="D-Pad">
  <div class="top">
    <label class="check">
      <input type="checkbox" checked={enabled} onchange={(e) => update((v) => (v.State = e.currentTarget.checked ? "Enabled" : "Disabled"))} />
      Use the D-Pad
    </label>
    <label class="show">
      Show on screen
      <select class="input" value={value.Display} disabled={!enabled} onchange={(e) => update((v) => (v.Display = e.currentTarget.value))}>
        <option value="Left">Left</option>
        <option value="Right">Right</option>
        <option value="None">Hidden</option>
      </select>
    </label>
  </div>
  <div class="cross" class:off={!enabled}>
    {#each SLOTS as slot, i (slot)}
      <label class="slot {slot.toLowerCase()}">
        <span class="small muted">{slot}</span>
        <select class="input" value={value.Pad.Values[i]} disabled={!enabled} onchange={(e) => setSlot(i, e.currentTarget.value)}>
          {#each def.options ?? [] as o (o.value)}<option value={o.value}>{NAMES[o.value] ?? o.label}</option>{/each}
        </select>
      </label>
    {/each}
    <span class="center" aria-hidden="true"></span>
  </div>
  <div class="presets">
    <span class="small muted">Presets:</span>
    {#each PRESETS as [name, values] (name)}
      <button class="btn small" disabled={!enabled} onclick={() => update((v) => (v.Pad.Values = [...values]))}>{name}</button>
    {/each}
  </div>
</div>

<style>
  .dpad-editor {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .top {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 20px;
  }
  .check,
  .show {
    display: flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
  }
  .check input {
    accent-color: var(--accent);
  }
  .input {
    padding: 4px 8px;
  }
  .cross {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 150px));
    grid-template-areas: ". up ." "left center right" ". down .";
    gap: 6px;
    align-items: center;
    justify-items: center;
  }
  .cross.off {
    opacity: 0.5;
  }
  .slot {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }
  .slot select {
    width: 100%;
  }
  .up {
    grid-area: up;
  }
  .down {
    grid-area: down;
  }
  .left {
    grid-area: left;
  }
  .right {
    grid-area: right;
  }
  .center {
    grid-area: center;
    width: 34px;
    height: 34px;
    border-radius: 6px;
    background: var(--surface-3);
    border: 1px solid var(--border);
  }
  .presets {
    display: flex;
    align-items: center;
    gap: 6px;
  }
</style>
