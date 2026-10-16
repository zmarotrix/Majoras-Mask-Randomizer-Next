<script lang="ts">
  // One form's tunic colour, instrument and energy colours, as on MMR
  // 1.16's per-form Cosmetics tabs (used by the classic layout).
  import FormCell from "./FormCell.svelte";
  import { app } from "../../lib/state.svelte";

  let { form }: { form: string } = $props();
  const hasInstrument = $derived(form in ((app.names.settings.get("Cosmetic.Instruments")?.default ?? {}) as object));
</script>

<div class="form-cosmetics">
  <span class="label" onmouseenter={() => (app.help = "Cosmetic.TunicColors")} role="presentation">Tunic color</span>
  <FormCell part="tunic" {form} />
  {#if hasInstrument}
    <span class="label" onmouseenter={() => (app.help = "Cosmetic.Instruments")} role="presentation">Instrument</span>
    <FormCell part="instruments" {form} />
  {/if}
  <span class="label" onmouseenter={() => (app.help = "Cosmetic.EnergyColors")} role="presentation">Energy color</span>
  <FormCell part="energy" {form} />
</div>

<style>
  .form-cosmetics {
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: center;
    gap: 10px 14px;
    padding: 4px 0;
  }
  .label {
    font-size: 13px;
    color: var(--text-2);
  }
</style>
