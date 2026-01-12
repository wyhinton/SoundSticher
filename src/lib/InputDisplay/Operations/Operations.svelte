<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { appState, setSelectedOperationName } from '$lib/state/state.svelte';
  import { deleteOperation, OperationInfoDictionary } from '$lib/state/operation';
  import type { OperationDef, MergeOp, PipelineOp } from '$lib/state/operation';
  import OperationParamsDebugPanel from './OperationParamsDebugPanel.svelte';

  // Use selected operation from global state
  $: selectedOperationName = $appState.uiSettings?.selectedOperationName || null;

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

  // Derived data about the selected operation
  $: selectedOperation =
    selectedOperationName && $appState.operations?.defs
      ? $appState.operations.defs[selectedOperationName]
      : null;

  $: operationInfo = selectedOperation ? OperationInfoDictionary[selectedOperation.kind] : null;

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
      Object.entries(schema).forEach(([key, config]: [string, any]) => {
        operationParams[key] = config.default;
      });
    }
    paramErrors = {};
  }

  function getOperationType(operation: OperationDef): string {
    switch (operation.kind) {
      case 'pipeline':
        return 'master_pipeline';
      case 'merge':
        return 'merge';
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

    Object.entries(schema).forEach(([key, config]: [string, any]) => {
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

  function resetParameters() {
    if (selectedOperation) {
      initializeParameters();
    } else {
      operationParams = {};
      paramErrors = {};
    }
  }

  function handleDeleteOperation() {
    if (selectedOperationName && confirm('Delete operation "' + selectedOperationName + '"?')) {
      deleteOperation(selectedOperationName);
      // Clear selection after deletion in global state
      setSelectedOperationName(null);
    }
  }

  function formatOperationSource(sources: OperationDef['sources']): string[] {
    return sources.map(source => {
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
        case 'operation':
          return `From: ${source.operationRef}`;
        case 'previousOperation':
          return 'Output from: ' + source.operationRef;
        default:
          return 'Unknown source';
      }
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
          <span class="info-value"
            >{JSON.stringify(formatOperationSource(selectedOperation.sources))}</span
          >
        </div>
      </div>

      <!-- Editable Parameters Section -->
      <div class="parameters-section">
        <div class="parameters-header">
          <span class="parameters-title">Parameters</span>
          <button
            class="reset-params-btn"
            onclick={resetParameters}
            title="Reset parameters to defaults"
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
                  <label for={key} class="parameter-label">{config.label}</label>

                  {#if config.type === 'number'}
                    <input
                      id={key}
                      type="number"
                      bind:value={operationParams[key]}
                      min={config.min}
                      max={config.max}
                      step={config.step || 1}
                      class="parameter-input"
                      class:error={paramErrors[key]}
                    />
                  {:else if config.type === 'text'}
                    <input
                      id={key}
                      type="text"
                      bind:value={operationParams[key]}
                      class="parameter-input"
                      class:error={paramErrors[key]}
                    />
                  {:else if config.type === 'select'}
                    <select
                      id={key}
                      bind:value={operationParams[key]}
                      class="parameter-select"
                      class:error={paramErrors[key]}
                    >
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
                              const target = e.target as HTMLInputElement;
                              if (!operationParams[key]) operationParams[key] = [];
                              if (target.checked) {
                                if (!operationParams[key].includes(option)) {
                                  operationParams[key] = [...operationParams[key], option];
                                }
                              } else {
                                operationParams[key] = operationParams[key].filter(
                                  (item: any) => item !== option
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
                <p>Parameters can be configured through the debug panel below.</p>
              </div>
            {/if}
          {/if}
        </div>
      </div>

      <!-- Debug Panel Component -->
      <OperationParamsDebugPanel
        {selectedOperation}
        {selectedOperationName}
        {operationParams}
        {validateParameters}
      />
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

  .parameters-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .parameters-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 0 4px 0;
    border-bottom: 1px solid #45475a;
  }

  .parameters-title {
    font-size: 12px;
    font-weight: 600;
    color: #cdd6f4;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .reset-params-btn {
    background: transparent;
    border: 1px solid #6c7086;
    color: #6c7086;
    padding: 3px 6px;
    border-radius: 3px;
    cursor: pointer;
    font-size: 10px;
    display: flex;
    align-items: center;
    gap: 3px;
    transition: all 0.2s;
  }

  .reset-params-btn:hover {
    background: #6c7086;
    color: #1e1e2e;
  }

  .parameter-grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 12px;
  }

  .parameter-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .parameter-label {
    font-size: 11px;
    font-weight: 600;
    color: #a6adc8;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .parameter-input,
  .parameter-select {
    background: #1e1e2e;
    border: 1px solid #45475a;
    color: #cdd6f4;
    padding: 6px 8px;
    border-radius: 4px;
    font-size: 12px;
    font-family: 'Fira Code', monospace;
    transition: border-color 0.2s;
    min-height: 32px;
  }

  .parameter-input:focus,
  .parameter-select:focus {
    outline: none;
    border-color: #74c0fc;
    box-shadow: 0 0 0 1px rgba(116, 192, 252, 0.1);
  }

  .parameter-input.error,
  .parameter-select.error {
    border-color: #f38ba8;
    box-shadow: 0 0 0 1px rgba(243, 139, 168, 0.1);
  }

  .param-error {
    font-size: 10px;
    color: #f38ba8;
    margin-top: 2px;
  }

  .checkbox-group,
  .multiselect-group {
    margin-top: 4px;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    font-size: 11px !important;
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
    height: 14px;
    width: 14px;
    background-color: #1e1e2e;
    border: 1px solid #45475a;
    border-radius: 3px;
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
    left: 4px;
    top: 1px;
    width: 4px;
    height: 7px;
    border: solid #1e1e2e;
    border-width: 0 2px 2px 0;
    transform: rotate(45deg);
  }

  .multiselect-group {
    background: #1e1e2e;
    border: 1px solid #45475a;
    border-radius: 4px;
    padding: 8px;
    max-height: 120px;
    overflow-y: auto;
  }

  .multiselect-group .checkbox-label {
    padding: 2px 0;
  }

  .no-schema {
    text-align: center;
    padding: 16px 12px;
    background: #181825;
    border: 1px dashed #45475a;
    border-radius: 4px;
    color: #a6adc8;
  }

  .no-schema p {
    margin: 4px 0;
    font-size: 12px;
  }

  .no-schema strong {
    color: #f9e2af;
    font-family: 'Fira Code', monospace;
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
    padding: 8px 16px;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
    display: flex;
    align-items: center;
    gap: 4px;
    transition: background 0.2s;
    margin-top: 8px;
  }

  .test-params-btn:hover {
    background: #1976d2;
  }

  .test-scheduler-btn {
    background: #ff9800;
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
    margin-top: 8px;
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
    padding: 8px 16px;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
    display: flex;
    align-items: center;
    gap: 4px;
    transition: background 0.2s;
    margin-top: 8px;
  }

  .open-artifacts-btn:hover {
    background: #5e35b1;
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
