<script lang="ts">
  // Classic-layout controls that aren't a single setting.
  import Modal from "../Modal.svelte";
  import TricksPicker from "../TricksPicker.svelte";
  import PoolEditor from "../pool/PoolEditor.svelte";
  import { backend } from "../../lib/backend";
  import { app, toasts } from "../../lib/state.svelte";
  import { decodeList, encodeList, poolSize, type ListPool } from "../../lib/legacyList";
  import { shuffledChecks, startingItemChecks } from "../../lib/pool";
  import type { ClassicCustom } from "../../lib/classic";
  import TrapWeights from "../editors/TrapWeights.svelte";
  import FormCosmetics from "../editors/FormCosmetics.svelte";
  import { openTool } from "./tools.svelte";

  let { kind, label, form }: { kind: ClassicCustom; label?: string; form?: string } = $props();

  let editing = $state(false);

  // Item lists ("Extra Starting Items" / "Enforce Junk Locations").
  const listInfo = $derived(
    kind === "starting-items"
      ? { key: "CustomStartingItemList", pool: "starting" as ListPool, noun: "items" }
      : { key: "CustomJunkLocations", pool: "locations" as ListPool, noun: "locations" },
  );
  const list = $derived((app.values[listInfo.key] as string[] | undefined) ?? []);
  // Only shuffled checks can be junk locations.
  const junkable = $derived(shuffledChecks(app.game!, (app.values.CustomItemList as string[] | undefined) ?? []));
  const junkIds = $derived(new Set(junkable.map((c) => c.id)));
  const shownCount = $derived(kind === "junk-locations" ? list.filter((id) => junkIds.has(id)).length : list.length);
  const shownTotal = $derived(kind === "junk-locations" ? junkable.length : poolSize(app.game!, listInfo.pool));
  const code = $derived(app.game ? encodeList(app.game, listInfo.pool, list) : "");
  function pasteCode(text: string) {
    const items = decodeList(app.game!, listInfo.pool, text);
    if (items) app.setValue(listInfo.key, items);
    else toasts.show("That list code is from a different randomizer version, so it can't be read.", "error");
  }

  // Hint count overrides: a checkbox, then Way of the Hero / Foolish / Max in Clock Town.
  const countKeys = $derived(
    kind === "gossip-counts"
      ? ["OverrideNumberOfRequiredGossipHints", "OverrideNumberOfNonRequiredGossipHints", "OverrideMaxNumberOfClockTownGossipHints"]
      : ["OverrideNumberOfRequiredGaroHints", "OverrideNumberOfNonRequiredGaroHints", "OverrideMaxNumberOfClockTownGaroHints"],
  );
  const overriding = $derived(countKeys.some((k) => app.values[k] !== null && app.values[k] !== undefined));
  const countsOff = $derived(countKeys.some((k) => app.inactive.has(k)));
  function setOverride(on: boolean) {
    const initial = [3, 3, 2];
    countKeys.forEach((k, i) => app.setValue(k, on ? initial[i] : null));
  }

  async function openLogic() {
    const path = await backend.pickFile("Open logic", [{ name: "Logic file", extensions: ["json", "txt"] }]);
    if (path) app.setValue("UserLogicFileName", path);
  }

  const tricks = $derived((app.values.EnabledTricks as string[] | undefined) ?? []);
  const userLogic = $derived(app.values.LogicMode === "UserLogic");
</script>

{#if kind === "tricks"}
  <div class="line">
    <span class="small muted">{tricks.length} tricks enabled</span>
    <button class="btn small" onclick={() => (editing = true)} disabled={app.values.LogicMode === "NoLogic"}>Edit Tricks</button>
  </div>
  {#if editing}
    <TricksPicker value={tricks} onsave={(v) => app.setValue("EnabledTricks", v)} onclose={() => (editing = false)} />
  {/if}
{:else if kind === "user-logic"}
  <div class="line">
    <button class="btn small" disabled={!userLogic} onclick={openLogic}>Open Logic</button>
    <input class="input grow" readonly disabled={!userLogic} value={String(app.values.UserLogicFileName ?? "")} aria-label="Logic file" />
  </div>
{:else if kind === "starting-items" || kind === "junk-locations"}
  <span class="small muted"
    >{shownCount}/{shownTotal} {kind === "junk-locations" ? "shuffled locations" : listInfo.noun} selected</span
  >
  <div class="line">
    <input class="input grow mono" value={code} onchange={(e) => pasteCode(e.currentTarget.value)} aria-label="List code" spellcheck="false" />
    <button class="btn small" onclick={() => (editing = true)}>Edit</button>
  </div>
  {#if editing}
    <Modal title={kind === "starting-items" ? "Extra Starting Items" : "Enforce Junk Locations"} wide onclose={() => (editing = false)}>
      {#if kind === "starting-items"}
        <PoolEditor
          id="classic-starting"
          kind="items"
          checks={startingItemChecks(app.game!)}
          value={list}
          onchange={(v) => app.setValue(listInfo.key, v)}
          views={["browse", "lists"]}
          labels={{ on: "Starting with", off: "Not starting with", add: "Start with", remove: "Remove", noun: "items" }}
        />
      {:else}
        <PoolEditor
          id="classic-junk"
          kind="locations"
          checks={junkable}
          value={list}
          onchange={(v) => app.setValue(listInfo.key, v)}
          views={["lists"]}
          labels={{ on: "Junk", off: "Normal", add: "Make junk", remove: "Make normal", noun: "checks" }}
        />
      {/if}
      {#snippet footer()}
        <button class="btn primary" onclick={() => (editing = false)}>Done</button>
      {/snippet}
    </Modal>
  {/if}
{:else if kind === "trap-weights"}
  <span class="sub">{label}</span>
  <TrapWeights />
{:else if kind === "gossip-counts" || kind === "garo-counts"}
  <div class="counts" class:off={countsOff} onmouseenter={() => (app.help = countKeys[0])} role="group" aria-label="Hint counts">
    <input type="checkbox" checked={overriding} onchange={(e) => setOverride(e.currentTarget.checked)} title="Choose the number of hints" />
    <div class="cols">
      <span class="small muted">WotH / Foolish / Max CT</span>
      <div class="nums">
        {#each countKeys as k, i (k)}
          {#if i > 0}<span class="faint">/</span>{/if}
          <input
            class="input num"
            type="number"
            min="0"
            disabled={!overriding}
            value={(app.values[k] as number | null) ?? ""}
            onchange={(e) => app.setValue(k, Number(e.currentTarget.value))}
            aria-label={["Way of the Hero hints", "Foolish hints", "Most hints in Clock Town"][i]}
          />
        {/each}
      </div>
    </div>
  </div>
{:else if kind === "hint-priorities"}
  <button class="btn small" onclick={() => openTool("hints")}>{label}</button>
{:else if kind === "random-starting"}
  <div class="line">
    <span class="small muted grow-text">{((app.values.RandomStartingItemGroups as unknown[]) ?? []).length} random groups</span>
    <button class="btn small" onclick={() => openTool("random-starting")}>{label}</button>
  </div>
{:else if kind === "hud-colors"}
  <button class="btn small" onclick={() => openTool("hud")}>{label}</button>
{:else if kind === "form-cosmetics" && form}
  <FormCosmetics {form} />
{/if}

<style>
  .line {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 0;
  }
  .grow {
    flex: 1;
    min-width: 0;
    padding: 4px 8px;
  }
  .mono {
    font-family: var(--mono);
    font-size: 12px;
  }
  .sub {
    display: block;
    margin-top: 4px;
    font-size: 13px;
    color: var(--text-2);
  }
  .num {
    width: 58px;
    padding: 3px 6px;
  }
  .counts {
    display: flex;
    align-items: flex-end;
    gap: 8px;
    padding-top: 4px;
  }
  .counts > input {
    width: 15px;
    height: 15px;
    margin: 0 0 8px;
    accent-color: var(--accent);
  }
  .cols {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .nums {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .off {
    opacity: 0.5;
  }
  .grow-text {
    flex: 1;
  }
</style>
