<script lang="ts">
  import { invokeWithPerf } from '$lib/state/performance';
  import type { OperationDef, OperationId } from '$lib/state/operation';

  // Props
  export let selectedOperation: OperationDef | null = null;
  export let selectedOperationId: OperationId | null = null;
  export let operationParams: Record<string, any> = {};
  export let validateParameters: () => boolean;

  // Test result state
  let testResult: { type: 'success' | 'error'; message: string } | null = null;
  let isTestingOperation = false;

  // Scheduler test result state
  let schedulerTestResult: { type: 'success' | 'error'; message: string } | null = null;
  let isTestingScheduler = false;

  // Parameter test state
  let paramTestResult: { type: 'success' | 'error'; message: string } | null = null;
  let isTestingWithParams = false;

  function getOperationType(operation: OperationDef): string {
    switch (operation.kind) {
      case 'pipeline':
        return 'master_pipeline';
      case 'combine':
        return 'combine';
      default:
        return operation.kind;
    }
  }

  async function handleTestOperation() {
    if (!selectedOperationId || !selectedOperation) return;

    isTestingOperation = true;
    testResult = null;

    try {
      const result = await invokeWithPerf<string>('test_operation', {
        operationName: selectedOperation.kind,
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

  async function handleTestWithParams() {
    if (!selectedOperationName || !selectedOperation) return;

    // Validate parameters using schema-based validation from parent
    if (!validateParameters()) {
      return;
    }

    isTestingWithParams = true;
    paramTestResult = null;

    try {
      const operationType = getOperationType(selectedOperation);

      const result = await invokeWithPerf<string>('test_operation_with_params', {
        operationName: operationType,
        params: {
          parameters: operationParams,
          operation_type: operationType,
        },
      });

      console.log('Parameter test result:', result);
      paramTestResult = { type: 'success', message: result };
    } catch (error) {
      console.log(error);
      console.error('Error testing operation with params:', error);
      paramTestResult = { type: 'error', message: JSON.stringify(error) };
    } finally {
      isTestingWithParams = false;
    }
  }

  async function handleTestScheduler() {
    isTestingScheduler = true;
    schedulerTestResult = null;

    try {
      const result = await invokeWithPerf<string>('test_scheduler');
      console.log('Scheduler test result:', result);
      schedulerTestResult = { type: 'success', message: result };
    } catch (error) {
      console.log(error);
      console.error('Error testing scheduler:', error);
      schedulerTestResult = { type: 'error', message: JSON.stringify(error) };
    } finally {
      isTestingScheduler = false;
    }
  }

  async function handleOpenArtifactsFolder() {
    try {
      const artifactsPath = await invokeWithPerf<string>('get_artifacts_directory');
      console.log('Opening artifacts folder:', artifactsPath);

      // Call open_in_explorer with the artifacts directory path
      await invokeWithPerf('open_in_explorer', {
        fileToOpen: artifactsPath,
      });
    } catch (error) {
      console.error('Error opening artifacts folder:', error);
    }
  }
</script>

<div class="debug-panel">
  <!-- Compact button row -->
  <div class="button-row">
    <button
      class="test-btn"
      onclick={handleTestOperation}
      disabled={isTestingOperation || !selectedOperation}
      title="Test operation"
    >
      {#if isTestingOperation}
        <i class="fa fa-spinner fa-spin"></i>
        Testing...
      {:else}
        <i class="fa fa-play"></i>
        Test
      {/if}
    </button>

    <button
      class="test-with-params-btn"
      onclick={handleTestWithParams}
      disabled={isTestingWithParams || !selectedOperation}
      title="Test with parameters"
    >
      {#if isTestingWithParams}
        <i class="fa fa-spinner fa-spin"></i>
        Testing...
      {:else}
        <i class="fa fa-flask"></i>
        Params
      {/if}
    </button>

    <button
      class="test-scheduler-btn"
      onclick={handleTestScheduler}
      disabled={isTestingScheduler}
      title="Test scheduler"
    >
      {#if isTestingScheduler}
        <i class="fa fa-spinner fa-spin"></i>
        Testing...
      {:else}
        <i class="fa fa-cogs"></i>
        Scheduler
      {/if}
    </button>

    <button
      class="open-artifacts-btn"
      onclick={handleOpenArtifactsFolder}
      title="Artifacts folder"
      aria-label="Open artifacts folder"
    >
      <i class="fa fa-folder-open"></i>
      Artifacts
    </button>
  </div>

  <!-- Test Results -->
  {#if testResult}
    <div class="test-result {testResult.type}">
      <div class="result-header">
        <i class="fa {testResult.type === 'success' ? 'fa-check-circle' : 'fa-exclamation-circle'}"
        ></i>
        <span>{testResult.type === 'success' ? 'Success' : 'Failed'}</span>
      </div>
      <div class="result-message">{testResult.message}</div>
    </div>
  {/if}

  {#if paramTestResult}
    <div class="test-result {paramTestResult.type}">
      <div class="result-header">
        <i
          class="fa {paramTestResult.type === 'success'
            ? 'fa-check-circle'
            : 'fa-exclamation-circle'}"
        ></i>
        <span>Parameters {paramTestResult.type === 'success' ? 'Success' : 'Failed'}</span>
      </div>
      <div class="result-message">{paramTestResult.message}</div>
    </div>
  {/if}

  {#if schedulerTestResult}
    <div class="test-result {schedulerTestResult.type}">
      <div class="result-header">
        <i
          class="fa {schedulerTestResult.type === 'success'
            ? 'fa-check-circle'
            : 'fa-exclamation-circle'}"
        ></i>
        <span>Scheduler {schedulerTestResult.type === 'success' ? 'Success' : 'Failed'}</span>
      </div>
      <div class="result-message">{schedulerTestResult.message}</div>
    </div>
  {/if}
</div>

<style>
  .debug-panel {
    padding: 8px;
    background: #262637;
    border: 1px solid #45475a;
    border-radius: 4px;
    margin-top: 8px;
  }

  .button-row {
    display: flex;
    gap: 4px;
    margin-bottom: 8px;
  }

  .test-btn,
  .test-params-btn,
  .test-scheduler-btn,
  .open-artifacts-btn {
    flex: 1;
    padding: 4px 8px;
    border: none;
    border-radius: 3px;
    cursor: pointer;
    font-size: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 3px;
    transition: all 0.2s;
    min-height: 24px;
  }

  .test-btn {
    background: #4caf50;
    color: white;
  }

  .test-btn:hover {
    background: #45a049;
  }

  .test-btn:disabled {
    background: #6c7086;
    cursor: not-allowed;
  }

  .test-params-btn {
    background: #2196f3;
    color: white;
  }

  .test-params-btn:hover {
    background: #1976d2;
  }

  .test-params-btn:disabled {
    background: #6c7086;
    cursor: not-allowed;
  }

  .test-scheduler-btn {
    background: #ff9800;
    color: white;
  }

  .test-scheduler-btn:hover {
    background: #f57c00;
  }

  .test-scheduler-btn:disabled {
    background: #6c7086;
    cursor: not-allowed;
  }

  .open-artifacts-btn {
    background: #673ab7;
    color: white;
  }

  .open-artifacts-btn:hover {
    background: #5e35b1;
  }

  .parameter-editor {
    background: #313244;
    border: 1px solid #45475a;
    border-radius: 4px;
    padding: 8px;
    margin-bottom: 8px;
  }

  .parameter-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
    padding-bottom: 4px;
    border-bottom: 1px solid #45475a;
  }

  .param-title {
    font-size: 11px;
    font-weight: 600;
    color: #cdd6f4;
  }

  .reset-btn {
    background: transparent;
    border: 1px solid #6c7086;
    color: #6c7086;
    padding: 2px 4px;
    border-radius: 2px;
    cursor: pointer;
    font-size: 10px;
    transition: all 0.2s;
  }

  .reset-btn:hover {
    background: #6c7086;
    color: #1e1e2e;
  }

  .parameter-grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 6px;
    margin-bottom: 8px;
  }

  .parameter-group {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .parameter-group label {
    font-size: 9px;
    font-weight: 600;
    color: #a6adc8;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .parameter-group input[type='number'],
  .parameter-group input[type='text'],
  .parameter-group select {
    background: #1e1e2e;
    border: 1px solid #45475a;
    color: #cdd6f4;
    padding: 2px 4px;
    border-radius: 2px;
    font-size: 10px;
    font-family: 'Fira Code', monospace;
    transition: border-color 0.2s;
    min-height: 20px;
  }

  .parameter-group input:focus,
  .parameter-group select:focus {
    outline: none;
    border-color: #74c0fc;
  }

  .parameter-group input.error,
  .parameter-group select.error {
    border-color: #f38ba8;
  }

  .param-error {
    font-size: 8px;
    color: #f38ba8;
  }

  .checkbox-group,
  .multiselect-group {
    margin-top: 2px;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 4px;
    cursor: pointer;
    font-size: 9px !important;
    color: #cdd6f4 !important;
    text-transform: none !important;
    letter-spacing: normal !important;
    font-weight: normal !important;
  }

  .checkbox-label input[type='checkbox'] {
    position: absolute;
    opacity: 0;
    cursor: pointer;
  }

  .checkmark {
    height: 10px;
    width: 10px;
    background-color: #1e1e2e;
    border: 1px solid #45475a;
    border-radius: 2px;
    position: relative;
    transition: all 0.2s;
  }

  .checkbox-label input:checked + .checkmark {
    background-color: #74c0fc;
    border-color: #74c0fc;
  }

  .checkbox-label input:checked + .checkmark:after {
    content: '';
    position: absolute;
    display: block;
    left: 2px;
    top: 0px;
    width: 2px;
    height: 4px;
    border: solid #1e1e2e;
    border-width: 0 1px 1px 0;
    transform: rotate(45deg);
  }

  .multiselect-group {
    background: #1e1e2e;
    border: 1px solid #45475a;
    border-radius: 2px;
    padding: 4px;
    max-height: 60px;
    overflow-y: auto;
  }

  .multiselect-group .checkbox-label {
    padding: 1px 0;
  }

  .no-schema {
    text-align: center;
    padding: 8px;
    background: #181825;
    border: 1px dashed #45475a;
    border-radius: 3px;
    color: #a6adc8;
  }

  .no-schema p {
    margin: 2px 0;
    font-size: 9px;
  }

  .no-schema strong {
    color: #f9e2af;
    font-family: 'Fira Code', monospace;
  }

  .test-with-params-btn {
    width: 100%;
    background: #9c27b0;
    color: white;
    padding: 4px 6px;
    border: none;
    border-radius: 2px;
    cursor: pointer;
    font-size: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 3px;
    transition: background 0.2s;
    min-height: 20px;
  }

  .test-with-params-btn:hover {
    background: #8e24aa;
  }

  .test-with-params-btn:disabled {
    background: #6c7086;
    cursor: not-allowed;
  }

  .test-result {
    padding: 6px;
    border-radius: 3px;
    margin-bottom: 6px;
    border: 1px solid;
    font-size: 12px;
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

  .result-header {
    display: flex;
    align-items: center;
    gap: 3px;
    font-weight: 600;
    font-size: 9px;
    margin-bottom: 3px;
  }

  .result-message {
    font-size: 12px;
    font-family: 'Fira Code', monospace;
    line-height: 1.2;
    word-break: break-word;
  }
</style>
