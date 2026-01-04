<script lang="ts">
  import { appState, setSelectedOperationName } from '$lib/state/state.svelte';
  import { deleteOperation, OperationInfoDictionary } from '$lib/state/operation';
  import type { OperationDef } from '$lib/state/operation';

  // Use selected operation from global state
  $: selectedOperationName = $appState.uiSettings?.selectedOperationName || null;

  // Derived data about the selected operation
  $: selectedOperation =
    selectedOperationName && $appState.operations?.defs
      ? $appState.operations.defs[selectedOperationName]
      : null;

  $: operationInfo = selectedOperation ? OperationInfoDictionary[selectedOperation.kind] : null;

  function handleDeleteOperation() {
    if (selectedOperationName && confirm('Delete operation "' + selectedOperationName + '"?')) {
      deleteOperation(selectedOperationName);
      // Clear selection after deletion in global state
      setSelectedOperationName(null);
    }
  }

  function formatOperationSource(source: OperationDef['source']): string {
    switch (source.type) {
      case 'group':
        return 'Group: ' + source.groupRef;
      case 'files':
        return source.fileIds.length + ' specific files';
      case 'all':
        return 'All files';
      case 'active':
        return 'Active files';
      case 'section':
        return 'Section ' + source.sectionIndex;
      case 'previousOperation':
        return 'Output from: ' + source.operationRef;
      default:
        return 'Unknown source';
    }
  }

  function formatOperationDetails(operation: OperationDef): { label: string; value: string }[] {
    const details: { label: string; value: string }[] = [];

    switch (operation.kind) {
      case 'combine':
        if ('outputPath' in operation)
          details.push({ label: 'Output Path', value: operation.outputPath });
        if ('gapSeconds' in operation)
          details.push({ label: 'Gap Seconds', value: operation.gapSeconds.toString() });
        if ('format' in operation) details.push({ label: 'Format', value: operation.format });
        break;
      case 'pipeline':
        if ('operations' in operation)
          details.push({ label: 'Operations', value: operation.operations.join(' → ') });
        break;
      default:
        // Add more operation types as needed
        break;
    }

    return details;
  }
</script>

<div class="operations-panel">
  {#if selectedOperationName && selectedOperation && operationInfo}
    <div class="operation-details">
      <div class="operation-header">
        <div class="operation-title">
          <span class="operation-icon">{operationInfo.icon}</span>
          <div class="operation-text">
            <h4 class="operation-name">{selectedOperationName}</h4>
            <span class="operation-type">{operationInfo.label} ({operationInfo.category})</span>
          </div>
        </div>
        <button
          class="delete-btn"
          onclick={handleDeleteOperation}
          title="Delete operation"
          aria-label="Delete operation"
        >
          <i class="fa fa-trash"></i>
        </button>
      </div>

      <div class="operation-info">
        <div class="info-section">
          <label class="info-label">Source:</label>
          <span class="info-value">{formatOperationSource(selectedOperation.source)}</span>
        </div>

        {#each formatOperationDetails(selectedOperation) as detail}
          <div class="info-section">
            <label class="info-label">{detail.label}:</label>
            <span class="info-value">{detail.value}</span>
          </div>
        {/each}
      </div>
    </div>
  {:else}
    <div class="no-selection">
      <i class="fa fa-project-diagram fa-2x"></i>
      <p>No operation selected</p>
      <span class="hint">Click on an operation flow header to view details</span>
    </div>
  {/if}
</div>

<style>
  .operations-panel {
    padding: 12px;
    height: 100%;
    color: #cdd6f4;
  }

  .no-selection {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #a6adc8;
    text-align: center;
    gap: 8px;
  }

  .no-selection i {
    color: #585b70;
    margin-bottom: 4px;
  }

  .no-selection p {
    margin: 0;
    font-size: 14px;
    font-weight: 500;
  }

  .hint {
    font-size: 11px;
    color: #6c7086;
    font-style: italic;
  }

  .operation-details {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .operation-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 8px 0;
    border-bottom: 1px solid #45475a;
  }

  .operation-title {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    flex: 1;
  }

  .operation-icon {
    font-size: 18px;
    margin-top: 2px;
  }

  .operation-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .operation-name {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: #cdd6f4;
  }

  .operation-type {
    font-size: 11px;
    color: #a6adc8;
    text-transform: capitalize;
  }

  .delete-btn {
    background: transparent;
    border: 1px solid #f38ba8;
    color: #f38ba8;
    padding: 4px 8px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    transition: all 0.2s;
  }

  .delete-btn:hover {
    background: #f38ba8;
    color: #1e1e2e;
  }

  .operation-info {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .info-section {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .info-label {
    font-size: 11px;
    font-weight: 600;
    color: #a6adc8;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .info-value {
    font-size: 12px;
    color: #cdd6f4;
    padding: 4px 8px;
    background: #313244;
    border-radius: 4px;
    border: 1px solid #45475a;
    font-family: 'Fira Code', monospace;
  }
</style>
