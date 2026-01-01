<script lang="ts">
  import type { GroupDef } from '../state/groups';

  export let groupName: string;
  export let definition: GroupDef;
  export let isSelected: boolean = false;
  export let resultCount: number | null = null;
  export let onSelect: (groupName: string) => void;

  // Get group type for display
  function getGroupType(def: GroupDef): string {
    switch (def.kind) {
      case 'query':
        return def.query.kind;
      case 'op':
        return `${def.op} (${def.refs.length} refs)`;
      case 'not':
        return `not ${def.ref}`;
      default:
        return 'unknown';
    }
  }
</script>

<div class="group-item" class:selected={isSelected} onclick={() => onSelect(groupName)}>
  <div class="group-main">
    <span class="group-name">{groupName}</span>
    <span class="group-type">{getGroupType(definition)}</span>
  </div>
  <div class="group-details">
    <span class="result-count">
      {#if resultCount !== null}
        {resultCount} items
      {:else}
        <em>not evaluated</em>
      {/if}
    </span>
  </div>
</div>

<style>
  .group-item {
    margin: 2px 0;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.2s ease;
    border: 1px solid transparent;
  }

  .group-item:hover {
    background: #2a2a2a;
    border-color: #555;
  }

  .group-item.selected {
    background: #1e40af;
    border-color: #3b82f6;
  }

  .group-main {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 4px;
  }

  .group-name {
    font-weight: 500;
    color: #fff;
    font-size: 12px;
  }

  .group-type {
    color: #888;
    font-size: 10px;
    background: #444;
    padding: 2px 6px;
    border-radius: 3px;
  }

  .group-details {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .result-count {
    color: #888;
    font-size: 10px;
  }
</style>
