<script lang="ts">
  import {
    exportAudio,
    invokeWithPerf,
    performanceStore,
    resetPerformance,
    type PerformanceMetric,
  } from '$lib/state/performance';
  import PerformanceDebugTable from '$lib/components/PerformanceDebugTable.svelte';
  import {
    appState,
    hoveredSourceItem,
    resetAppState,
    setDebugActiveTab,
    setSvgPathDisplayMode,
    callSiteTrackingEnabled,
    toggleCallSiteTrackingEnabled,
  } from '$lib/state/state.svelte';
  import clipboard from 'tauri-plugin-clipboard-api';
  import { derived, get } from 'svelte/store';
  import { toSource } from '$lib/utils/format';
  import { onDestroy, onMount } from 'svelte';
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { exportState } from '$lib/state/export';
  import TabContainer from '$lib/components/TabContainer.svelte';
  import PrismWrapper from '$lib/components/PrismWrapper.svelte';
  import LoggingControls from '$lib/components/LoggingControls.svelte';
  import ArtifactRegistryDebug from '$lib/components/ArtifactRegistryDebug.svelte';
  import {
    selectedCount,
    previewCount,
    selectionSource,
    selectionDisplayData as selectionDisplayDataStore,
  } from '$lib/state/selection.svelte';
  import {
    initializeBackendLogListener,
    updateBackendLoggingConfig,
    loggingState,
    listenerLogs,
  } from '$lib/state/logging';
  import { addToFavorites } from '$lib/state/favorites';
  import {
    undo,
    redo,
    canUndo,
    canRedo,
    getUndoRedoLabels,
    getUndoRedoStackSizes,
    getUndoStack,
    getRedoStack,
    clearUndoRedoHistory,
  } from '$lib/state/undo/undo';

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
  $: frontEndStateFormatted = processSvgPaths(
    {
      ...$appState,
      // sections property removed - operations no longer have sections
    },
    svgDisplayMode
  );

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
  // Derive invoke history - all invokes sorted by timestamp
  const invokeHistory = derived(performanceStore, $store => {
    const allInvokes: Array<{
      command: string;
      metric: PerformanceMetric;
      timestamp: number;
    }> = [];

    // Collect all metrics from all commands
    Object.entries($store).forEach(([command, metrics]) => {
      metrics.forEach(metric => {
        allInvokes.push({
          command,
          metric,
          timestamp: metric.timeStamp,
        });
      });
    });

    // Sort by timestamp (newest first)
    return allInvokes.sort((a, b) => b.timestamp - a.timestamp).slice(0, 100); // Show last 100 invokes
  });

  // Derived store for selection state display
  // Using the store from selection.svelte.ts which includes all reactive properties

  interface AppStateDebug {
    audio_files: { [key: string]: any };
    combined_audio: string;
    buffering_samples: boolean;
    svg_path: string;
    cancel_token: number;
    combine_process: number;
  }

  let appStateDebug: undefined | AppStateDebug = undefined;

  const addTestFavorites = () => {
    const testFavoritePaths = [
      'C:\\Users\\Primary User\\Desktop\\AUDIO\\A_NUMBERED_SMALL',
      'C:\\Users\\Primary User\\Desktop\\AUDIO\\FREESOUNDS\\808-bass-drums',
    ];

    testFavoritePaths.forEach(path => {
      addToFavorites(path);
    });

    console.log('Added test favorites:', testFavoritePaths);
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

  // Reactive undo/redo state for debugging
  $: undoRedoStackSizes = getUndoRedoStackSizes();
  $: undoStack = getUndoStack();
  $: redoStack = getRedoStack();
  $: undoRedoLabels = getUndoRedoLabels();
  $: undoAvailable = canUndo();
  $: redoAvailable = canRedo();

  // Tab configuration
  const tabs = [
    { id: 'frontend', label: 'Frontend State', icon: 'fa-code' },
    { id: 'backend', label: 'Backend State', icon: 'fa-server' },
    { id: 'selection', label: 'Selection', icon: 'fa-check-square' },
    { id: 'performance', label: 'Performance', icon: 'fa-chart-line' },
    { id: 'invoke-history', label: 'Invoke History', icon: 'fa-history' },
    { id: 'undo-redo', label: 'Undo/Redo', icon: 'fa-undo' },
    { id: 'export', label: 'Export State', icon: 'fa-download' },
    { id: 'logging', label: 'Logging', icon: 'fa-terminal' },
    { id: 'listeners', label: 'Listeners', icon: 'fa-ear-listen' },
    { id: 'artifacts', label: 'Artifact Registry', icon: 'fa-archive' },
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
        () => console.log('Global sections removed - use currentOperationSections instead'),
        'fa-arrows-spin',
        'Log Current Op Sections',
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
      {@render actionButton(() => testExport(), 'fa-download', 'Test Export', false, 'success')}
      {@render actionButton(
        () => openAudioFolder(),
        'fa-folder-open',
        'Open Audio Folder',
        false,
        'secondary'
      )}
    </div>

    <!-- <div class="button-group">
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
      {@render actionButton(
        () => addTestFavorites(),
        'fa-heart',
        'Add Test Favorites',
        false,
        'primary'
      )}
    </div> -->
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
      <PrismWrapper data={frontEndStateFormatted} />
    </div>

    <!-- Backend State Tab -->
    <div slot="backend">
      <h3>Backend State</h3>
      <PrismWrapper data={appStateDebug || {}} />
    </div>

    <!-- Selection Tab -->
    <div slot="selection">
      <div class="d-flex justify-content-between align-items-center mb-3">
        <h3>Selection State</h3>
        <div class="selection-stats">
          <span class="stat-item">
            <strong>Selected:</strong>
            {$selectedCount}
          </span>
          <span class="stat-item">
            <strong>Preview:</strong>
            {$previewCount}
          </span>
          <span class="stat-item">
            <strong>Source:</strong>
            {$selectionSource || 'none'}
          </span>
        </div>
      </div>
      <PrismWrapper data={$selectionDisplayDataStore} panelKey="selection" />
    </div>

    <!-- Performance Tab -->
    <div slot="performance">
      <PerformanceDebugTable />
    </div>

    <!-- Invoke History Tab -->
    <div slot="invoke-history">
      <div class="d-flex justify-content-between align-items-center mb-3">
        <h3>Invoke History</h3>
        <div class="history-controls">
          <label class="toggle-label me-3">
            <input
              type="checkbox"
              checked={$callSiteTrackingEnabled}
              on:change={() => toggleCallSiteTrackingEnabled()}
            />
            <i class="fa fa-map-marker"></i> Call Site Tracking
          </label>
          {@render actionButton(
            () => resetPerformance(),
            'fa-trash',
            'Clear History',
            false,
            'danger'
          )}
        </div>
      </div>

      {#if $invokeHistory.length === 0}
        <div class="empty-state">
          <i class="fa fa-history"></i>
          <p>No invoke history yet. Call some Tauri commands to see them here.</p>
        </div>
      {:else}
        <div class="history-table-container">
          <table class="history-table">
            <thead>
              <tr>
                <th>Time</th>
                <th>Command</th>
                <th>Duration (ms)</th>
                <th>Call Site</th>
              </tr>
            </thead>
            <tbody>
              {#each $invokeHistory as { command, metric, timestamp }}
                <tr>
                  <td class="timestamp">
                    {new Date(timestamp).toLocaleTimeString('en-US', {
                      hour12: false,
                      hour: '2-digit',
                      minute: '2-digit',
                      second: '2-digit',
                    })}
                  </td>
                  <td class="command">
                    <code>{command}</code>
                  </td>
                  <td
                    class="duration text-center"
                    class:slow={metric.time > 100}
                    class:medium={metric.time > 50 && metric.time <= 100}
                  >
                    {metric.time.toFixed(2)}
                  </td>
                  <td class="call-site">
                    {#if metric.callSite}
                      <span class="call-site-info" title={metric.callSite}>
                        <i class="fa fa-map-marker"></i>
                        {metric.fileName}:{metric.lineNumber}
                      </span>
                    {:else}
                      <span class="no-call-site">
                        <i class="fa fa-question-circle"></i>
                        No tracking
                      </span>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </div>

    <!-- Undo/Redo Tab -->
    <div slot="undo-redo">
      <div class="d-flex justify-content-between align-items-center mb-3">
        <h3>Undo/Redo System</h3>
        <div class="undo-redo-controls">
          {@render actionButton(() => undo(), 'fa-undo', 'Undo', !undoAvailable, 'primary')}
          {@render actionButton(() => redo(), 'fa-redo', 'Redo', !redoAvailable, 'secondary')}
          {@render actionButton(
            () => clearUndoRedoHistory(),
            'fa-trash',
            'Clear History',
            false,
            'danger'
          )}
        </div>
      </div>

      <div class="undo-redo-stats">
        <div class="stat-card">
          <div class="stat-label">Undo Stack</div>
          <div class="stat-value">{undoRedoStackSizes.undoCount}</div>
          <div class="stat-description">Commands available to undo</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">Redo Stack</div>
          <div class="stat-value">{undoRedoStackSizes.redoCount}</div>
          <div class="stat-description">Commands available to redo</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">Next Undo</div>
          <div class="stat-value">{undoRedoLabels.undo || 'None'}</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">Next Redo</div>
          <div class="stat-value">{undoRedoLabels.redo || 'None'}</div>
        </div>
      </div>

      <div class="undo-redo-stacks">
        <div class="stack-section">
          <h4><i class="fa fa-undo"></i> Undo Stack</h4>
          {#if undoStack.length === 0}
            <div class="empty-stack">
              <i class="fa fa-inbox"></i>
              <p>No commands in undo stack</p>
            </div>
          {:else}
            <div class="stack-table-container">
              <table class="stack-table">
                <thead>
                  <tr>
                    <th>#</th>
                    <th>Command</th>
                    <th>ID</th>
                  </tr>
                </thead>
                <tbody>
                  {#each undoStack.toReversed() as command, index}
                    <tr class:next-command={index === 0}>
                      <td class="stack-index">{undoStack.length - index}</td>
                      <td class="command-label">{command.label}</td>
                      <td class="command-id">
                        {#if command.id}
                          <code>{command.id.slice(0, 8)}...</code>
                        {:else}
                          <span class="no-id">-</span>
                        {/if}
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {/if}
        </div>

        <div class="stack-section">
          <h4><i class="fa fa-redo"></i> Redo Stack</h4>
          {#if redoStack.length === 0}
            <div class="empty-stack">
              <i class="fa fa-inbox"></i>
              <p>No commands in redo stack</p>
            </div>
          {:else}
            <div class="stack-table-container">
              <table class="stack-table">
                <thead>
                  <tr>
                    <th>#</th>
                    <th>Command</th>
                    <th>ID</th>
                  </tr>
                </thead>
                <tbody>
                  {#each redoStack.toReversed() as command, index}
                    <tr class:next-command={index === 0}>
                      <td class="stack-index">{redoStack.length - index}</td>
                      <td class="command-label">{command.label}</td>
                      <td class="command-id">
                        {#if command.id}
                          <code>{command.id.slice(0, 8)}...</code>
                        {:else}
                          <span class="no-id">-</span>
                        {/if}
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {/if}
        </div>
      </div>
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

    <!-- Listeners Tab -->
    <div slot="listeners">
      <div class="d-flex justify-content-between align-items-center mb-3">
        <h3>Event Listeners</h3>
        <div class="listeners-controls">
          {@render actionButton(
            () => listenerLogs.set([]),
            'fa-trash',
            'Clear Logs',
            false,
            'danger'
          )}
        </div>
      </div>

      {#if $listenerLogs.length === 0}
        <div class="empty-state">
          <i class="fa fa-ear-listen"></i>
          <p>
            No event listener logs yet. Enable Listeners logging and interact with the UI to see
            logs here.
          </p>
          <small class="text-muted">
            Event listeners that use the <code>listenWithLogging</code> utility will appear here.
          </small>
        </div>
      {:else}
        <div class="listeners-table-container">
          <table class="listeners-table">
            <thead>
              <tr>
                <th>Time</th>
                <th>Action</th>
                <th>Element</th>
                <th>Event</th>
                <th>Details</th>
              </tr>
            </thead>
            <tbody>
              {#each $listenerLogs.slice().reverse() as log}
                <tr class="listener-log-{log.action}">
                  <td class="timestamp">
                    {new Date(log.timestamp).toLocaleTimeString('en-US', {
                      hour12: false,
                      hour: '2-digit',
                      minute: '2-digit',
                      second: '2-digit',
                    })}
                  </td>
                  <td class="action">
                    <span class="action-badge action-{log.action}">
                      {#if log.action === 'attach'}
                        <i class="fa fa-link"></i>
                      {:else if log.action === 'detach'}
                        <i class="fa fa-unlink"></i>
                      {:else if log.action === 'event'}
                        <i class="fa fa-bolt"></i>
                      {/if}
                      {log.action}
                    </span>
                  </td>
                  <td class="element">
                    <code class="element-info">
                      {log.elementType}
                      {#if log.elementId}
                        <span class="element-id">#{log.elementId}</span>
                      {/if}
                      {#if log.elementClass}
                        <span class="element-class"
                          >.{log.elementClass.split(' ').slice(0, 2).join('.')}</span
                        >
                      {/if}
                    </code>
                  </td>
                  <td class="event-type">
                    <code>{log.eventType}</code>
                  </td>
                  <td class="details">
                    {#if log.details}
                      <details class="details-dropdown">
                        <summary class="details-summary">
                          <i class="fa fa-info-circle"></i>
                        </summary>
                        <div class="details-content">
                          <PrismWrapper data={log.details} maxHeight="150px" fontSize="10px" />
                        </div>
                      </details>
                    {:else}
                      <span class="no-details">-</span>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </div>

    <!-- Artifact Registry Tab -->
    <div slot="artifacts">
      <ArtifactRegistryDebug />
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

  /* Invoke History Styles */
  .history-controls {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .history-table-container {
    max-height: 500px;
    overflow-y: auto;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    background-color: rgba(255, 255, 255, 0.05);
  }

  .history-table {
    width: 100%;
    background-color: transparent;
  }

  .history-table th {
    background-color: rgba(245, 158, 11, 0.2);
    color: #f59e0b;
    padding: 12px;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    position: sticky;
    top: 0;
    z-index: 1;
  }

  .history-table td {
    padding: 8px 12px;
    font-size: 11px;
    color: rgba(255, 255, 255, 0.8);
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    vertical-align: middle;
  }

  .history-table tr:hover {
    background-color: rgba(255, 255, 255, 0.05);
  }

  .timestamp {
    font-family: 'Courier New', monospace;
    color: rgba(156, 163, 175, 0.9);
    white-space: nowrap;
    min-width: 100px;
  }

  .command {
    min-width: 200px;
  }

  .command code {
    background-color: rgba(59, 130, 246, 0.1);
    color: #60a5fa;
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 10px;
    border: 1px solid rgba(59, 130, 246, 0.2);
  }

  .duration {
    font-family: 'Courier New', monospace;
    font-weight: 600;
    min-width: 80px;
  }

  .duration.medium {
    color: #fbbf24;
  }

  .duration.slow {
    color: #f87171;
  }

  .call-site {
    max-width: 250px;
  }

  .call-site-info {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background-color: rgba(34, 197, 94, 0.1);
    color: #4ade80;
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 10px;
    border: 1px solid rgba(34, 197, 94, 0.2);
  }

  .call-site-info i {
    font-size: 8px;
  }

  .no-call-site {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: rgba(156, 163, 175, 0.6);
    font-size: 10px;
    font-style: italic;
  }

  .no-call-site i {
    font-size: 8px;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 60px 20px;
    color: rgba(156, 163, 175, 0.6);
    text-align: center;
  }

  .empty-state i {
    font-size: 48px;
    margin-bottom: 16px;
    opacity: 0.3;
  }

  .empty-state p {
    margin: 0;
    font-style: italic;
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

  /* Selection Section */
  .selection-stats {
    display: flex;
    gap: 16px;
    align-items: center;
  }

  .stat-item {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.7);
    padding: 4px 8px;
    background-color: rgba(48, 145, 241, 0.1);
    border-radius: 4px;
    border: 1px solid rgba(48, 145, 241, 0.3);
  }

  .stat-item strong {
    color: rgba(255, 255, 255, 0.9);
    font-weight: 600;
  }

  /* Listeners Tab Styles */
  .listeners-controls {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .listeners-table-container {
    max-height: 500px;
    overflow-y: auto;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    background-color: rgba(255, 255, 255, 0.02);
  }

  .listeners-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 11px;
  }

  .listeners-table th {
    background-color: rgba(0, 188, 212, 0.2);
    color: #00bcd4;
    padding: 8px 12px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    position: sticky;
    top: 0;
    z-index: 1;
  }

  .listeners-table td {
    padding: 6px 12px;
    color: rgba(255, 255, 255, 0.8);
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    vertical-align: top;
  }

  .listeners-table tr:last-child td {
    border-bottom: none;
  }

  /* Row styling based on action */
  .listener-log-attach {
    background-color: rgba(76, 175, 80, 0.05);
  }

  .listener-log-detach {
    background-color: rgba(255, 87, 34, 0.05);
  }

  .listener-log-event {
    background-color: rgba(156, 39, 176, 0.05);
  }

  /* Action badge styling */
  .action-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 6px;
    border-radius: 12px;
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .action-attach {
    background-color: rgba(76, 175, 80, 0.2);
    color: #4caf50;
  }

  .action-detach {
    background-color: rgba(255, 87, 34, 0.2);
    color: #ff5722;
  }

  .action-event {
    background-color: rgba(156, 39, 176, 0.2);
    color: #9c27b0;
  }

  /* Element info styling */
  .element-info {
    font-size: 10px;
    color: rgba(255, 255, 255, 0.9);
    font-family: 'Fira Code', monospace;
  }

  .element-id {
    color: #81c784;
    font-weight: 600;
  }

  .element-class {
    color: #64b5f6;
    font-weight: 500;
  }

  /* Details dropdown */
  .details-dropdown {
    display: inline-block;
  }

  .details-summary {
    cursor: pointer;
    color: rgba(255, 255, 255, 0.6);
    font-size: 10px;
    list-style: none;
    padding: 2px 4px;
    border-radius: 3px;
    transition: background-color 0.2s;
  }

  .details-summary:hover {
    background-color: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.9);
  }

  .details-content {
    margin-top: 4px;
    padding: 8px;
    background-color: rgba(0, 0, 0, 0.3);
    border-radius: 4px;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .no-details {
    color: rgba(255, 255, 255, 0.4);
    font-style: italic;
  }

  /* Timestamp styling */
  .timestamp {
    font-family: 'Fira Code', monospace;
    color: rgba(255, 255, 255, 0.6);
    font-size: 10px;
  }

  .event-type {
    font-family: 'Fira Code', monospace;
    color: #ffb74d;
    font-weight: 500;
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

  /* Undo/Redo Tab Styles */
  .undo-redo-controls {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .undo-redo-stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 16px;
    margin-bottom: 24px;
  }

  .stat-card {
    background-color: rgba(255, 255, 255, 0.05);
    padding: 16px;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    text-align: center;
  }

  .stat-label {
    font-size: 12px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.6);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 8px;
  }

  .stat-value {
    font-weight: 700;
    color: #f59e0b;
    margin-bottom: 4px;
    font-family: 'Courier New', monospace;
  }

  .stat-description {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.5);
    font-style: italic;
  }

  .undo-redo-stacks {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 24px;
  }

  .stack-section h4 {
    margin: 0 0 16px 0;
    color: #60a5fa;
    font-size: 16px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .empty-stack {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 40px 20px;
    color: rgba(156, 163, 175, 0.5);
    text-align: center;
    background-color: rgba(255, 255, 255, 0.02);
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.05);
  }

  .empty-stack i {
    font-size: 32px;
    margin-bottom: 12px;
    opacity: 0.3;
  }

  .empty-stack p {
    margin: 0;
    font-style: italic;
    font-size: 12px;
  }

  .stack-table-container {
    max-height: 300px;
    overflow-y: auto;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    background-color: rgba(255, 255, 255, 0.02);
  }

  .stack-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 11px;
  }

  .stack-table th {
    background-color: rgba(96, 165, 250, 0.2);
    color: #60a5fa;
    padding: 8px 12px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    position: sticky;
    top: 0;
    z-index: 1;
  }

  .stack-table td {
    padding: 6px 12px;
    color: rgba(255, 255, 255, 0.8);
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    vertical-align: middle;
  }

  .stack-table tr:last-child td {
    border-bottom: none;
  }

  .stack-table tr:hover {
    background-color: rgba(255, 255, 255, 0.05);
  }

  .stack-table tr.next-command {
    background-color: rgba(245, 158, 11, 0.1);
    border: 1px solid rgba(245, 158, 11, 0.3);
  }

  .stack-table tr.next-command td {
    color: rgba(255, 255, 255, 0.95);
    font-weight: 500;
  }

  .stack-index {
    text-align: center;
    font-family: 'Courier New', monospace;
    font-weight: 600;
    color: #60a5fa;
    min-width: 40px;
  }

  .command-label {
    font-weight: 500;
    color: rgba(255, 255, 255, 0.9);
  }

  .command-id {
    font-family: 'Courier New', monospace;
    text-align: center;
    min-width: 80px;
  }

  .command-id code {
    background-color: rgba(156, 163, 175, 0.1);
    color: #9ca3af;
    padding: 2px 4px;
    border-radius: 3px;
    font-size: 9px;
    border: 1px solid rgba(156, 163, 175, 0.2);
  }

  .no-id {
    color: rgba(156, 163, 175, 0.4);
    font-style: italic;
  }

  @media (max-width: 768px) {
    .undo-redo-stacks {
      grid-template-columns: 1fr;
      gap: 16px;
    }

    .undo-redo-stats {
      grid-template-columns: repeat(2, 1fr);
    }
  }
</style>
