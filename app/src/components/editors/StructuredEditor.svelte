<script lang="ts" module>
  /** Settings the guided layout shows inside another setting's editor
   * (the forms table shows tunic, energy and instrument together). */
  export const MERGED = new Set(["Cosmetic.EnergyColors", "Cosmetic.Instruments"]);
</script>

<script lang="ts">
  // The editor for a structured ("custom") setting.
  import FormsTable from "./FormsTable.svelte";
  import HudColors from "./HudColors.svelte";
  import DPad from "./DPad.svelte";
  import TrapWeights from "./TrapWeights.svelte";
  import StartingGroups from "./StartingGroups.svelte";
  import HintPriorities from "./HintPriorities.svelte";
  import type { SettingDef } from "../../lib/types";

  let { def }: { def: SettingDef } = $props();
</script>

{#if def.editor === "tunic-colors"}
  <FormsTable />
{:else if def.editor === "hud-colors"}
  <HudColors />
{:else if def.editor === "dpad"}
  <DPad />
{:else if def.editor === "trap-weights"}
  <TrapWeights />
{:else if def.editor === "starting-groups"}
  <StartingGroups />
{:else if def.editor === "hint-priorities"}
  <HintPriorities />
{:else}
  <span class="small faint">No editor for this setting yet</span>
{/if}
