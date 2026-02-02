<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import PrismWrapper from '$lib/components/PrismWrapper.svelte';

  // Types
  interface ArtifactRecord {
    id: string;
    creator_op_id: string;
    created_at: number;
    artifact_type: string;
    size_bytes: number;
    exists: boolean;
    tags: Record<string, string>;
    file_paths: string[];
  }

  interface ArtifactRegistryStats {
    total_artifacts: number;
    existing_artifacts: number;
    total_size_bytes: number;
    operations_with_artifacts: number;
    artifacts_by_type: Record<string, number>;
  }

  interface ArtifactDebugInfo {
    stats: ArtifactRegistryStats;
    all_records: ArtifactRecord[];
    records_by_operation: Record<string, ArtifactRecord[]>;
    total_operations_with_artifacts: number;
  }

  interface ArtifactFilter {
    artifact_type?: string;
    exists?: boolean;
    operation_id?: string;
    min_size_bytes?: number;
    max_size_bytes?: number;
  }

  // State
  let debugInfo: ArtifactDebugInfo | null = null;
  let filteredRecords: ArtifactRecord[] = [];
  let selectedRecord: ArtifactRecord | null = null;
  let loading = false;
  let error = '';
  let autoRefresh = false;
  let refreshInterval: number | null = null;

  // Filter state
  let filter: ArtifactFilter = {};
  let filterType = '';
  let filterExists = '';
  let filterOperation = '';
  let filterMinSize = '';
  let filterMaxSize = '';

  // View options
  let viewMode: 'table' | 'operations' | 'stats' = 'table';
  let showDetails = false;

  // Load initial data
  onMount(() => {
    loadDebugInfo();
    return () => {
      if (refreshInterval) {
        clearInterval(refreshInterval);
      }
    };
  });

  // Auto refresh handling
  $: {
    if (autoRefresh && !refreshInterval) {
      refreshInterval = setInterval(loadDebugInfo, 2000);
    } else if (!autoRefresh && refreshInterval) {
      clearInterval(refreshInterval);
      refreshInterval = null;
    }
  }

  async function loadDebugInfo() {
    if (loading) return;
    loading = true;
    error = '';

    try {
      debugInfo = await invoke('get_artifact_debug_info');
      if (!filteredRecords.length || !isFilterActive()) {
        filteredRecords = debugInfo?.all_records || [];
      }
    } catch (err) {
      error = `Failed to load artifact debug info: ${err}`;
      console.error(err);
    } finally {
      loading = false;
    }
  }

  async function applyFilter() {
    if (!debugInfo) return;

    loading = true;
    try {
      const activeFilter: ArtifactFilter = {};

      if (filterType) activeFilter.artifact_type = filterType;
      if (filterExists !== '') activeFilter.exists = filterExists === 'true';
      if (filterOperation) activeFilter.operation_id = filterOperation;
      if (filterMinSize) activeFilter.min_size_bytes = parseInt(filterMinSize);
      if (filterMaxSize) activeFilter.max_size_bytes = parseInt(filterMaxSize);

      filteredRecords = await invoke('get_filtered_artifacts', { filter: activeFilter });
    } catch (err) {
      error = `Failed to apply filter: ${err}`;
      console.error(err);
    } finally {
      loading = false;
    }
  }

  function clearFilter() {
    filterType = '';
    filterExists = '';
    filterOperation = '';
    filterMinSize = '';
    filterMaxSize = '';
    filteredRecords = debugInfo?.all_records || [];
  }

  function isFilterActive(): boolean {
    return !!(
      filterType ||
      filterExists !== '' ||
      filterOperation ||
      filterMinSize ||
      filterMaxSize
    );
  }

  async function clearRegistry() {
    if (!confirm('Are you sure you want to clear the entire artifact registry?')) return;

    loading = true;
    try {
      const result = await invoke('clear_artifact_registry_debug');
      console.log(result);
      await loadDebugInfo();
    } catch (err) {
      error = `Failed to clear registry: ${err}`;
    } finally {
      loading = false;
    }
  }

  async function refreshExistence() {
    loading = true;
    try {
      const result = await invoke('refresh_artifacts_existence');
      console.log(result);
      await loadDebugInfo();
    } catch (err) {
      error = `Failed to refresh existence: ${err}`;
    } finally {
      loading = false;
    }
  }

  async function removeArtifactsByOperation(operationId: string) {
    if (!confirm(`Remove all artifacts created by operation '${operationId}'?`)) return;

    loading = true;
    try {
      const result = await invoke('remove_artifacts_by_operation_debug', { operationId });
      console.log(result);
      await loadDebugInfo();
    } catch (err) {
      error = `Failed to remove artifacts: ${err}`;
    } finally {
      loading = false;
    }
  }

  function selectRecord(record: ArtifactRecord) {
    selectedRecord = selectedRecord?.id === record.id ? null : record;
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  function formatTimestamp(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleString();
  }

  function getUniqueArtifactTypes(): string[] {
    if (!debugInfo) return [];
    return Object.keys(debugInfo.stats.artifacts_by_type);
  }

  function getUniqueOperations(): string[] {
    if (!debugInfo) return [];
    return Object.keys(debugInfo.records_by_operation);
  }
</script>

<div class="artifact-debug-container">
  <!-- Controls -->
  <div class="controls">
    <div class="button-group">
      <button class="btn btn-primary btn-sm" on:click={loadDebugInfo} disabled={loading}>
        <i class="fa fa-refresh {loading ? 'fa-spin' : ''}"></i> Refresh
      </button>

      <label class="auto-refresh-toggle">
        <input type="checkbox" bind:checked={autoRefresh} /> Auto Refresh
      </label>
    </div>

    <div class="button-group">
      <button class="btn btn-warning btn-sm" on:click={refreshExistence}>
        <i class="fa fa-check-circle"></i> Check Existence
      </button>

      <button class="btn btn-danger btn-sm" on:click={clearRegistry}>
        <i class="fa fa-trash"></i> Clear Registry
      </button>
    </div>

    <div class="view-mode-group">
      <button
        class="btn btn-sm {viewMode === 'stats' ? 'btn-primary' : 'btn-secondary'}"
        on:click={() => (viewMode = 'stats')}
      >
        <i class="fa fa-chart-bar"></i> Stats
      </button>
      <button
        class="btn btn-sm {viewMode === 'table' ? 'btn-primary' : 'btn-secondary'}"
        on:click={() => (viewMode = 'table')}
      >
        <i class="fa fa-table"></i> Table
      </button>
      <button
        class="btn btn-sm {viewMode === 'operations' ? 'btn-primary' : 'btn-secondary'}"
        on:click={() => (viewMode = 'operations')}
      >
        <i class="fa fa-sitemap"></i> By Operation
      </button>
    </div>
  </div>

  {#if error}
    <div class="error">
      <i class="fa fa-exclamation-triangle"></i>
      {error}
    </div>
  {/if}

  {#if loading && !debugInfo}
    <div class="loading">
      <i class="fa fa-spinner fa-spin"></i>
      Loading artifact registry info...
    </div>
  {:else if debugInfo}
    <!-- Stats View -->
    {#if viewMode === 'stats'}
      <div class="stats-view">
        <h3><i class="fa fa-chart-bar"></i> Registry Statistics</h3>

        <div class="stats-grid">
          <div class="stat-card">
            <div class="stat-value">{debugInfo.stats.total_artifacts}</div>
            <div class="stat-label">Total Artifacts</div>
          </div>

          <div class="stat-card">
            <div class="stat-value">{debugInfo.stats.existing_artifacts}</div>
            <div class="stat-label">Existing</div>
          </div>

          <div class="stat-card">
            <div class="stat-value">
              {debugInfo.stats.total_artifacts - debugInfo.stats.existing_artifacts}
            </div>
            <div class="stat-label">Missing</div>
          </div>

          <div class="stat-card">
            <div class="stat-value">{formatBytes(debugInfo.stats.total_size_bytes)}</div>
            <div class="stat-label">Total Size</div>
          </div>

          <div class="stat-card">
            <div class="stat-value">{debugInfo.stats.operations_with_artifacts}</div>
            <div class="stat-label">Operations</div>
          </div>
        </div>

        <h4>Artifacts by Type</h4>
        <div class="type-breakdown">
          {#each Object.entries(debugInfo.stats.artifacts_by_type) as [type, count]}
            <div class="type-item">
              <span class="type-name">{type}</span>
              <span class="type-count">{count}</span>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Operations View -->
    {#if viewMode === 'operations'}
      <div class="operations-view">
        <h3><i class="fa fa-sitemap"></i> Artifacts by Operation</h3>

        {#each Object.entries(debugInfo.records_by_operation) as [operationId, records]}
          <div class="operation-section">
            <div class="operation-header">
              <h4>
                <i class="fa fa-cog"></i>
                {operationId}
                <span class="artifact-count">({records.length} artifacts)</span>
              </h4>
              <button
                class="btn btn-danger btn-sm"
                on:click={() => removeArtifactsByOperation(operationId)}
              >
                <i class="fa fa-trash"></i> Remove All
              </button>
            </div>

            <div class="operation-artifacts">
              {#each records as record}
                <div class="artifact-item" class:missing={!record.exists}>
                  <div class="artifact-info">
                    <span class="artifact-type">{record.artifact_type}</span>
                    <span class="artifact-size">{formatBytes(record.size_bytes)}</span>
                    {#if !record.exists}
                      <span class="missing-indicator">
                        <i class="fa fa-exclamation-triangle"></i> Missing
                      </span>
                    {/if}
                  </div>
                  <div class="artifact-paths">
                    {#each record.file_paths as path}
                      <code class="file-path">{path}</code>
                    {/each}
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/each}
      </div>
    {/if}

    <!-- Table View -->
    {#if viewMode === 'table'}
      <!-- Filters -->
      <div class="filters">
        <div class="filter-row">
          <select bind:value={filterType} class="filter-select">
            <option value="">All Types</option>
            {#each getUniqueArtifactTypes() as type}
              <option value={type}>{type}</option>
            {/each}
          </select>

          <select bind:value={filterExists} class="filter-select">
            <option value="">All Status</option>
            <option value="true">Exists</option>
            <option value="false">Missing</option>
          </select>

          <select bind:value={filterOperation} class="filter-select">
            <option value="">All Operations</option>
            {#each getUniqueOperations() as op}
              <option value={op}>{op}</option>
            {/each}
          </select>

          <input
            type="number"
            bind:value={filterMinSize}
            placeholder="Min size (bytes)"
            class="filter-input"
          />

          <input
            type="number"
            bind:value={filterMaxSize}
            placeholder="Max size (bytes)"
            class="filter-input"
          />

          <button class="btn btn-primary btn-sm" on:click={applyFilter}>
            <i class="fa fa-filter"></i> Filter
          </button>

          <button class="btn btn-secondary btn-sm" on:click={clearFilter}>
            <i class="fa fa-times"></i> Clear
          </button>
        </div>
      </div>

      <!-- Table -->
      <div class="table-container">
        <table class="artifacts-table">
          <thead>
            <tr>
              <th>Type</th>
              <th>Size</th>
              <th>Status</th>
              <th>Operation</th>
              <th>Created</th>
              <th>Tags</th>
              <th>Files</th>
            </tr>
          </thead>
          <tbody>
            {#each filteredRecords as record}
              <tr
                class:selected={selectedRecord?.id === record.id}
                class:missing={!record.exists}
                on:click={() => selectRecord(record)}
              >
                <td>
                  <span class="type-badge">{record.artifact_type}</span>
                </td>
                <td class="size-cell">{formatBytes(record.size_bytes)}</td>
                <td>
                  {#if record.exists}
                    <span class="status-exists"><i class="fa fa-check"></i> Exists</span>
                  {:else}
                    <span class="status-missing"
                      ><i class="fa fa-exclamation-triangle"></i> Missing</span
                    >
                  {/if}
                </td>
                <td class="operation-cell">
                  <code>{record.creator_op_id}</code>
                </td>
                <td class="timestamp-cell">{formatTimestamp(record.created_at)}</td>
                <td class="tags-cell">
                  {#if Object.keys(record.tags).length > 0}
                    <span class="tag-count">{Object.keys(record.tags).length} tags</span>
                  {:else}
                    <span class="no-tags">No tags</span>
                  {/if}
                </td>
                <td class="files-cell">
                  {#if record.file_paths.length > 0}
                    <span class="file-count">{record.file_paths.length} files</span>
                  {:else}
                    <span class="no-files">No files</span>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>

        {#if filteredRecords.length === 0}
          <div class="empty-state">
            <i class="fa fa-archive"></i>
            <p>No artifacts found</p>
          </div>
        {/if}
      </div>
    {/if}

    <!-- Selected Record Details -->
    {#if selectedRecord}
      <div class="record-details">
        <h4>
          <i class="fa fa-info-circle"></i>
          Artifact Details: {selectedRecord.id}
          <button class="btn btn-sm btn-secondary" on:click={() => (selectedRecord = null)}>
            <i class="fa fa-times"></i>
          </button>
        </h4>

        <PrismWrapper
          code={JSON.stringify(selectedRecord, null, 2)}
          language="json"
          showLineNumbers={false}
        />
      </div>
    {/if}
  {:else}
    <div class="empty-state">
      <i class="fa fa-archive"></i>
      <p>No artifact registry data available</p>
    </div>
  {/if}
</div>

<style>
  .artifact-debug-container {
    display: flex;
    flex-direction: column;
    gap: 16px;
    height: 100%;
  }

  .controls {
    display: flex;
    gap: 12px;
    align-items: center;
    flex-wrap: wrap;
    padding: 12px;
    background-color: rgba(255, 255, 255, 0.05);
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .button-group {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .view-mode-group {
    display: flex;
    gap: 2px;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
    overflow: hidden;
  }

  .auto-refresh-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.8);
    cursor: pointer;
  }

  .error {
    color: #f87171;
    background-color: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    padding: 12px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .loading {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 60px;
    color: rgba(255, 255, 255, 0.6);
  }

  /* Stats View */
  .stats-view h3 {
    margin: 0 0 16px 0;
    color: #f59e0b;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: 12px;
    margin-bottom: 24px;
  }

  .stat-card {
    background-color: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    padding: 16px;
    text-align: center;
  }

  .stat-value {
    font-size: 24px;
    font-weight: 600;
    color: #60a5fa;
    margin-bottom: 4px;
  }

  .stat-label {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.6);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .type-breakdown {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .type-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background-color: rgba(255, 255, 255, 0.05);
    border-radius: 4px;
  }

  .type-name {
    font-family: 'Courier New', monospace;
    color: #4ade80;
  }

  .type-count {
    color: #60a5fa;
    font-weight: 600;
  }

  /* Operations View */
  .operations-view h3 {
    margin: 0 0 16px 0;
    color: #f59e0b;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .operation-section {
    margin-bottom: 24px;
    background-color: rgba(255, 255, 255, 0.03);
    border-radius: 6px;
    padding: 16px;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .operation-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }

  .operation-header h4 {
    margin: 0;
    color: #4ade80;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .artifact-count {
    color: rgba(255, 255, 255, 0.6);
    font-weight: normal;
  }

  .operation-artifacts {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .artifact-item {
    padding: 8px 12px;
    background-color: rgba(255, 255, 255, 0.05);
    border-radius: 4px;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .artifact-item.missing {
    border-color: rgba(239, 68, 68, 0.3);
    background-color: rgba(239, 68, 68, 0.05);
  }

  .artifact-info {
    display: flex;
    gap: 12px;
    align-items: center;
    margin-bottom: 4px;
  }

  .artifact-type {
    background-color: rgba(59, 130, 246, 0.2);
    color: #60a5fa;
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 11px;
    font-weight: 600;
  }

  .artifact-size {
    color: rgba(255, 255, 255, 0.8);
    font-size: 12px;
  }

  .missing-indicator {
    color: #f87171;
    font-size: 11px;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .artifact-paths {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .file-path {
    font-size: 10px;
    color: rgba(255, 255, 255, 0.6);
    background-color: rgba(0, 0, 0, 0.3);
    padding: 2px 4px;
    border-radius: 2px;
  }

  /* Filters */
  .filters {
    background-color: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    padding: 12px;
  }

  .filter-row {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }

  .filter-select,
  .filter-input {
    background-color: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    color: white;
    padding: 6px 8px;
    border-radius: 4px;
    font-size: 12px;
    min-width: 120px;
  }

  .filter-input {
    min-width: 140px;
  }

  .filter-select option {
    background-color: #1a1a1a;
  }

  /* Table */
  .table-container {
    flex: 1;
    overflow: auto;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    background-color: rgba(255, 255, 255, 0.02);
  }

  .artifacts-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }

  .artifacts-table th {
    background-color: rgba(245, 158, 11, 0.2);
    color: #f59e0b;
    padding: 12px 8px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    position: sticky;
    top: 0;
    z-index: 1;
  }

  .artifacts-table td {
    padding: 8px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    color: rgba(255, 255, 255, 0.8);
    vertical-align: top;
  }

  .artifacts-table tr:hover {
    background-color: rgba(255, 255, 255, 0.05);
    cursor: pointer;
  }

  .artifacts-table tr.selected {
    background-color: rgba(59, 130, 246, 0.1);
    border: 1px solid rgba(59, 130, 246, 0.3);
  }

  .artifacts-table tr.missing {
    background-color: rgba(239, 68, 68, 0.05);
  }

  .type-badge {
    background-color: rgba(34, 197, 94, 0.2);
    color: #4ade80;
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 10px;
    font-weight: 600;
  }

  .size-cell {
    font-family: 'Courier New', monospace;
    text-align: right;
  }

  .status-exists {
    color: #4ade80;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .status-missing {
    color: #f87171;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .operation-cell code {
    background-color: rgba(107, 114, 128, 0.2);
    color: #9ca3af;
    padding: 2px 4px;
    border-radius: 2px;
    font-size: 10px;
  }

  .timestamp-cell {
    font-family: 'Courier New', monospace;
    font-size: 10px;
    color: rgba(255, 255, 255, 0.6);
  }

  .tags-cell,
  .files-cell {
    text-align: center;
  }

  .tag-count,
  .file-count {
    color: #60a5fa;
    font-size: 11px;
  }

  .no-tags,
  .no-files {
    color: rgba(255, 255, 255, 0.4);
    font-style: italic;
    font-size: 11px;
  }

  /* Record Details */
  .record-details {
    background-color: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    padding: 16px;
  }

  .record-details h4 {
    margin: 0 0 12px 0;
    color: #60a5fa;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
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

  /* Button Styles */
  .btn {
    border: 1px solid rgba(255, 255, 255, 0.3);
    background-color: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.9);
    transition: all 0.2s ease;
    font-size: 12px;
    padding: 6px 12px;
    border-radius: 4px;
    cursor: pointer;
    white-space: nowrap;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-primary {
    background-color: rgba(59, 130, 246, 0.2);
    border-color: rgba(59, 130, 246, 0.5);
    color: #60a5fa;
  }

  .btn-primary:hover:not(:disabled) {
    background-color: rgba(59, 130, 246, 0.3);
  }

  .btn-secondary {
    background-color: rgba(107, 114, 128, 0.2);
    border-color: rgba(107, 114, 128, 0.5);
    color: #9ca3af;
  }

  .btn-secondary:hover:not(:disabled) {
    background-color: rgba(107, 114, 128, 0.3);
  }

  .btn-warning {
    background-color: rgba(245, 158, 11, 0.2);
    border-color: rgba(245, 158, 11, 0.5);
    color: #fbbf24;
  }

  .btn-warning:hover:not(:disabled) {
    background-color: rgba(245, 158, 11, 0.3);
  }

  .btn-danger {
    background-color: rgba(239, 68, 68, 0.2);
    border-color: rgba(239, 68, 68, 0.5);
    color: #f87171;
  }

  .btn-danger:hover:not(:disabled) {
    background-color: rgba(239, 68, 68, 0.3);
  }

  .btn-sm {
    padding: 4px 8px;
    font-size: 11px;
  }
</style>
