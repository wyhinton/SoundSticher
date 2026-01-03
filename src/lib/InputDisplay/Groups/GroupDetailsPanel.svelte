<script lang="ts">
  import { type GroupDef, type ItemQuery } from '$lib/state/groups';
  import GroupParams from './GroupParams.svelte';

  export let groupName: string;
  export let definition: GroupDef;
  export let result: Set<string> | null = null;
  export let onClose: (() => void) | undefined = undefined;
  export let onUpdateQuery: ((groupName: string, patch: Partial<ItemQuery>) => void) | null = null;
</script>

<div class="group-details-panel">
  <div class="panel-header">
    <h5>Group: {groupName}</h5>
    <div class="header-controls">
      {#if onClose}
        <button
          class="close-button"
          onclick={onClose}
          title="Close group details"
          aria-label="Close group details"
        >
          ×
        </button>
      {/if}
    </div>
  </div>
  <!-- <div class="definition">
    <strong>Definition:</strong>
    <pre>{JSON.stringify(definition, null, 2)}</pre>
  </div> -->

  {#if definition.kind === 'query' && onUpdateQuery}
    <div class="edit-section">
      <strong>Edit Parameters:</strong>
      <GroupParams {groupName} {definition} {onUpdateQuery} />
    </div>
  {/if}

  {#if result}
    <div class="results">
      <strong>Results ({result.size} items):</strong>
      <div class="result-list">
        {#each Array.from(result) as itemId}
          <span class="result-item">{itemId}</span>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .group-details-panel {
    border-top: 1px solid #444;
    padding-top: 12px;
    max-height: 40%;
    overflow-y: auto;
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }

  .header-controls {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .group-details-panel h5 {
    margin: 0;
    font-size: 13px;
    color: #fff;
  }

  .close-button {
    background: transparent;
    border: 1px solid #666;
    color: #ccc;
    width: 20px;
    height: 20px;
    border-radius: 3px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 14px;
    line-height: 1;
    padding: 0;
    transition: all 0.2s ease;
  }

  .close-button:hover {
    background: #444;
    border-color: #888;
    color: #fff;
  }

  .close-button:active {
    background: #555;
  }

  .edit-section {
    margin-bottom: 12px;
  }

  .edit-section strong {
    font-size: 11px;
    color: #ccc;
  }

  .results strong {
    font-size: 11px;
    color: #ccc;
  }

  .result-list {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 4px;
  }

  .result-item {
    background: #444;
    color: #fff;
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 10px;
    font-family: monospace;
  }

  /* Scrollbar styling */
  .group-details-panel::-webkit-scrollbar {
    width: 6px;
  }

  .group-details-panel::-webkit-scrollbar-track {
    background: #1a1a1a;
  }

  .group-details-panel::-webkit-scrollbar-thumb {
    background: #444;
    border-radius: 3px;
  }

  .group-details-panel::-webkit-scrollbar-thumb:hover {
    background: #555;
  }
</style>
