<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { OperationDef } from '$lib/state/operation';

  // Props
  export let selectedOperation: OperationDef | null = null;
  export let selectedOperationName: string | null = null;

  // Test result state
  let testResult: { type: 'success' | 'error'; message: string } | null = null;
  let isTestingOperation = false;

  // Scheduler test result state
  let schedulerTestResult: { type: 'success' | 'error'; message: string } | null = null;
  let isTestingScheduler = false;

  // Parameter test state
  let paramTestResult: { type: 'success' | 'error'; message: string } | null = null;
  let isTestingWithParams = false;
  let showParameterEditor = false;

  // Operation parameters with validation - will be dynamic based on operation type
  let operationParams: Record<string, any> = {};
  let paramErrors: Record<string, string> = {};

  // Parameter schemas for different operation types
  const parameterSchemas: Record<string, any> = {
    combine: {
      crossfade_ms: {
        type: 'number',
        min: 0,
        max: 5000,
        default: 100,
        label: 'Crossfade (ms)',
        step: 10,
      },
      gap_seconds: {
        type: 'number',
        min: 0,
        max: 60,
        default: 0,
        step: 0.1,
        label: 'Gap (seconds)',
      },
      normalize: { type: 'boolean', default: false, label: 'Normalize Audio' },
      sample_rate: {
        type: 'select',
        options: [8000, 11025, 16000, 22050, 44100, 48000, 88200, 96000],
        default: 44100,
        label: 'Sample Rate (Hz)',
      },
      bit_depth: {
        type: 'select',
        options: [8, 16, 24, 32],
        default: 16,
        label: 'Bit Depth',
      },
      output_format: {
        type: 'select',
        options: ['wav', 'mp3', 'flac', 'ogg', 'm4a'],
        default: 'wav',
        label: 'Output Format',
      },
    },
    master_pipeline: {
      operations: {
        type: 'multiselect',
        options: ['combine', 'normalize', 'export', 'merge', 'compress'],
        default: ['combine', 'normalize'],
        label: 'Pipeline Steps',
      },
      parallel_execution: { type: 'boolean', default: false, label: 'Parallel Execution' },
      batch_size: { type: 'number', min: 1, max: 100, default: 10, label: 'Batch Size', step: 1 },
    },
    normalize: {
      target_db: { type: 'number', min: -60, max: 0, default: -12, step: 0.1, label: 'Target dB' },
      preserve_peaks: { type: 'boolean', default: true, label: 'Preserve Peaks' },
      target_lufs: {
        type: 'number',
        min: -40,
        max: -6,
        default: -23,
        step: 0.1,
        label: 'Target LUFS (optional)',
      },
      true_peak_limit: {
        type: 'number',
        min: -6,
        max: 0,
        default: -1,
        step: 0.1,
        label: 'True Peak Limit (dB)',
      },
    },
    export: {
      format: {
        type: 'select',
        options: ['wav', 'mp3', 'flac', 'ogg', 'm4a', 'aac'],
        default: 'wav',
        label: 'Export Format',
      },
      quality: {
        type: 'select',
        options: ['low', 'medium', 'high', 'lossless'],
        default: 'high',
        label: 'Quality',
      },
      output_path: { type: 'text', default: './output', label: 'Output Path' },
      bit_rate: {
        type: 'number',
        min: 64,
        max: 2048,
        default: 320,
        step: 32,
        label: 'Bit Rate (kbps)',
      },
      sample_rate: {
        type: 'select',
        options: [8000, 11025, 16000, 22050, 44100, 48000, 88200, 96000],
        default: 44100,
        label: 'Sample Rate (Hz)',
      },
      normalize_before_export: {
        type: 'boolean',
        default: false,
        label: 'Normalize Before Export',
      },
    },
  };

  // Initialize parameters based on selected operation
  $: if (selectedOperation) {
    initializeParameters();
  }

  function initializeParameters() {
    if (!selectedOperation) return;

    const operationType = getOperationType(selectedOperation);

    const schema = parameterSchemas[operationType];
    if (schema) {
      operationParams = {};
      Object.entries(schema).forEach(([key, config]) => {
        operationParams[key] = config.default;
      });
    }
    paramErrors = {};
  }

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

  function validateParameters(): boolean {
    if (!selectedOperation) return false;

    const operationType = getOperationType(selectedOperation);

    const schema = parameterSchemas[operationType];
    if (!schema) return true; // No validation for unknown types

    paramErrors = {};
    let hasErrors = false;

    Object.entries(schema).forEach(([key, config]) => {
      const value = operationParams[key];

      if (config.type === 'number') {
        if (value != null) {
          if (config.min != null && value < config.min) {
            paramErrors[key] = `Must be at least ${config.min}`;
            hasErrors = true;
          }
          if (config.max != null && value > config.max) {
            paramErrors[key] = `Must be at most ${config.max}`;
            hasErrors = true;
          }
        }
      }

      if (config.type === 'select' && config.options) {
        if (!config.options.includes(value)) {
          paramErrors[key] = `Invalid option`;
          hasErrors = true;
        }
      }

      if (config.type === 'text') {
        if (value != null && typeof value === 'string' && value.trim().length === 0) {
          paramErrors[key] = `This field is required`;
          hasErrors = true;
        }
      }
    });

    return !hasErrors;
  }

  async function handleTestOperation() {
    if (!selectedOperationName || !selectedOperation) return;

    isTestingOperation = true;
    testResult = null;

    try {
      const result = await invoke<string>('test_operation', {
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

    // Validate parameters using schema-based validation
    if (!validateParameters()) {
      return;
    }

    isTestingWithParams = true;
    paramTestResult = null;

    try {
      const operationType = getOperationType(selectedOperation);

      const result = await invoke<string>('test_operation_with_params', {
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

  function resetParameters() {
    if (selectedOperation) {
      initializeParameters();
    } else {
      operationParams = {};
      paramErrors = {};
    }
  }

  async function handleTestScheduler() {
    isTestingScheduler = true;
    schedulerTestResult = null;

    try {
      const result = await invoke<string>('test_scheduler');
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
      const artifactsPath = await invoke<string>('get_artifacts_directory');
      console.log('Opening artifacts folder:', artifactsPath);

      // Call open_in_explorer with the artifacts directory path
      await invoke('open_in_explorer', {
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
      {:else}
        <i class="fa fa-play"></i>
      {/if}
    </button>

    <button
      class="test-params-btn"
      onclick={() => (showParameterEditor = !showParameterEditor)}
      disabled={!selectedOperation}
      title="Parameters"
      aria-label="Toggle parameter editor"
    >
      <i class="fa {showParameterEditor ? 'fa-eye-slash' : 'fa-cog'}"></i>
    </button>

    <button
      class="test-scheduler-btn"
      onclick={handleTestScheduler}
      disabled={isTestingScheduler}
      title="Test scheduler"
    >
      {#if isTestingScheduler}
        <i class="fa fa-spinner fa-spin"></i>
      {:else}
        <i class="fa fa-cogs"></i>
      {/if}
    </button>

    <button
      class="open-artifacts-btn"
      onclick={handleOpenArtifactsFolder}
      title="Artifacts folder"
      aria-label="Open artifacts folder"
    >
      <i class="fa fa-folder-open"></i>
    </button>
  </div>

  <!-- Parameter Editor -->
  {#if showParameterEditor}
    <div class="parameter-editor">
      <div class="parameter-header">
        <span class="param-title">Parameters</span>
        <button
          class="reset-btn"
          onclick={resetParameters}
          title="Reset"
          aria-label="Reset parameters"
        >
          <i class="fa fa-undo"></i>
        </button>
      </div>

      <div class="parameter-grid">
        {#if selectedOperation}
          {@const operationType = getOperationType(selectedOperation)}
          {@const schema = parameterSchemas[operationType]}
          {#if schema}
            {#each Object.entries(schema) as [key, config]}
              <div class="parameter-group">
                <label for={key}>{config.label}</label>

                {#if config.type === 'number'}
                  <input
                    id={key}
                    type="number"
                    bind:value={operationParams[key]}
                    min={config.min}
                    max={config.max}
                    step={config.step || 1}
                    class:error={paramErrors[key]}
                  />
                {:else if config.type === 'text'}
                  <input
                    id={key}
                    type="text"
                    bind:value={operationParams[key]}
                    class:error={paramErrors[key]}
                  />
                {:else if config.type === 'select'}
                  <select id={key} bind:value={operationParams[key]} class:error={paramErrors[key]}>
                    {#each config.options as option}
                      <option value={option}>
                        {typeof option === 'string'
                          ? option.toUpperCase()
                          : `${option}${config.label.includes('Rate') ? ' Hz' : config.label.includes('Depth') ? '-bit' : ''}`}
                      </option>
                    {/each}
                  </select>
                {:else if config.type === 'boolean'}
                  <div class="checkbox-group">
                    <label class="checkbox-label">
                      <input type="checkbox" bind:checked={operationParams[key]} />
                      <span class="checkmark"></span>
                      {config.label}
                    </label>
                  </div>
                {:else if config.type === 'multiselect'}
                  <div class="multiselect-group">
                    {#each config.options as option}
                      <label class="checkbox-label">
                        <input
                          type="checkbox"
                          checked={operationParams[key] && operationParams[key].includes(option)}
                          onchange={e => {
                            if (!operationParams[key]) operationParams[key] = [];
                            if (e.target.checked) {
                              if (!operationParams[key].includes(option)) {
                                operationParams[key] = [...operationParams[key], option];
                              }
                            } else {
                              operationParams[key] = operationParams[key].filter(
                                item => item !== option
                              );
                            }
                          }}
                        />
                        <span class="checkmark"></span>
                        {option}
                      </label>
                    {/each}
                  </div>
                {/if}

                {#if paramErrors[key]}
                  <span class="param-error">{paramErrors[key]}</span>
                {/if}
              </div>
            {/each}
          {:else}
            <div class="no-schema">
              <p>
                No parameter schema available for operation type: <strong>{operationType}</strong>
              </p>
              <p>You can still test this operation with generic parameters.</p>
            </div>
          {/if}
        {/if}
      </div>

      <button
        class="test-with-params-btn"
        onclick={handleTestWithParams}
        disabled={isTestingWithParams || !selectedOperation}
        title="Test with parameters"
      >
        {#if isTestingWithParams}
          <i class="fa fa-spinner fa-spin"></i>
        {:else}
          <i class="fa fa-flask"></i>
        {/if}
      </button>
    </div>
  {/if}

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
    font-size: 11px;
    display: flex;
    align-items: center;
    justify-content: center;
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
