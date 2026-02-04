<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import PrismWrapper from './PrismWrapper.svelte';
  import {
    timelineStore,
    selectionStore,
    playbackStore,
    timelineDebugMode,
    type TimelineId,
    type TimelineState,
    type TimelineSelection,
    type TimelinePlayback,
  } from '../state/timelines';

  // Internal state for debugging
  let timelineStoreData: Record<TimelineId, TimelineState> = {};
  let selectionStoreData: Record<TimelineId, TimelineSelection> = {};
  let playbackStoreData: Record<TimelineId, TimelinePlayback> = {};
  let debugModeEnabled = false;

  // Track all timeline IDs that have been registered
  let allTimelineIds: Set<TimelineId> = new Set();

  // Auto-refresh toggle
  let autoRefresh = true;
  let refreshInterval: number | null = null;

  // Subscriptions
  let unsubscribeTimeline: (() => void) | null = null;
  let unsubscribeSelection: (() => void) | null = null;
  let unsubscribePlayback: (() => void) | null = null;
  let unsubscribeDebugMode: (() => void) | null = null;

  function refreshStoreData() {
    // Get current store data directly from the internal stores
    timelineStoreData = get(timelineStore) || {};
    selectionStoreData = get(selectionStore) || {};
    playbackStoreData = get(playbackStore) || {};
    debugModeEnabled = get(timelineDebugMode);

    // Track all timeline IDs across all stores
    const allIds = new Set([
      ...Object.keys(timelineStoreData),
      ...Object.keys(selectionStoreData),
      ...Object.keys(playbackStoreData),
    ]);
    allTimelineIds = allIds;
  }

  function setupSubscriptions() {
    // Subscribe to all stores for real-time updates
    unsubscribeTimeline = timelineStore.subscribe(data => {
      timelineStoreData = data || {};
      updateAllTimelineIds();
    });

    unsubscribeSelection = selectionStore.subscribe(data => {
      selectionStoreData = data || {};
      updateAllTimelineIds();
    });

    unsubscribePlayback = playbackStore.subscribe(data => {
      playbackStoreData = data || {};
      updateAllTimelineIds();
    });

    unsubscribeDebugMode = timelineDebugMode.subscribe(enabled => {
      debugModeEnabled = enabled;
    });
  }

  function updateAllTimelineIds() {
    const allIds = new Set([
      ...Object.keys(timelineStoreData),
      ...Object.keys(selectionStoreData),
      ...Object.keys(playbackStoreData),
    ]);
    allTimelineIds = allIds;
  }

  function clearAllStores() {
    if (
      confirm(
        'Are you sure you want to clear all timeline stores? This will remove all timeline data.'
      )
    ) {
      timelineStore.clear();
      // Note: selectionStore and playbackStore will be cleared when their associated timelines are removed
      console.log('🗑️ All timeline stores cleared');
    }
  }

  function removeTimeline(timelineId: TimelineId) {
    if (confirm(`Remove timeline "${timelineId}" from all stores?`)) {
      timelineStore.remove(timelineId);
      selectionStore.remove(timelineId);
      playbackStore.remove(timelineId);
      console.log(`🗑️ Timeline "${timelineId}" removed from all stores`);
    }
  }

  function initTimeline(timelineId: string) {
    if (!timelineId.trim()) return;

    timelineStore.init(timelineId);
    selectionStore.init(timelineId);
    playbackStore.init(timelineId);
    console.log(`✨ Timeline "${timelineId}" initialized in all stores`);
  }

  function toggleDebugMode() {
    timelineDebugMode.toggle();
  }

  // Auto-refresh logic
  $: {
    if (autoRefresh && refreshInterval === null) {
      refreshInterval = setInterval(refreshStoreData, 1000) as unknown as number;
    } else if (!autoRefresh && refreshInterval !== null) {
      clearInterval(refreshInterval);
      refreshInterval = null;
    }
  }

  onMount(() => {
    refreshStoreData();
    setupSubscriptions();
  });

  onDestroy(() => {
    // Clean up subscriptions
    if (unsubscribeTimeline) unsubscribeTimeline();
    if (unsubscribeSelection) unsubscribeSelection();
    if (unsubscribePlayback) unsubscribePlayback();
    if (unsubscribeDebugMode) unsubscribeDebugMode();

    // Clean up auto-refresh
    if (refreshInterval !== null) {
      clearInterval(refreshInterval);
    }
  });

  // New timeline ID for initialization
  let newTimelineId = '';

  // Computed summary data
  $: timelineCount = allTimelineIds.size;
  $: storeStats = {
    timelineStoreEntries: Object.keys(timelineStoreData).length,
    selectionStoreEntries: Object.keys(selectionStoreData).length,
    playbackStoreEntries: Object.keys(playbackStoreData).length,
    totalSelectedItems: Object.values(selectionStoreData).reduce(
      (sum, sel) => sum + sel.selectedIds.size,
      0
    ),
    totalPreviewItems: Object.values(selectionStoreData).reduce(
      (sum, sel) => sum + sel.previewIds.size,
      0
    ),
    playingTimelines: Object.values(playbackStoreData).filter(pb => pb.isPlaying).length,
  };

  // Individual timeline data for detailed view
  $: timelineDetails = Array.from(allTimelineIds).map(id => ({
    id,
    timeline: timelineStoreData[id] || null,
    selection: selectionStoreData[id] || null,
    playback: playbackStoreData[id] || null,
  }));
</script>

<div class="timeline-store-debug">
  <!-- Header Controls -->
  <div class="debug-header">
    <div class="header-section">
      <h3><i class="fa fa-database"></i> Timeline Stores Debug</h3>
      <div class="stats-summary">
        <span class="stat-badge">
          <i class="fa fa-layer-group"></i>
          {timelineCount} Timelines
        </span>
        <span class="stat-badge stat-{storeStats.playingTimelines > 0 ? 'playing' : 'stopped'}">
          <i class="fa fa-play"></i>
          {storeStats.playingTimelines} Playing
        </span>
        <span class="stat-badge">
          <i class="fa fa-check-square"></i>
          {storeStats.totalSelectedItems} Selected
        </span>
      </div>
    </div>

    <div class="controls-section">
      <div class="control-group">
        <label class="toggle-control">
          <input type="checkbox" bind:checked={autoRefresh} />
          <i class="fa fa-sync-alt"></i> Auto-refresh
        </label>

        <label class="toggle-control">
          <input type="checkbox" checked={debugModeEnabled} on:change={toggleDebugMode} />
          <i class="fa fa-bug"></i> Debug Mode
        </label>

        <button class="btn btn-sm btn-secondary" on:click={refreshStoreData} disabled={autoRefresh}>
          <i class="fa fa-refresh"></i> Refresh
        </button>
      </div>

      <div class="control-group">
        <div class="new-timeline-input">
          <input
            type="text"
            bind:value={newTimelineId}
            placeholder="Timeline ID"
            class="timeline-id-input"
            on:keydown={e => e.key === 'Enter' && initTimeline(newTimelineId)}
          />
          <button
            class="btn btn-sm btn-success"
            on:click={() => initTimeline(newTimelineId)}
            disabled={!newTimelineId.trim()}
          >
            <i class="fa fa-plus"></i> Init
          </button>
        </div>

        <button class="btn btn-sm btn-danger" on:click={clearAllStores}>
          <i class="fa fa-trash"></i> Clear All
        </button>
      </div>
    </div>
  </div>

  <!-- Store Statistics -->
  <div class="store-stats">
    <div class="stat-card">
      <div class="stat-header">
        <i class="fa fa-layer-group"></i>
        Timeline Store
      </div>
      <div class="stat-value">{storeStats.timelineStoreEntries}</div>
      <div class="stat-label">Entries</div>
    </div>

    <div class="stat-card">
      <div class="stat-header">
        <i class="fa fa-mouse-pointer"></i>
        Selection Store
      </div>
      <div class="stat-value">{storeStats.selectionStoreEntries}</div>
      <div class="stat-label">Entries</div>
    </div>

    <div class="stat-card">
      <div class="stat-header">
        <i class="fa fa-play"></i>
        Playback Store
      </div>
      <div class="stat-value">{storeStats.playbackStoreEntries}</div>
      <div class="stat-label">Entries</div>
    </div>

    <div class="stat-card">
      <div class="stat-header">
        <i class="fa fa-check"></i>
        Selected Items
      </div>
      <div class="stat-value">{storeStats.totalSelectedItems}</div>
      <div class="stat-label">Total</div>
    </div>

    <div class="stat-card">
      <div class="stat-header">
        <i class="fa fa-eye"></i>
        Preview Items
      </div>
      <div class="stat-value">{storeStats.totalPreviewItems}</div>
      <div class="stat-label">Total</div>
    </div>
  </div>

  <!-- Timeline Details -->
  {#if timelineCount === 0}
    <div class="empty-state">
      <i class="fa fa-inbox"></i>
      <h4>No Timelines Found</h4>
      <p>Initialize a timeline using the controls above to see store data here.</p>
    </div>
  {:else}
    <div class="timeline-details">
      {#each timelineDetails as detail (detail.id)}
        <div class="timeline-card">
          <div class="timeline-header">
            <div class="timeline-title">
              <i class="fa fa-layer-group"></i>
              <strong>{detail.id}</strong>
              {#if detail.playback?.isPlaying}
                <span class="playing-indicator">
                  <i class="fa fa-play"></i>
                  Playing
                </span>
              {/if}
            </div>
            <button
              class="btn btn-sm btn-outline-danger"
              on:click={() => removeTimeline(detail.id)}
              aria-label="Remove timeline {detail.id}"
            >
              <i class="fa fa-trash"></i>
            </button>
          </div>

          <div class="timeline-stores">
            <!-- Timeline State -->
            <div class="store-section">
              <div class="store-title">
                <i class="fa fa-database"></i>
                Timeline State
              </div>
              {#if detail.timeline}
                <div class="store-summary">
                  <span class="summary-item">
                    <strong>Items:</strong>
                    {detail.timeline.items.length}
                  </span>
                  <span class="summary-item">
                    <strong>Duration:</strong>
                    {detail.timeline.duration.toFixed(2)}s
                  </span>
                  <span class="summary-item">
                    <strong>Loading:</strong>
                    {detail.timeline.waveformsLoading ? 'Yes' : 'No'}
                  </span>
                </div>
                <details class="store-details">
                  <summary class="details-summary">View Details</summary>
                  <PrismWrapper data={detail.timeline} maxHeight="200px" fontSize="10px" />
                </details>
              {:else}
                <div class="no-data">No timeline data</div>
              {/if}
            </div>

            <!-- Selection State -->
            <div class="store-section">
              <div class="store-title">
                <i class="fa fa-mouse-pointer"></i>
                Selection State
              </div>
              {#if detail.selection}
                <div class="store-summary">
                  <span class="summary-item">
                    <strong>Selected:</strong>
                    {detail.selection.selectedIds.size}
                  </span>
                  <span class="summary-item">
                    <strong>Preview:</strong>
                    {detail.selection.previewIds.size}
                  </span>
                  <span class="summary-item">
                    <strong>Last:</strong>
                    {detail.selection.lastSelectedIndex ?? 'None'}
                  </span>
                </div>
                <details class="store-details">
                  <summary class="details-summary">View Details</summary>
                  <PrismWrapper
                    data={{
                      ...detail.selection,
                      selectedIds: Array.from(detail.selection.selectedIds),
                      previewIds: Array.from(detail.selection.previewIds),
                    }}
                    maxHeight="200px"
                    fontSize="10px"
                  />
                </details>
              {:else}
                <div class="no-data">No selection data</div>
              {/if}
            </div>

            <!-- Playback State -->
            <div class="store-section">
              <div class="store-title">
                <i class="fa fa-play"></i>
                Playback State
              </div>
              {#if detail.playback}
                <div class="store-summary">
                  <span class="summary-item">
                    <strong>Progress:</strong>
                    {(detail.playback.progress * 100).toFixed(1)}%
                  </span>
                  <span class="summary-item">
                    <strong>Time:</strong>
                    {detail.playback.currentTime.toFixed(2)}s
                  </span>
                  <span class="summary-item">
                    <strong>State:</strong>
                    {detail.playback.isPlaying ? 'Playing' : 'Stopped'}
                  </span>
                </div>
                <details class="store-details">
                  <summary class="details-summary">View Details</summary>
                  <PrismWrapper data={detail.playback} maxHeight="200px" fontSize="10px" />
                </details>
              {:else}
                <div class="no-data">No playback data</div>
              {/if}
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  <!-- Raw Store Data -->
  <div class="raw-data-section">
    <h4><i class="fa fa-code"></i> Raw Store Data</h4>

    <div class="raw-stores">
      <div class="raw-store">
        <div class="raw-store-title">Timeline Store</div>
        <PrismWrapper data={timelineStoreData} maxHeight="300px" fontSize="10px" />
      </div>

      <div class="raw-store">
        <div class="raw-store-title">Selection Store</div>
        <PrismWrapper
          data={Object.fromEntries(
            Object.entries(selectionStoreData).map(([id, sel]) => [
              id,
              {
                ...sel,
                selectedIds: Array.from(sel.selectedIds),
                previewIds: Array.from(sel.previewIds),
              },
            ])
          )}
          maxHeight="300px"
          fontSize="10px"
        />
      </div>

      <div class="raw-store">
        <div class="raw-store-title">Playback Store</div>
        <PrismWrapper data={playbackStoreData} maxHeight="300px" fontSize="10px" />
      </div>
    </div>
  </div>
</div>

<style>
  .timeline-store-debug {
    display: flex;
    flex-direction: column;
    gap: 20px;
    padding: 16px;
  }

  /* Header */
  .debug-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 16px;
    padding: 16px;
    background-color: rgba(255, 255, 255, 0.05);
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    flex-wrap: wrap;
  }

  .header-section h3 {
    margin: 0;
    color: #f8fafc;
    font-size: 18px;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .stats-summary {
    display: flex;
    gap: 8px;
    margin-top: 8px;
    flex-wrap: wrap;
  }

  .stat-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    background-color: rgba(59, 130, 246, 0.1);
    color: #60a5fa;
    border: 1px solid rgba(59, 130, 246, 0.2);
    border-radius: 4px;
    font-size: 11px;
    font-weight: 500;
  }

  .stat-badge.stat-playing {
    background-color: rgba(34, 197, 94, 0.1);
    color: #4ade80;
    border-color: rgba(34, 197, 94, 0.2);
  }

  .controls-section {
    display: flex;
    gap: 16px;
    align-items: flex-end;
    flex-wrap: wrap;
  }

  .control-group {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .toggle-control {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.8);
    cursor: pointer;
  }

  .toggle-control input {
    margin: 0;
  }

  .new-timeline-input {
    display: flex;
    gap: 4px;
  }

  .timeline-id-input {
    padding: 6px 8px;
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
    background-color: rgba(255, 255, 255, 0.05);
    color: #f8fafc;
    font-size: 12px;
    width: 120px;
  }

  .timeline-id-input::placeholder {
    color: rgba(255, 255, 255, 0.4);
  }

  /* Statistics Cards */
  .store-stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 16px;
  }

  .stat-card {
    background-color: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    padding: 16px;
    text-align: center;
  }

  .stat-header {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.6);
    font-weight: 500;
    margin-bottom: 8px;
  }

  .stat-value {
    font-size: 24px;
    font-weight: 700;
    color: #f8fafc;
    line-height: 1;
  }

  .stat-label {
    font-size: 10px;
    color: rgba(255, 255, 255, 0.5);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-top: 4px;
  }

  /* Timeline Details */
  .timeline-details {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .timeline-card {
    background-color: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    padding: 16px;
  }

  .timeline-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
  }

  .timeline-title {
    display: flex;
    align-items: center;
    gap: 8px;
    color: #f8fafc;
  }

  .playing-indicator {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 6px;
    background-color: rgba(34, 197, 94, 0.1);
    color: #4ade80;
    border: 1px solid rgba(34, 197, 94, 0.2);
    border-radius: 3px;
    font-size: 10px;
    font-weight: 500;
  }

  .timeline-stores {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 16px;
  }

  .store-section {
    background-color: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    padding: 12px;
  }

  .store-title {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.8);
    margin-bottom: 8px;
  }

  .store-summary {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 8px;
  }

  .summary-item {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.7);
  }

  .summary-item strong {
    color: rgba(255, 255, 255, 0.9);
  }

  .store-details {
    margin-top: 8px;
  }

  .details-summary {
    cursor: pointer;
    font-size: 11px;
    color: rgba(59, 130, 246, 0.8);
    padding: 4px 0;
  }

  .details-summary:hover {
    color: #60a5fa;
  }

  .no-data {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.4);
    font-style: italic;
  }

  /* Raw Data Section */
  .raw-data-section {
    margin-top: 20px;
  }

  .raw-data-section h4 {
    color: #f8fafc;
    margin-bottom: 16px;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .raw-stores {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 16px;
  }

  .raw-store {
    background-color: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    padding: 12px;
  }

  .raw-store-title {
    font-size: 13px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.8);
    margin-bottom: 8px;
  }

  /* Empty State */
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

  .empty-state h4 {
    margin: 0 0 8px 0;
    color: rgba(255, 255, 255, 0.7);
  }

  .empty-state p {
    margin: 0;
    font-size: 14px;
  }

  /* Button Styles */
  .btn {
    padding: 6px 12px;
    border-radius: 4px;
    border: 1px solid transparent;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-secondary {
    background-color: rgba(107, 114, 128, 0.1);
    color: #d1d5db;
    border-color: rgba(107, 114, 128, 0.2);
  }

  .btn-secondary:hover:not(:disabled) {
    background-color: rgba(107, 114, 128, 0.2);
  }

  .btn-success {
    background-color: rgba(34, 197, 94, 0.1);
    color: #4ade80;
    border-color: rgba(34, 197, 94, 0.2);
  }

  .btn-success:hover:not(:disabled) {
    background-color: rgba(34, 197, 94, 0.2);
  }

  .btn-danger {
    background-color: rgba(239, 68, 68, 0.1);
    color: #f87171;
    border-color: rgba(239, 68, 68, 0.2);
  }

  .btn-danger:hover:not(:disabled) {
    background-color: rgba(239, 68, 68, 0.2);
  }

  .btn-outline-danger {
    background-color: transparent;
    color: #f87171;
    border-color: rgba(239, 68, 68, 0.3);
  }

  .btn-outline-danger:hover:not(:disabled) {
    background-color: rgba(239, 68, 68, 0.1);
  }
</style>
