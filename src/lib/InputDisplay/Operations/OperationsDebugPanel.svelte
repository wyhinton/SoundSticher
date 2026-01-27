<script lang="ts">
  import {
    deleteAllOperations,
    deleteOperationsById,
    type OperationsState,
    type OperationId,
  } from '$lib/state/operation';
  import { appState } from '$lib/state/state.svelte';
  import { operationMeta, isValidOperationKind, type OperationKind } from '$lib/types';

  export let onClose: () => void;

  function exportOperations() {
    const operationsData = $appState.operations;
    if (!operationsData) {
      alert('No operations to export');
      return;
    }

    const dataStr = JSON.stringify(operationsData, null, 2);
    const dataBlob = new Blob([dataStr], { type: 'application/json' });
    const url = URL.createObjectURL(dataBlob);

    const link = document.createElement('a');
    link.href = url;
    link.download = 'operations-export.json';
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);

    console.log('📥 Operations exported');
  }

  function importOperations() {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json';

    input.onchange = event => {
      const file = (event.target as HTMLInputElement).files?.[0];
      if (!file) return;

      const reader = new FileReader();
      reader.onload = e => {
        try {
          const importedData = JSON.parse(e.target?.result as string) as OperationsState;

          if (!importedData.defs || typeof importedData.defs !== 'object') {
            throw new Error('Invalid operations file format');
          }

          appState.update(state => {
            state.operations = {
              defs: importedData.defs,
              pipelines: importedData.pipelines || {},
              _version: (importedData._version || 0) + 1,
            };
            state._rev = (state._rev || 0) + 1;
            return state;
          });

          console.log('📤 Operations imported successfully');
        } catch (error) {
          console.error('Failed to import operations:', error);
          alert('Failed to import operations: Invalid file format');
        }
      };
      reader.readAsText(file);
    };

    input.click();
  }

  function getOperationsStats() {
    const operations = $appState.operations;
    if (!operations) return { totalOps: 0, totalPipelines: 0, byCategory: {} };

    const defs = Object.values(operations.defs);
    const byCategory: Record<string, number> = {};

    defs.forEach(def => {
      if (isValidOperationKind(def.kind)) {
        const category = operationMeta[def.kind as OperationKind].category;
        byCategory[category] = (byCategory[category] || 0) + 1;
      }
    });

    return {
      totalOps: defs.length,
      totalPipelines: Object.keys(operations.pipelines || {}).length,
      byCategory,
    };
  }

  $: stats = getOperationsStats();
  $: operationsList = $appState.operations?.defs ? Object.entries($appState.operations.defs) : [];
</script>

<div class="debug-panel">
  <div class="debug-header">
    <span class="debug-title">
      <i class="fa fa-bug"></i>
      Operations Debug
    </span>
    <button
      class="btn-close"
      onclick={onClose}
      title="Close debug panel"
      aria-label="Close debug panel"
    >
      <i class="fa fa-times"></i>
    </button>
  </div>

  <div class="debug-buttons">
    <div class="button-group">
      <span class="group-title">Stats</span>
      <div class="stats-row">
        <span class="stat-badge">Ops: {stats.totalOps}</span>
        <span class="stat-badge">Pipes: {stats.totalPipelines}</span>
      </div>
      <div class="stats-row">
        {#each Object.entries(stats.byCategory) as [category, count]}
          <span class="stat-badge stat-{category}">{category}: {count}</span>
        {/each}
      </div>
    </div>

    <div class="button-group">
      <span class="group-title">Actions</span>
      <button
        class="btn btn-xs btn-outline-danger"
        onclick={() => {
          if (confirm('Delete all operations? This cannot be undone.')) {
            deleteAllOperations();
          }
        }}
        title="Delete all operations"
      >
        <i class="fa fa-trash"></i>
        Delete All
      </button>
    </div>

    <div class="button-group">
      <span class="group-title">File I/O</span>
      <button
        class="btn btn-xs btn-outline-secondary"
        onclick={exportOperations}
        title="Export operations to JSON"
      >
        <i class="fa fa-download"></i>
        Export
      </button>
      <button
        class="btn btn-xs btn-outline-secondary"
        onclick={importOperations}
        title="Import operations from JSON"
      >
        <i class="fa fa-upload"></i>
        Import
      </button>
    </div>
  </div>

  <div class="operations-list">
    <span class="group-title">Defined Operations</span>
    {#if operationsList.length === 0}
      <div class="empty-message">No operations defined</div>
    {:else}
      <div class="list-container">
        {#each operationsList as [id, def]}
          {#if isValidOperationKind(def.kind)}
            {@const info = operationMeta[def.kind as OperationKind]}
            <div class="operation-item">
              <span class="op-icon">{info.icon}</span>
              <div class="op-info">
                <span class="op-name">{def.name}</span>
                <span class="op-kind">{info.label}</span>
                <span class="op-id" title={id}>{id.substring(0, 12)}...</span>
              </div>
              <span class="op-category category-{info.category}">{info.category}</span>
              <button
                class="btn-delete"
                onclick={() => deleteOperationsById([id])}
                title="Delete operation"
                aria-label="Delete operation {def.name}"
              >
                <i class="fa fa-times"></i>
              </button>
            </div>
          {/if}
        {/each}
      </div>
    {/if}
  </div>

  <div class="debug-info">
    <small>
      <i class="fa fa-info-circle"></i>
      Operations are re-evaluated when the content revision (_rev) changes.
    </small>
  </div>
</div>

<style>
  .debug-panel {
    background: #1a1b26;
    border: 1px solid #3b4261;
    border-radius: 8px;
    padding: 12px;
    font-size: 0.85rem;
    max-height: 400px;
    overflow-y: auto;
  }

  .debug-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
    padding-bottom: 8px;
    border-bottom: 1px solid #3b4261;
  }

  .debug-title {
    font-weight: 600;
    color: #a9b1d6;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .btn-close {
    background: transparent;
    border: none;
    color: #565f89;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 4px;
    transition: all 0.2s;
  }

  .btn-close:hover {
    background: #3b4261;
    color: #f7768e;
  }

  .debug-buttons {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 12px;
  }

  .button-group {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }

  .group-title {
    font-size: 0.7rem;
    color: #565f89;
    text-transform: uppercase;
    font-weight: 600;
    width: 100%;
    margin-bottom: 4px;
  }

  .stats-row {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .stat-badge {
    background: #3b4261;
    color: #a9b1d6;
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .stat-render {
    background: rgba(245, 158, 11, 0.2);
    color: #f59e0b;
  }

  .stat-edit {
    background: rgba(139, 92, 246, 0.2);
    color: #8b5cf6;
  }

  .stat-meta {
    background: rgba(34, 197, 94, 0.2);
    color: #22c55e;
  }

  .btn-xs {
    font-size: 0.7rem;
    padding: 4px 8px;
  }

  .operations-list {
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid #3b4261;
  }

  .list-container {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 8px;
    max-height: 200px;
    overflow-y: auto;
  }

  .operation-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px;
    background: #24283b;
    border-radius: 6px;
    transition: background 0.2s;
  }

  .operation-item:hover {
    background: #3b4261;
  }

  .op-icon {
    font-size: 1.1rem;
  }

  .op-info {
    flex: 1;
    display: flex;
    flex-direction: column;
  }

  .op-name {
    font-weight: 500;
    color: #c0caf5;
    font-family: 'Fira Code', monospace;
    font-size: 0.8rem;
  }

  .op-kind {
    font-size: 0.7rem;
    color: #565f89;
  }

  .op-category {
    font-size: 0.65rem;
    padding: 2px 6px;
    border-radius: 4px;
    text-transform: uppercase;
    font-weight: 500;
  }

  .category-render {
    background: rgba(245, 158, 11, 0.2);
    color: #f59e0b;
  }

  .category-edit {
    background: rgba(139, 92, 246, 0.2);
    color: #8b5cf6;
  }

  .category-meta {
    background: rgba(34, 197, 94, 0.2);
    color: #22c55e;
  }

  .btn-delete {
    background: transparent;
    border: none;
    color: #565f89;
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    transition: all 0.2s;
  }

  .btn-delete:hover {
    background: rgba(247, 118, 142, 0.2);
    color: #f7768e;
  }

  .empty-message {
    color: #565f89;
    font-style: italic;
    padding: 12px;
    text-align: center;
  }

  .debug-info {
    margin-top: 12px;
    padding-top: 8px;
    border-top: 1px solid #3b4261;
    color: #565f89;
  }

  .debug-info i {
    margin-right: 4px;
  }
</style>
