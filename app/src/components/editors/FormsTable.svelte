<script lang="ts">
  // Link's forms at a glance: tunic, energy colours and instrument per form.
  import FormCell from "./FormCell.svelte";
  import { app } from "../../lib/state.svelte";
  import { formName } from "../../lib/colors";

  const forms = $derived(app.names.settings.get("Cosmetic.TunicColors")?.forms ?? []);
  const instruments = $derived((app.names.settings.get("Cosmetic.Instruments")?.default ?? {}) as Record<string, string>);
</script>

<!-- Hovering a column shows its explanation in the help panel. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="table">
  <div class="row head" aria-hidden="true">
    <span>Form</span>
    <span onmouseenter={() => (app.help = "Cosmetic.TunicColors")}>Tunic</span>
    <span onmouseenter={() => (app.help = "Cosmetic.EnergyColors")}>Energy</span>
    <span onmouseenter={() => (app.help = "Cosmetic.Instruments")}>Instrument</span>
  </div>
  {#each forms as form (form)}
    <div class="row">
      <span class="form">{formName(form)}</span>
      <span class="cell" data-label="Tunic" onmouseenter={() => (app.help = "Cosmetic.TunicColors")}><FormCell part="tunic" {form} /></span>
      <span class="cell" data-label="Energy" onmouseenter={() => (app.help = "Cosmetic.EnergyColors")}><FormCell part="energy" {form} /></span>
      <span class="cell" class:none={!(form in instruments)} data-label="Instrument" onmouseenter={() => (app.help = "Cosmetic.Instruments")}><FormCell part="instruments" {form} /></span>
    </div>
  {/each}
</div>

<style>
  .table {
    display: flex;
    flex-direction: column;
    container-type: inline-size;
  }
  .row {
    display: grid;
    grid-template-columns: 110px minmax(150px, 1fr) minmax(190px, 1.3fr) minmax(150px, 1fr);
    align-items: center;
    gap: 12px;
    padding: 8px 4px;
    border-top: 1px solid var(--border);
  }
  .row.head {
    border-top: none;
    padding-top: 0;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-3);
  }
  .form {
    font-weight: 600;
  }
  /* Narrow: one block per form, labelled cells. */
  @container (max-width: 620px) {
    .row.head {
      display: none;
    }
    .row {
      grid-template-columns: 90px 1fr;
      row-gap: 8px;
    }
    .form {
      grid-row: 1 / span 3;
      align-self: start;
      padding-top: 4px;
    }
    .cell {
      display: flex;
      align-items: center;
      gap: 10px;
    }
    .cell.none {
      display: none;
    }
    .cell::before {
      content: attr(data-label);
      width: 74px;
      flex: none;
      font-size: 12.5px;
      color: var(--text-3);
    }
  }
</style>
