<script lang="ts">
  import { appState } from '$lib/state/state.svelte';
  import { type MergeOp, type OperationDef } from '$lib/state/operation';
  import {
    dispatch,
    type AddOperationCommand,
    type DeleteMultipleOperationsCommand,
  } from '$lib/state/undo';
  import MergeOpFlow from './MergeOpFlow.svelte';
  import { dropzone } from '$lib/attachments/droppable';
  import { SvelteFlowProvider } from '@xyflow/svelte';

  // Panel visibility
  export let isExpanded = true;
  // Panel height passed from parent Splitpanes resize events
  export let panelHeight: number = 0;
  // @ts-ignore - panelHeight is used for tracking/debugging resize events

  // Log panel height whenever it changes
  $: if (panelHeight > 0) {
    console.log('📏 OperationsFlowPanel height changed:', panelHeight);
  }

  $: selectedOperationId = $appState.uiSettings?.selectedOperationId || null;

  // Get MergeOp operations with revision tracking
  $: mergeOperations = $appState.operations?.defs
    ? Object.entries($appState.operations.defs)
        .filter(([id, def]) => def.kind === 'merge')
        .map(([id, def]) => {
          // Create a revision key that includes sources data to ensure re-rendering
          const sourcesHash = JSON.stringify(def.sources || []);
          console.log(id);
          return {
            id,
            name: def.name,
            operation: def as MergeOp,
            revisionKey: `${id}-${sourcesHash}-${$appState._rev || 0}`,
          };
        })
    : [];

  // Stats for MergeOps only
  $: stats = {
    total: mergeOperations.length,
    merge: mergeOperations.length,
  };

  // Add merge operation
  function addMergeOperation() {
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    const operationName = `merge_${timestamp}`;

    // Use the undo system to add the operation
    const command: AddOperationCommand = {
      type: 'add-operation',
      operation: {
        name: operationName,
        kind: 'merge',
        sources: [],
        outputPath: `output/combined_${timestamp}.wav`,
        gapSeconds: 0,
        format: 'wav',
      } as Omit<OperationDef, 'id'>,
    };

    dispatch(command, `Add Merge Operation: ${operationName}`);
  }

  // Delete all operations using undo system
  function handleDeleteAllOperations() {
    if (!confirm('Delete all operations?')) return;

    // Get all current operation IDs
    const allOperationIds = $appState.operations?.order || [];

    if (allOperationIds.length === 0) {
      console.log('No operations to delete');
      return;
    }

    // Use the undo system to delete all operations
    const command: DeleteMultipleOperationsCommand = {
      type: 'delete-multiple-operations',
      operationIds: allOperationIds,
    };

    dispatch(command, `Delete All Operations (${allOperationIds.length} operations)`);
  }

  // Resize functionality - removed
</script>

<div class="operations-flow-panel h-fill-available" class:collapsed={!isExpanded}>
  <div
    class="panel-header"
    style="--header-bg: {$appState.uiSettings?.theme?.panelHeaderBackgroundColor}"
  >
    <div class="header-left">
      <button
        class="toggle-btn"
        onclick={() => (isExpanded = !isExpanded)}
        title={isExpanded ? 'Collapse panel' : 'Expand panel'}
        aria-label={isExpanded ? 'Collapse panel' : 'Expand panel'}
      >
        <i class="fa fa-{isExpanded ? 'chevron-down' : 'chevron-right'}"></i>
      </button>
      <span class="panel-title">
        <i class="fa fa-project-diagram"></i>
        Operations
      </span>
      <div class="stats-badges">
        <span class="badge badge-total" title="Total MergeOp operations">{stats.total}</span>
        {#if stats.merge > 0}
          <span class="badge badge-merge" title="Merge operations">� {stats.merge}</span>
        {/if}
      </div>
    </div>
    <div class="header-actions">
      <button
        class="btn btn-xs btn-outline-danger"
        onclick={handleDeleteAllOperations}
        title="Delete all operations"
        aria-label="Delete all operations"
      >
        <i class="fa fa-trash"></i>
      </button>
    </div>
  </div>

  {#if isExpanded}
    <div class="panel-content" style:height={200}>
      <div class="flow-container">
        {#if mergeOperations.length > 0}
          <!-- Show MergeOpFlow components for each merge operation -->
          <div
            class="merge-flows-row h-100 d-flex"
            use:dropzone={{
              accepts: ['sample'],
              on_drop: ({ data, sourceId }) => {
                console.log('Dropped sample:', data, sourceId);
              },
            }}
          >
            {#each mergeOperations as mergeOp (mergeOp.revisionKey)}
              <SvelteFlowProvider>
                <MergeOpFlow
                  operation={mergeOp.operation}
                  operationId={mergeOp.id}
                  operationName={mergeOp.name}
                  isSelected={selectedOperationId === mergeOp.id}
                  {panelHeight}
                />
              </SvelteFlowProvider>
            {/each}
          </div>
        {:else}
          <div class="empty-state">
            <i class="fa fa-project-diagram fa-3x"></i>
            <p>No merge operations defined</p>
            <button class="btn btn-sm btn-primary" onclick={addMergeOperation}>
              <i class="fa fa-plus"></i> Add Merge Operation
            </button>
          </div>
        {/if}
      </div>

      <div class="operation-creation-panel">
        <div
          class="creation-header"
          style="--header-bg: {$appState.uiSettings?.theme?.panelHeaderBackgroundColor}"
        >
          <h4>Add Operations</h4>
        </div>
        <div class="operation-buttons">
          <button class="operation-add-btn" onclick={addMergeOperation} title="Add merge operation">
            <span class="operation-icon">🔗</span>
            <span class="operation-label">Merge</span>
            <i class="fa fa-plus"></i>
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .operations-flow-panel {
    background: var(--panel-bg, #1e1e2e);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    width: 100%;
  }

  .operations-flow-panel.collapsed {
    height: auto;
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 4px 8px;
    /* background: var(--header-bg); */
    background-color: #161616;
    border-bottom: 1px solid var(--border-color, #313244);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .toggle-btn {
    background: transparent;
    border: none;
    color: var(--text-muted, #a6adc8);
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 3px;
    transition: background 0.2s;
    font-size: 0.75rem;
  }

  .toggle-btn:hover {
    background: var(--hover-bg, #313244);
  }

  .panel-title {
    font-weight: 600;
    color: var(--text-primary, #cdd6f4);
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 0.8rem;
  }

  .stats-badges {
    display: flex;
    gap: 3px;
    margin-left: 6px;
  }

  .badge {
    font-size: 0.65rem;
    padding: 1px 4px;
    border-radius: 3px;
    font-weight: 500;
    line-height: 1.2;
  }

  .badge-total {
    background: var(--badge-bg, #45475a);
    color: var(--text-primary, #cdd6f4);
  }

  .badge-merge {
    background: rgba(245, 158, 11, 0.2);
    color: #f59e0b;
  }

  .header-actions {
    display: flex;
    gap: 2px;
  }

  .btn-xs {
    font-size: 0.65rem;
    padding: 2px 4px;
    line-height: 1.2;
  }

  .panel-content {
    display: flex;
    width: 100%;
    flex: 1;
  }

  .flow-container {
    flex: 1;
    position: relative;
    border-right: 1px solid var(--border-color, #313244);
  }

  .merge-flows-row {
    height: 100%;
    overflow-x: auto;
    overflow-y: hidden;
  }

  .operation-creation-panel {
    width: 200px;
    background: var(--panel-bg, #1e1e2e);
    border-left: 1px solid var(--border-color, #313244);
    display: flex;
    flex-direction: column;
  }

  .creation-header {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-color, #313244);
    background: var(--header-bg, #181825);
  }

  .creation-header h4 {
    margin: 0;
    font-size: 0.8rem;
    color: var(--text-primary, #cdd6f4);
  }

  .operation-buttons {
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .operation-add-btn {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
    background: var(--panel-bg, #1e1e2e);
    border: 1px solid var(--border-color, #313244);
    border-radius: 6px;
    color: var(--text-primary, #cdd6f4);
    cursor: pointer;
    transition: all 0.2s;
    font-size: 0.75rem;
    min-height: 36px;
  }

  .operation-add-btn:hover {
    background: var(--hover-bg, #313244);
    border-color: #3b82f6;
    transform: translateY(-1px);
  }

  .operation-add-btn:active {
    transform: translateY(0);
  }

  .operation-icon {
    font-size: 1rem;
  }

  .operation-label {
    flex: 1;
    text-align: left;
    margin-left: 8px;
    font-weight: 500;
  }

  .operation-add-btn .fa-plus {
    color: #22c55e;
    font-size: 0.7rem;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted, #a6adc8);
    gap: 12px;
    padding: 24px;
  }

  .empty-state p {
    margin: 0;
    font-size: 0.9rem;
  }
</style>
