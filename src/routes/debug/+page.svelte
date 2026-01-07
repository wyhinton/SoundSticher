<script lang="ts">
  import {
    exportAudio,
    invokeWithPerf,
    performanceStore,
    resetPerformance,
    type PerformanceMetric,
  } from '$lib/state/performance';
  import { addNewFolderOnDrop, positionStore } from '$lib/state/position';
  import {
    addSource,
    appState,
    hoveredSourceItem,
    resetAppState,
    setDebugActiveTab,
    currentOperationSections,
    setSvgPathDisplayMode,
  } from '$lib/state/state.svelte';
  import clipboard from 'tauri-plugin-clipboard-api';
  import { derived, get } from 'svelte/store';
  import { toSource } from '$lib/utils/format';
  import { examples } from '$lib/utils/examples';
  import { onDestroy, onMount } from 'svelte';
  import { Channel, invoke } from '@tauri-apps/api/core';
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import type { CombineAudioEvent, ExportAudioEvent } from '$lib/state/events';
  import { exportState } from '$lib/state/export';
  import TabContainer from '$lib/components/TabContainer.svelte';
  import PrismWrapper from '$lib/components/PrismWrapper.svelte';
  import LoggingControls from '$lib/components/LoggingControls.svelte';
  import {
    initializeBackendLogListener,
    updateBackendLoggingConfig,
    loggingState,
  } from '$lib/state/logging';

  // Helper function to process svg_path properties based on display mode
  function processSvgPaths(obj: any, mode: 'full' | 'trim' | 'hide', maxLength: number = 100): any {
    if (obj === null || obj === undefined) {
      return obj;
    }

    if (Array.isArray(obj)) {
      return obj.map(item => processSvgPaths(item, mode, maxLength));
    }

    if (typeof obj === 'object') {
      const result: any = {};
      for (const [key, value] of Object.entries(obj)) {
        if (key === 'svg_path' || key === 'svgPath') {
          // Process svg path properties based on mode
          if (mode === 'hide') {
            result[key] = '[SVG Path Hidden]';
          } else if (mode === 'trim' && typeof value === 'string' && value.length > maxLength) {
            result[key] = value.substring(0, maxLength) + `... (${value.length} chars total)`;
          } else {
            // mode === 'full' or value is short enough
            result[key] = value;
          }
        } else {
          // Recursively process other properties
          result[key] = processSvgPaths(value, mode, maxLength);
        }
      }
      return result;
    }

    return obj;
  }

  // Reactive derived state for simplified display
  $: svgDisplayMode = $appState.uiSettings?.svgPathDisplayMode || 'trim';
  $: forPrint = processSvgPaths(
    {
      ...$appState,
      sections: $appState.sections.map(s => ({
        folderPath: s.folderPath,
        files: s.files.length,
        // files: s.files.length,
      })),
      currentOperationSections: $currentOperationSections.map(s => ({
        folderPath: s.folderPath,
        files: s.files.length,
      })),
    },
    svgDisplayMode
  );

  $: t = {
    x: JSON.stringify($positionStore),
  };

  let seconds = 0;
  let interval: number;

  onMount(() => {
    interval = setInterval(() => {
      seconds += 50;
    }, 50);

    // Initialize backend log listener
    initializeBackendLogListener();

    // Update backend logging config based on current settings
    updateBackendLoggingConfig($loggingState);

    // Cleanup when component is destroyed
    onDestroy(() => {
      clearInterval(interval);
    });
  });

  async function copyStateToClipboard() {
    return await clipboard.writeText(toSource(get(appState)));
  }

  const applyExampleState = (k: string) => {
    const exampleState = examples[k];
    if (exampleState) {
      appState.set(exampleState);
    }
  };

  function test_async() {
    invokeWithPerf('test_async');
  }

  const openAudioFolder = () => {
    invokeWithPerf('open_in_explorer', {
      fileToOpen: 'C:\\Users\\Primary User\\Desktop\\AUDIO',
    });
  };
  const testExport = () => {
    const s = get(exportState);
    console.log(s);
    if (s.settings) {
      exportAudio(
        s.settings,
        `C:\\Users\\Primary User\\Desktop\\AUDIO\\test_audio2.${s.settings.format.toLowerCase()}`
      );
    }
  };
  const sortedPerformance = derived(performanceStore, $store => {
    return Object.entries($store).sort((a, b) => {
      const lastA = a[1][a[1].length - 1] ?? 0;
      const lastB = b[1][b[1].length - 1] ?? 0;
      return lastB.timeStamp - lastA.timeStamp;
    });
  });

  interface AppStateDebug {
    audio_files: { [key: string]: any };
    combined_audio: string;
    buffering_samples: boolean;
    svg_path: string;
    cancel_token: number;
    combine_process: number;
  }

  let appStateDebug: undefined | AppStateDebug = undefined;

  const addTwoSections = () => {
    addSource('C:\\Users\\Primary User\\Desktop\\AUDIO\\FREESOUNDS\\37427__dbs_sounds__foley');
    setTimeout(() => {
      addSource('C:\\Users\\Primary User\\Desktop\\AUDIO\\FREESOUNDS\\WOMB_VOX');
    }, 100);
  };

  const combineTest = () => {
    const onCombineAudioEvent = new Channel<CombineAudioEvent>();

    onCombineAudioEvent.onmessage = message => {
      if (message.event === 'started') {
        appState.update(state => {
          state.isCombiningFile = true;
          state.combinedFileLength = message.data.duration;
          return state;
        });
      }
      if (message.event === 'progress') {
        appState.update(s => {
          s.combinedFile = { svgPath: message.data.svgPath };
          return s;
        });
      }
      if (message.event === 'finished') {
        console.log(message);
        appState.update(s => {
          s.isCombiningFile = false;
          s.combinedFile = { svgPath: message.data.svgPath };
          return s;
        });
        console.log(message.event);
      }
    };

    invokeWithPerf('combine_all_cached_samples', {
      onEvent: onCombineAudioEvent,
    });
  };

  let intervalId: number;
  let refreshBackendState = false; // Toggle for auto-refresh
  let isFetching = false;

  // Function to fetch backend state
  async function fetchBackendState() {
    if (isFetching) return;
    isFetching = true;

    try {
      let zDebug = await invokeWithPerf<AppStateDebug>('get_app_state');

      if (zDebug.ok) {
        appStateDebug = zDebug.value;
      }
    } catch (err) {
      console.error('Failed to fetch app state', err);
    } finally {
      isFetching = false;
    }
  }

  // Reactive statement to handle auto-refresh toggle
  $: {
    if (refreshBackendState) {
      // Start interval when toggle is enabled
      intervalId = setInterval(fetchBackendState, 1000);
    } else {
      // Clear interval when toggle is disabled
      if (intervalId) {
        clearInterval(intervalId);
      }
    }
  }

  onMount(() => {
    // Initial fetch
    fetchBackendState();
  });

  onDestroy(() => {
    clearInterval(intervalId);
  });

  let selectedKey = Object.keys(examples)[0]; // default selection

  // Tab configuration
  const tabs = [
    { id: 'frontend', label: 'Frontend State', icon: 'fa-code' },
    { id: 'backend', label: 'Backend State', icon: 'fa-server' },
    { id: 'performance', label: 'Performance', icon: 'fa-chart-line' },
    { id: 'export', label: 'Export State', icon: 'fa-download' },
    { id: 'logging', label: 'Logging', icon: 'fa-terminal' },
    { id: 'debug', label: 'Debug Info', icon: 'fa-bug' },
  ];

  function handleTabChange(tabId: string) {
    setDebugActiveTab(tabId);
  }

  const resetMainWindow = async () => {
    try {
      // Get reference to the main window by its label
      const mainWindow = await WebviewWindow.getByLabel('main');
      if (!mainWindow) {
        console.error('Main window not found');
        return;
      }
      // Close the main window
      await mainWindow.close();

      // Create a new main window with the same properties
      const newWindow = new WebviewWindow('main', {
        url: '/',
        title: 'Sound Stitch',
        width: 1200,
        height: 800,
        center: true,
        resizable: true,
        decorations: true,
      });

      // Wait for the new window to be ready
      await newWindow.once('tauri://created', () => {
        console.log('New main window created');
      });
    } catch (error) {
      console.error('Failed to reset main window:', error);
    }
  };
</script>

{#snippet actionButton(
  onClick: () => void,
  icon: string,
  text: string,
  disabled: boolean = false,
  variant:
    | 'default'
    | 'primary'
    | 'secondary'
    | 'danger'
    | 'warning'
    | 'success'
    | 'info' = 'default'
)}
  <button on:click={onClick} class="btn btn-sm btn-{variant}" {disabled}>
    <i class="me-1 fa {icon}"></i>{text}
  </button>
{/snippet}

<div>
  <!-- Main Controls -->
  <div class="controls-section">
    <div class="button-group">
      <span class="group-label">State Management</span>
      {@render actionButton(
        () => resetAppState(),
        'fa-arrows-spin',
        'Reset AppState',
        false,
        'danger'
      )}
      {@render actionButton(
        () => resetMainWindow(),
        'fa-window-restore',
        'Reset Main Window',
        false,
        'warning'
      )}
    </div>

    <div class="button-group">
      <span class="group-label">Logging</span>
      {@render actionButton(
        () => console.log($appState),
        'fa-arrows-spin',
        'Log AppState',
        false,
        'info'
      )}
      {@render actionButton(
        () => console.log($appState.sections),
        'fa-arrows-spin',
        'Log Sections',
        false,
        'info'
      )}
      {@render actionButton(
        () => copyStateToClipboard(),
        'fa-clipboard',
        'Copy to Clipboard',
        false,
        'secondary'
      )}
    </div>

    <div class="button-group">
      <span class="group-label">Backend</span>
      <div class="backend-controls">
        <label class="toggle-label">
          <input type="checkbox" bind:checked={refreshBackendState} />
          <i class="fa fa-sync-alt"></i> Auto-refresh
        </label>
        {#if !refreshBackendState}
          {@render actionButton(
            fetchBackendState,
            isFetching ? 'fa-spinner fa-spin' : 'fa-download',
            isFetching ? 'Fetching...' : 'Fetch State',
            isFetching,
            'primary'
          )}
        {/if}
      </div>
    </div>

    <div class="button-group">
      <span class="group-label">Testing</span>
      {@render actionButton(() => test_async(), 'fa-play', 'Test Async', false, 'success')}
      {@render actionButton(() => testExport(), 'fa-download', 'Test Export', false, 'success')}
      {@render actionButton(
        () => openAudioFolder(),
        'fa-folder-open',
        'Open Audio Folder',
        false,
        'secondary'
      )}
    </div>

    <div class="button-group">
      <span class="group-label">Examples</span>
      <select bind:value={selectedKey} class="example-select">
        {#each Object.keys(examples) as key}
          <option value={key}>{key}</option>
        {/each}
      </select>
      {@render actionButton(
        () => selectedKey && applyExampleState(selectedKey),
        'fa-cog',
        'Apply Example',
        false,
        'warning'
      )}
      {@render actionButton(() => addTwoSections(), 'fa-plus', 'Add Sections', false, 'secondary')}
      {@render actionButton(() => combineTest(), 'fa-mix', 'Combine Test', false, 'primary')}
    </div>
  </div>

  <!-- Tab Container -->
  <TabContainer
    activeTab={$appState.uiSettings?.debugActiveTab || 'frontend'}
    {tabs}
    onTabChange={handleTabChange}
    contentHeight={600}
  >
    <!-- Frontend State Tab -->
    <div slot="frontend">
      <div class="d-flex justify-content-between align-items-center mb-3">
        <h3>Frontend State</h3>
        <div class="svg-path-controls">
          <label for="svg-path-select" class="control-label">SVG Path Display:</label>
          <select
            id="svg-path-select"
            bind:value={svgDisplayMode}
            on:change={e =>
              setSvgPathDisplayMode(
                (e.target as HTMLSelectElement).value as 'full' | 'trim' | 'hide'
              )}
            class="svg-path-select"
          >
            <option value="full">Show Full</option>
            <option value="trim">Trim</option>
            <option value="hide">Hide</option>
          </select>
        </div>
      </div>
      <PrismWrapper data={forPrint} />
    </div>

    <!-- Backend State Tab -->
    <div slot="backend">
      <h3>Backend State</h3>
      <PrismWrapper data={appStateDebug || {}} />
    </div>

    <!-- Performance Tab -->
    <div slot="performance">
      <div class="d-flex justify-content-between">
        <h3>Performance Metrics</h3>
        <div class="performance-controls">
          {@render actionButton(() => resetPerformance(), 'fa-trash', 'Reset Performance')}
        </div>
      </div>
      <table class="performance-table">
        <thead>
          <tr>
            <th style:min-width="150px">Metric</th>
            <th>Time (ms)</th>
            <th>Count</th>
          </tr>
        </thead>
        <tbody>
          {#each $sortedPerformance as [key, value]}
            <tr>
              <td><b>{key}</b></td>
              {#if value.length > 0}
                <td class="text-center">{value[value.length - 1].time.toFixed(2)}</td>
              {/if}
              <td class="text-center">{value.length}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <!-- Export State Tab -->
    <div slot="export">
      <h3>Export State</h3>
      <PrismWrapper data={$exportState} />
    </div>

    <!-- Logging Tab -->
    <div slot="logging">
      <LoggingControls />
    </div>

    <!-- Debug Info Tab -->
    <div slot="debug">
      <h3>Debug Information</h3>
      <div class="debug-info">
        <div class="debug-item">
          <strong>Hovered Source Item:</strong>
          <span>{$hoveredSourceItem === null ? 'None' : $hoveredSourceItem}</span>
        </div>
        <div class="debug-item">
          <strong>Timer:</strong>
          <span>{seconds}ms</span>
        </div>
        <div class="debug-item">
          <strong>Is Over Table Container:</strong>
          <span>{$positionStore.isOverTableContainer}</span>
        </div>
        <div class="debug-item">
          <strong>Inputs Under Mouse:</strong>
          <span>{$positionStore.inputsUnderMouse}</span>
        </div>
        <div class="debug-item">
          <strong>Add New Folder on Drop:</strong>
          <span>{$addNewFolderOnDrop}</span>
        </div>
        <div class="debug-item">
          <strong>Position Store:</strong>
          <PrismWrapper data={$positionStore} maxHeight="200px" fontSize="11px" />
        </div>
        {#if appStateDebug}
          <div class="debug-item">
            <strong>Backend Debug:</strong>
            <PrismWrapper data={appStateDebug} maxHeight="300px" fontSize="11px" />
          </div>
        {/if}
      </div>
    </div>
  </TabContainer>
</div>

<style>
  /* Controls Section */
  .controls-section {
    display: flex;
    flex-direction: row;
    flex-wrap: wrap;
    gap: 20px;
    margin-bottom: 20px;
    padding: 16px;
    background-color: rgba(255, 255, 255, 0.05);
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    align-items: flex-start;
  }

  .button-group {
    display: flex;
    gap: 8px;
    align-items: flex-start;
  }

  .group-label {
    font-size: 11px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.6);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 4px;
  }

  /* Performance Section */
  .performance-controls {
    margin-bottom: 16px;
  }

  .performance-table {
    width: 100%;
    background-color: rgba(255, 255, 255, 0.05);
    border-radius: 6px;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .performance-table th {
    background-color: rgba(245, 158, 11, 0.2);
    color: #f59e0b;
    padding: 12px;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .performance-table td {
    padding: 8px 12px;
    font-size: 11px;
    color: rgba(255, 255, 255, 0.8);
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }

  .performance-table tr:last-child td {
    border-bottom: none;
  }

  /* Tab Content Styling for TabContainer */
  .performance-table {
    width: 100%;
    background-color: rgba(255, 255, 255, 0.05);
    border-radius: 6px;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  /* Override h3 styles for tab content */
  :global(.tab-panel h3) {
    margin: 0 0 16px 0;
    color: #f59e0b;
    font-size: 18px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  /* Debug Info Section */
  .debug-info {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .debug-item {
    background-color: rgba(255, 255, 255, 0.05);
    padding: 12px;
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .debug-item strong {
    color: #f59e0b;
    margin-right: 8px;
  }

  .debug-item span {
    color: rgba(255, 255, 255, 0.8);
  }

  /* Button Styles */
  .btn {
    border: 1px solid rgba(255, 255, 255, 0.3) !important;
    background-color: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.9);
    transition: all 0.2s ease;
    font-size: 12px;
    padding: 6px 12px;
    border-radius: 4px;
    width: min-content;
    white-space: nowrap;
  }

  /* Button Color Variants */
  .btn-primary {
    background-color: rgba(59, 130, 246, 0.2);
    border-color: rgba(59, 130, 246, 0.5) !important;
    color: #60a5fa;
  }

  .btn-primary:hover {
    background-color: rgba(59, 130, 246, 0.3);
    border-color: rgba(59, 130, 246, 0.7) !important;
  }

  .btn-success {
    background-color: rgba(34, 197, 94, 0.2);
    border-color: rgba(34, 197, 94, 0.5) !important;
    color: #4ade80;
  }

  .btn-success:hover {
    background-color: rgba(34, 197, 94, 0.3);
    border-color: rgba(34, 197, 94, 0.7) !important;
  }

  .btn-warning {
    background-color: rgba(245, 158, 11, 0.2);
    border-color: rgba(245, 158, 11, 0.5) !important;
    color: #fbbf24;
  }

  .btn-warning:hover {
    background-color: rgba(245, 158, 11, 0.3);
    border-color: rgba(245, 158, 11, 0.7) !important;
  }

  .btn-danger {
    background-color: rgba(239, 68, 68, 0.2);
    border-color: rgba(239, 68, 68, 0.5) !important;
    color: #f87171;
  }

  .btn-danger:hover {
    background-color: rgba(239, 68, 68, 0.3);
    border-color: rgba(239, 68, 68, 0.7) !important;
  }

  .btn-info {
    background-color: rgba(14, 165, 233, 0.2);
    border-color: rgba(14, 165, 233, 0.5) !important;
    color: #38bdf8;
  }

  .btn-info:hover {
    background-color: rgba(14, 165, 233, 0.3);
    border-color: rgba(14, 165, 233, 0.7) !important;
  }

  .btn-secondary {
    background-color: rgba(107, 114, 128, 0.2);
    border-color: rgba(107, 114, 128, 0.5) !important;
    color: #9ca3af;
  }

  .btn-secondary:hover {
    background-color: rgba(107, 114, 128, 0.3);
    border-color: rgba(107, 114, 128, 0.7) !important;
  }

  .btn:hover {
    background-color: rgba(255, 255, 255, 0.2);
    border-color: rgba(255, 255, 255, 0.5) !important;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Backend Controls */
  .backend-controls {
    display: flex;
    flex-direction: column;
    gap: 6px;
    align-items: flex-start;
  }

  .toggle-label {
    display: flex;
    align-items: center;
    gap: 6px;
    color: white;
    font-size: 12px;
    cursor: pointer;
    user-select: none;
  }

  .toggle-label input[type='checkbox'] {
    margin: 0;
  }

  /* Select Styles */
  select,
  .example-select,
  .svg-path-select {
    background-color: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.3);
    color: white;
    padding: 6px 8px;
    border-radius: 4px;
    font-size: 12px;
    width: min-content;
    min-width: 120px;
  }

  select option,
  .example-select option,
  .svg-path-select option {
    background-color: #1a1a1a;
    color: white;
  }

  /* SVG Path Controls */
  .svg-path-controls {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .control-label {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.8);
    margin: 0;
    white-space: nowrap;
  }

  .svg-path-select {
    min-width: 100px;
  }

  /* Responsive Design */
  @media (max-width: 768px) {
    .controls-section {
      flex-direction: row;
      gap: 4px;
    }

    .button-group {
      flex-direction: column;
      gap: 4px;
    }
  }
</style>
