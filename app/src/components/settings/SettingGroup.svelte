<script lang="ts">
  // A group of related settings as a card (search results, item pool rules).
  import Icon from "../Icon.svelte";
  import GroupForm from "./GroupForm.svelte";
  import type { LayoutGroup } from "../../lib/types";

  let { group, only = null }: { group: LayoutGroup; only?: Set<string> | null } = $props();
</script>

<section class="card group">
  <header>
    <span class="icon"><Icon name={group.icon} size={16} /></span>
    <div>
      <h3>{group.title}</h3>
      <p class="small muted">{group.description}</p>
    </div>
  </header>
  <GroupForm {group} {only} />
</section>

<style>
  .group {
    padding: 16px 18px 18px;
    box-shadow: none;
    margin-bottom: 16px;
  }
  header {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    margin-bottom: 14px;
  }
  .icon {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 8px;
    background: var(--accent-soft);
    color: var(--accent);
    flex: none;
  }
  h3 {
    font-size: 15px;
  }
</style>
