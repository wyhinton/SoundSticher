<script lang="ts">
  import { appState, setSelectedOperationName } from '$lib/state/state.svelte';
  import { deleteOperation, OperationInfoDictionary } from '$lib/state/operation';
  import type { OperationDef, CombineOperation, PipelineOperation } from '$lib/state/operation';
  import { invoke } from '@tauri-apps/api/core';

  // Use selected operation from global state
  $: selectedOperationName = $appState.uiSettings?.selectedOperationName || null;

  // Test result state
  let testResult: { type: 'success' | 'error'; message: string } | null = null;
  let isTestingOperation = false;

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
        // Cast to combine operation to access properties
        const combineOp = operation as CombineOperation;
        if ('outputPath' in combineOp && combineOp.outputPath)
          details.push({ label: 'Output Path', value: String(combineOp.outputPath) });
        if ('gapSeconds' in combineOp && combineOp.gapSeconds !== undefined)
          details.push({ label: 'Gap Seconds', value: String(combineOp.gapSeconds) });
        if ('format' in combineOp && combineOp.format)
          details.push({ label: 'Format', value: String(combineOp.format) });
        break;
      case 'pipeline':
        const pipelineOp = operation as PipelineOperation;
        if ('operations' in pipelineOp)
          details.push({ label: 'Operations', value: pipelineOp.operations.join(' → ') });
        break;
      default:
        // Add more operation types as needed
        break;
    }

    return details;
  }

  async function handleTestOperation() {
    if (!selectedOperationName) return;

    isTestingOperation = true;
    testResult = null;

    try {
      const result = await invoke<string>('test_operation', {
        operationName: selectedOperationName,
      });

      console.log('Operation test result:', result);
      testResult = { type: 'success', message: result };
    } catch (error) {
      console.log(error);
      console.error('Error testing operation:', error);
      testResult = { type: 'error', message: JSON.stringify(error) };
    } finally {
      isTestingOperation = false;
    }
  }

  // Function to add test operations for demonstration
  function addTestOperations() {
    import('$lib/state/operation').then(({ addTestOperations }) => {
      addTestOperations();
      console.log('Test operations added!');
    });
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
          <span class="info-label">Source:</span>
          <span class="info-value">{formatOperationSource(selectedOperation.source)}</span>
        </div>

        {#each formatOperationDetails(selectedOperation) as detail}
          <div class="info-section">
            <span class="info-label">{detail.label}:</span>
            <span class="info-value">{detail.value}</span>
          </div>
        {/each}
      </div>

      <button
        class="test-btn"
        onclick={handleTestOperation}
        disabled={isTestingOperation}
        title="Test operation"
        aria-label="Test operation"
      >
        {#if isTestingOperation}
          <i class="fa fa-spinner fa-spin"></i> Testing...
        {:else}
          <i class="fa fa-play"></i> Test Operation
        {/if}
      </button>

      {#if testResult}
        <div class="test-result {testResult.type}">
          <div class="test-result-header">
            <i
              class="fa {testResult.type === 'success'
                ? 'fa-check-circle'
                : 'fa-exclamation-circle'}"
            ></i>
            <span>{testResult.type === 'success' ? 'Test Successful' : 'Test Failed'}</span>
          </div>
          <div class="test-result-message">{testResult.message}</div>
        </div>
      {/if}
    </div>
  {:else}
    <div class="no-selection">
      <i class="fa fa-project-diagram fa-2x"></i>
      <p>No operation selected</p>
      <span class="hint">Click on an operation flow header to view details</span>

      <button class="add-test-btn" onclick={addTestOperations} title="Add test operations for demo">
        <i class="fa fa-plus"></i> Add Test Operations
      </button>
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

  .test-btn {
    background: #4caf50;
    color: white;
    padding: 8px 16px;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
    display: flex;
    align-items: center;
    gap: 4px;
    transition: background 0.2s;
  }

  .test-btn:hover {
    background: #45a049;
  }

  .test-btn:disabled {
    background: #6c7086;
    cursor: not-allowed;
  }

  .test-result {
    padding: 12px;
    border-radius: 4px;
    margin-top: 8px;
    border: 1px solid;
  }

  .test-result.success {
    background: rgba(76, 175, 80, 0.1);
    border-color: #4caf50;
    color: #4caf50;
  }

  .test-result.error {
    background: rgba(244, 67, 54, 0.1);
    border-color: #f44336;
    color: #f44336;
  }

  .test-result-header {
    display: flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
    font-size: 12px;
    margin-bottom: 6px;
  }

  .test-result-message {
    font-size: 11px;
    font-family: 'Fira Code', monospace;
    line-height: 1.4;
    word-break: break-word;
  }

  .add-test-btn {
    background: #2196f3;
    color: white;
    padding: 8px 16px;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    display: flex;
    align-items: center;
    gap: 4px;
    transition: background 0.2s;
    margin-top: 16px;
  }

  .add-test-btn:hover {
    background: #1976d2;
  }
</style>
