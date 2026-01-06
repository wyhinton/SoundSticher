<script lang="ts">
  import {
    loggingState,
    updateBackendLoggingConfig,
    backendLogs,
    type BackendLogMessage,
  } from '$lib/state/logging';
  import PrismWrapper from './PrismWrapper.svelte';

  // Define logging system configurations
  const loggingConfigs = {
    // Frontend logging systems
    groupsLog: {
      category: 'frontend',
      label: 'Groups',
      icon: 'fa-folder',
    },
    selectionLog: {
      category: 'frontend',
      label: 'Selection',
      icon: 'fa-cursor',
    },
    dragdropLog: {
      category: 'frontend',
      label: 'Drag & Drop',
      icon: 'fa-arrows-alt',
    },
    // Backend logging systems
    encoderLog: {
      category: 'backend',
      label: 'Encoder',
      icon: 'fa-music',
    },
    combineLog: {
      category: 'backend',
      label: 'Combine',
      icon: 'fa-layer-group',
    },
    playbackLog: {
      category: 'backend',
      label: 'Playback',
      icon: 'fa-play',
    },
    sortingLog: {
      category: 'backend',
      label: 'Sorting',
      icon: 'fa-sort',
    },
    waveformLog: {
      category: 'backend',
      label: 'Waveform',
      icon: 'fa-wave-square',
    },
  };

  // Get frontend and backend systems
  $: frontendSystems = Object.entries(loggingConfigs).filter(
    ([_, config]) => config.category === 'frontend'
  );
  $: backendSystems = Object.entries(loggingConfigs).filter(
    ([_, config]) => config.category === 'backend'
  );

  // Handle logging toggle changes
  const handleLoggingChange = async (category: keyof typeof $loggingState, enabled: boolean) => {
    loggingState.update(state => ({
      ...state,
      [category]: enabled,
    }));

    // Update backend logging if it's a backend category
    const config = loggingConfigs[category];
    if (config && config.category === 'backend') {
      await updateBackendLoggingConfig($loggingState);
    }
  };

  // Clear backend logs
  const clearBackendLogs = () => {
    backendLogs.set([]);
  };

  // Filter logs by level
  let showDebug = true;
  let showInfo = true;
  let showWarning = true;
  let showError = true;

  $: filteredLogs = $backendLogs.filter(log => {
    if (!showDebug && log.level === 'debug') return false;
    if (!showInfo && log.level === 'info') return false;
    if (!showWarning && log.level === 'warning') return false;
    if (!showError && log.level === 'error') return false;
    return true;
  });

  // Log display limit
  let logLimit = 50;
</script>

<div class="logging-container">
  <div class="d-flex row g-1">
    <!-- Frontend Logging Controls -->
    <section class="logging-section col-6">
      <h4><i class="fa fa-desktop"></i> Frontend Logging</h4>
      <div class="logging-controls">
        {#each frontendSystems as [key, config] (key)}
          <label class="toggle-label">
            <input
              type="checkbox"
              bind:checked={$loggingState[key]}
              on:change={e => handleLoggingChange(key, e.currentTarget.checked)}
            />
            <i class="fa {config.icon}"></i>
            {config.label}
          </label>
        {/each}
      </div>
    </section>

    <!-- Backend Logging Controls -->
    <section class="logging-section col-6">
      <h4><i class="fa fa-server"></i> Backend Logging</h4>
      <div class="logging-controls">
        {#each backendSystems as [key, config] (key)}
          <label class="toggle-label">
            <input
              type="checkbox"
              bind:checked={$loggingState[key]}
              on:change={e => handleLoggingChange(key, e.currentTarget.checked)}
            />
            <i class="fa {config.icon}"></i>
            {config.label}
          </label>
        {/each}
      </div>
    </section>
  </div>

  <!-- Backend Logs Display Controls -->
  <section class="logging-section">
    <div class="logs-header">
      <h4>
        <i class="fa fa-terminal"></i> Backend Logs ({$backendLogs.length} total, {filteredLogs.length}
        filtered)
      </h4>
      <div class="log-controls">
        <button class="btn btn-sm btn-warning" on:click={clearBackendLogs}>
          <i class="fa fa-trash"></i> Clear Logs
        </button>
      </div>
    </div>

    <!-- Log Level Filters -->
    <div class="log-filters">
      <span class="filter-label">Show:</span>
      <div class="d-flex gap-2">
        <label class="filter-toggle">
          <input type="checkbox" bind:checked={showDebug} />
          <span class="level-badge debug">Debug</span>
        </label>
        <label class="filter-toggle">
          <input type="checkbox" bind:checked={showInfo} />
          <span class="level-badge info">Info</span>
        </label>
        <label class="filter-toggle">
          <input type="checkbox" bind:checked={showWarning} />
          <span class="level-badge warning">Warning</span>
        </label>
        <label class="filter-toggle">
          <input type="checkbox" bind:checked={showError} />
          <span class="level-badge error">Error</span>
        </label>
      </div>
      <div class="log-limit-control">
        <label for="log-limit">Limit:</label>
        <select id="log-limit" bind:value={logLimit}>
          <option value={20}>20</option>
          <option value={50}>50</option>
          <option value={100}>100</option>
          <option value={200}>200</option>
        </select>
      </div>
    </div>

    <!-- Backend Logs Display -->
    <div class="backend-logs">
      {#each filteredLogs.slice(-logLimit) as log (log.timestamp)}
        <div class="log-entry log-{log.level}">
          <span class="log-timestamp">{new Date(log.timestamp).toLocaleTimeString()}</span>
          <span class="log-level">{log.level.toUpperCase()}</span>
          <span class="log-system">{log.system.toUpperCase()}</span>
          {#if log.category}
            <span class="log-category">[{log.category}]</span>
          {/if}
          <span class="log-message">{log.message}</span>
          {#if log.data}
            <details class="log-data">
              <summary>Data</summary>
              <PrismWrapper data={log.data} maxHeight="150px" fontSize="10px" />
            </details>
          {/if}
        </div>
      {/each}
      {#if filteredLogs.length === 0}
        <div class="no-logs">
          {#if $backendLogs.length === 0}
            No backend logs yet. Enable logging above to see messages.
          {:else}
            No logs match current filters. Try adjusting the level filters above.
          {/if}
        </div>
      {/if}
    </div>
  </section>
</div>

<style>
  .logging-container {
    display: flex;
    flex-direction: column;
    gap: 8px;
    height: 100%;
  }

  .logging-section {
    background-color: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 4px;
    padding: 8px;
  }

  h4 {
    margin: 0 0 6px 0;
    color: #f59e0b;
    font-size: 12px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .logging-controls {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }

  .toggle-label {
    display: flex;
    align-items: center;
    gap: 4px;
    color: white;
    font-size: 11px;
    cursor: pointer;
    user-select: none;
    padding: 2px 6px;
    border-radius: 3px;
    border: 1px solid rgba(255, 255, 255, 0.2);
    background-color: rgba(255, 255, 255, 0.05);
    transition: all 0.2s ease;
  }

  .toggle-label:hover {
    background-color: rgba(255, 255, 255, 0.1);
    border-color: rgba(255, 255, 255, 0.3);
  }

  .toggle-label input[type='checkbox'] {
    margin: 0;
  }

  .logs-header {
    display: flex;
    justify-content: between;
    align-items: center;
    margin-bottom: 6px;
    gap: 8px;
  }

  .log-controls {
    display: flex;
    gap: 4px;
    align-items: center;
    margin-left: auto;
  }

  .log-filters {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    align-items: center;
    margin-bottom: 6px;
    padding: 4px;
    background-color: rgba(255, 255, 255, 0.03);
    border-radius: 3px;
  }

  .filter-label {
    color: rgba(255, 255, 255, 0.7);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .filter-toggle {
    display: flex;
    align-items: center;
    gap: 2px;
    cursor: pointer;
    user-select: none;
  }

  .level-badge {
    padding: 1px 4px;
    border-radius: 2px;
    font-size: 9px;
    font-weight: bold;
    text-transform: uppercase;
  }

  .level-badge.debug {
    background-color: #9e9e9e;
    color: white;
  }

  .level-badge.info {
    background-color: #2196f3;
    color: white;
  }

  .level-badge.warning {
    background-color: #ff9800;
    color: white;
  }

  .level-badge.error {
    background-color: #f44336;
    color: white;
  }

  .log-limit-control {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-left: auto;
  }

  .log-limit-control label {
    color: rgba(255, 255, 255, 0.7);
    font-size: 10px;
  }

  .log-limit-control select {
    background-color: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.3);
    color: white;
    padding: 1px 2px;
    border-radius: 2px;
    font-size: 10px;
  }

  .backend-logs {
    background-color: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 3px;
    padding: 4px;
    max-height: 300px;
    overflow-y: auto;
    font-family: 'Fira Code', monospace;
    font-size: 10px;
  }

  .log-entry {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-bottom: 2px;
    padding: 2px 0;
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
    align-items: center;
  }

  .log-entry:last-child {
    border-bottom: none;
  }

  .log-timestamp {
    color: #888;
    font-size: 9px;
    min-width: 60px;
  }

  .log-level {
    padding: 1px 3px;
    border-radius: 2px;
    font-weight: bold;
    font-size: 8px;
    min-width: 40px;
    text-align: center;
  }

  .log-entry.log-debug .log-level {
    background-color: #9e9e9e;
    color: white;
  }

  .log-entry.log-info .log-level {
    background-color: #2196f3;
    color: white;
  }

  .log-entry.log-warning .log-level {
    background-color: #ff9800;
    color: white;
  }

  .log-entry.log-error .log-level {
    background-color: #f44336;
    color: white;
  }

  .log-system {
    background-color: rgba(255, 255, 255, 0.15);
    padding: 1px 3px;
    border-radius: 2px;
    font-weight: bold;
    font-size: 8px;
    min-width: 50px;
    text-align: center;
    color: white;
  }

  .log-category {
    color: #ccc;
    font-style: italic;
    font-size: 9px;
  }

  .log-message {
    flex: 1;
    color: white;
    min-width: 150px;
  }

  .log-data {
    width: 100%;
    margin-top: 2px;
  }

  .log-data summary {
    cursor: pointer;
    color: #aaa;
    font-size: 9px;
    padding: 1px 0;
  }

  .log-data summary:hover {
    color: white;
  }

  .no-logs {
    color: #888;
    font-style: italic;
    text-align: center;
    padding: 12px;
    font-size: 11px;
  }

  .btn {
    border: 1px solid rgba(255, 255, 255, 0.3) !important;
    background-color: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.9);
    transition: all 0.2s ease;
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 3px;
    cursor: pointer;
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

  /* Responsive */
  @media (max-width: 768px) {
    .logs-header {
      flex-direction: column;
      align-items: flex-start;
      gap: 4px;
    }

    .log-controls {
      margin-left: 0;
    }

    .log-filters {
      flex-direction: column;
      align-items: flex-start;
      gap: 3px;
    }

    .log-limit-control {
      margin-left: 0;
    }
  }
</style>
