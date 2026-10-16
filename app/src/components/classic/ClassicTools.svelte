<script lang="ts">
  // Tool windows of the classic layout, like MMR 2.0's separate forms.
  import Modal from "../Modal.svelte";
  import HudColors from "../editors/HudColors.svelte";
  import DPad from "../editors/DPad.svelte";
  import HintPriorities from "../editors/HintPriorities.svelte";
  import StartingGroups from "../editors/StartingGroups.svelte";
  import { tools, type ClassicTool } from "./tools.svelte";

  const TITLES: Record<ClassicTool, [string, string]> = {
    hud: ["HUD Configuration", "Colours of the buttons, hearts, magic meter, clock, minimap and rupees."],
    dpad: ["D-Pad Configuration", "Shortcuts on the D-Pad."],
    hints: ["Customize Hint Priorities", "Choose which locations hints talk about first."],
    "random-starting": ["Random Starting Items", "Start with some items chosen at random from lists you make."],
  };
  const close = () => (tools.open = null);
</script>

{#if tools.open}
  {@const [title, subtitle] = TITLES[tools.open]}
  <Modal {title} {subtitle} wide={tools.open !== "dpad"} onclose={close}>
    {#if tools.open === "hud"}
      <HudColors />
    {:else if tools.open === "dpad"}
      <DPad />
    {:else if tools.open === "hints"}
      <HintPriorities />
    {:else}
      <StartingGroups />
    {/if}
    {#snippet footer()}
      <button class="btn primary" onclick={close}>Done</button>
    {/snippet}
  </Modal>
{/if}
