<script lang="ts">
  import {
    timelinesStore,
    createTimelineStateForOp,
    toggleTimelineVisibilityByOpId,
    type Timeline,
    operationTimelines,
  } from '$lib/state/timeline/timelines';
  import { appState } from '$lib/state/state.svelte';
  import PrismWrapper from '$lib/components/Shared/PrismWrapper.svelte';
  // Reactive stores
  $: timelinesState = $timelinesStore;

  // Computed data for display
  $: timelineCount = Object.keys($timelinesStore.timelines).length;

  // Timeline summary data
  $: timelinesSummary = Object.values(timelinesState.timelines).map((timeline: Timeline) => ({
    id: timeline.id,
    sourceKind: timeline.source.kind,
    isActive: false, // No active timeline concept in simplified version
    zoom: timeline.view.zoom,
    scrollX: timeline.view.scrollX,
    playheadTime: timeline.view.playheadTime,
    hasSelection: !!timeline.view.selection,
    visibleTracksCount: timeline.view.visibleTracks.length,
    isDocked: false, // No layout concept in simplified version
    isFloating: false, // No layout concept in simplified version
    // Add source-specific info
    ...(timeline.source.kind === 'operation' ? { operationId: timeline.source.operationId } : {}),
    ...(timeline.source.kind === 'audioFile' ? { fileId: timeline.source.fileId } : {}),
    ...(timeline.source.kind === 'comparison'
      ? {
          comparisonA: timeline.source.a,
          comparisonB: timeline.source.b,
        }
      : {}),
  }));

  // Interaction functions
  function createTestTimeline() {
    const selectedOperationId = $appState.uiSettings?.selectedOperationId;
    if (selectedOperationId) {
      const newTimeline = createTimelineStateForOp(selectedOperationId);
      timelinesStore.update(state => ({
        ...state,
        timelines: {
          ...state.timelines,
          [newTimeline.id]: newTimeline,
        },
      }));
    }
  }

  function clearAllTimelines() {
    timelinesStore.set({
      timelines: {},
    });
  }

  function toggleTimelineForCurrentOp() {
    const selectedOperationId = $appState.uiSettings?.selectedOperationId;
    if (selectedOperationId) {
      toggleTimelineVisibilityByOpId(selectedOperationId);
    }
  }

  //   // Layout manipulation
  //   function moveToFloating(timelineId: string) {
  //     timelinesStore.update(state => {
  //       const layout = { ...state.layout };
  //       layout.docked = layout.docked.filter(id => id !== timelineId);
  //       if (!layout.floating.includes(timelineId)) {
  //         layout.floating.push(timelineId);
  //       }
  //       return { ...state, layout };
  //     });
  //   }

  //   function moveToDocked(timelineId: string) {
  //     timelinesStore.update(state => {
  //       const layout = { ...state.layout };
  //       layout.floating = layout.floating.filter(id => id !== timelineId);
  //       if (!layout.docked.includes(timelineId)) {
  //         layout.docked.push(timelineId);
  //       }
  //       return { ...state, layout };
  //     });
  //   }

  // View state manipulation
  function resetViewState(timelineId: string) {
    timelinesStore.update(state => {
      const timeline = state.timelines[timelineId];
      if (!timeline) return state;

      return {
        ...state,
        timelines: {
          ...state.timelines,
          [timelineId]: {
            ...timeline,
            view: {
              zoom: 1,
              scrollX: 0,
              playheadTime: 0,
              visibleTracks: [],
              selection: undefined,
            },
          },
        },
      };
    });
  }
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

<div class="timeline-store-debug">
  <!-- Controls -->
  <div class="controls-section">
    <div class="button-group">
      <span class="group-label">Timeline Management</span>
      {@render actionButton(
        createTestTimeline,
        'fa-plus',
        'Create Test Timeline',
        false,
        'primary'
      )}

      {@render actionButton(clearAllTimelines, 'fa-trash', 'Clear All', false, 'danger')}
    </div>
  </div>

  <!-- Timelines List -->
  <div class="timelines-section">
    <h4><i class="fa fa-list"></i> Timelines ({timelineCount})</h4>

    {#if timelineCount === 0}
      <div class="empty-state">
        <i class="fa fa-clock"></i>
        <p>No timelines created yet</p>
        <small>Create a timeline to see it here</small>
      </div>
    {:else}
      <div class="timeline-table-container">
        <table class="timeline-table">
          <thead>
            <tr>
              <th>ID</th>
              <th>Source</th>
              <th>Status</th>
              <th>Layout</th>
              <th>View</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {#each timelinesSummary as timeline}
              <tr class:active-timeline={timeline.isActive}>
                <td class="timeline-id">
                  <code class="id-code">{timeline.id.slice(0, 12)}...</code>
                </td>
                <td class="source-info">
                  <div class="source-kind">
                    <i
                      class="fa fa-{timeline.sourceKind === 'operation'
                        ? 'cogs'
                        : timeline.sourceKind === 'audioFile'
                          ? 'file-audio'
                          : timeline.sourceKind === 'comparison'
                            ? 'balance-scale'
                            : 'tools'}"
                    ></i>
                    {timeline.sourceKind}
                  </div>
                  {#if timeline.operationId}
                    <div class="source-detail">
                      Op: <code>{timeline.operationId.slice(0, 8)}...</code>
                    </div>
                  {:else if timeline.fileId}
                    <div class="source-detail">
                      File: <code>{timeline.fileId.slice(0, 8)}...</code>
                    </div>
                  {:else if timeline.comparisonA && timeline.comparisonB}
                    <div class="source-detail">A vs B</div>
                  {/if}
                </td>
                <td class="status-info">
                  {#if timeline.isActive}
                    <span class="status-badge active">
                      <i class="fa fa-star"></i> Active
                    </span>
                  {:else}
                    <span class="status-badge inactive">Inactive</span>
                  {/if}
                </td>
                <td class="layout-info">
                  {#if timeline.isDocked}
                    <span class="layout-badge docked">
                      <i class="fa fa-anchor"></i> Docked
                    </span>
                  {:else if timeline.isFloating}
                    <span class="layout-badge floating">
                      <i class="fa fa-window-maximize"></i> Floating
                    </span>
                  {:else}
                    <span class="layout-badge unknown">Unknown</span>
                  {/if}
                </td>
                <td class="view-info">
                  <div class="view-details">
                    <div class="view-item">
                      <span class="view-label">Zoom:</span>
                      <span class="view-value">{timeline.zoom.toFixed(2)}x</span>
                    </div>
                    <div class="view-item">
                      <span class="view-label">Scroll:</span>
                      <span class="view-value">{timeline.scrollX.toFixed(0)}px</span>
                    </div>
                    <div class="view-item">
                      <span class="view-label">Playhead:</span>
                      <span class="view-value">{timeline.playheadTime.toFixed(2)}s</span>
                    </div>
                    <div class="view-item">
                      <span class="view-label">Tracks:</span>
                      <span class="view-value">{timeline.visibleTracksCount}</span>
                    </div>
                    {#if timeline.hasSelection}
                      <div class="view-item">
                        <span class="view-label">Selection:</span>
                        <span class="view-value selection">Yes</span>
                      </div>
                    {/if}
                  </div>
                </td>
                <td class="actions">
                  <div class="action-buttons">
                    <!-- {#if !timeline.isActive}
                      <button
                        class="btn-micro btn-primary"
                        on:click={() => setAsActive(timeline.id)}
                      >
                        <i class="fa fa-star"></i>
                      </button>
                    {/if} -->

                    <!-- {#if timeline.isFloating}
                      <button
                        class="btn-micro btn-secondary"
                        on:click={() => moveToDocked(timeline.id)}
                      >
                        <i class="fa fa-anchor"></i>
                      </button>
                    {:else}
                      <button
                        class="btn-micro btn-secondary"
                        on:click={() => moveToFloating(timeline.id)}
                      >
                        <i class="fa fa-window-maximize"></i>
                      </button>
                    {/if} -->

                    <button class="btn-micro btn-info" on:click={() => resetViewState(timeline.id)}>
                      <i class="fa fa-refresh"></i>
                    </button>
                    <PrismWrapper data={$operationTimelines}></PrismWrapper>
                  </div>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </div>

  <!-- Active Timeline Details -->

  <!-- Full Store State -->
  <div class="store-state-section">
    <h4><i class="fa fa-database"></i> Full Store State</h4>
    <PrismWrapper data={timelinesState} maxHeight="400px" fontSize="11px" />
  </div>
</div>

<style>
  .timeline-store-debug {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  /* Stats Section */
  .stats-section {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 16px;
  }

  .stat-card {
    background-color: rgba(255, 255, 255, 0.05);
    padding: 12px;
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    text-align: center;
  }

  .stat-label {
    font-size: 11px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.6);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 6px;
  }

  .stat-value {
    font-size: 18px;
    font-weight: 700;
    color: #60a5fa;
    font-family: 'Courier New', monospace;
  }

  /* Controls Section */
  .controls-section {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    padding: 16px;
    background-color: rgba(255, 255, 255, 0.05);
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.1);
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

  /* Timelines Section */
  .timelines-section h4 {
    margin: 0 0 16px 0;
    color: #60a5fa;
    font-size: 16px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .timeline-table-container {
    background-color: rgba(255, 255, 255, 0.02);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    overflow: hidden;
  }

  .timeline-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 11px;
  }

  .timeline-table th {
    background-color: rgba(96, 165, 250, 0.2);
    color: #60a5fa;
    padding: 12px 8px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    text-align: left;
  }

  .timeline-table td {
    padding: 8px;
    color: rgba(255, 255, 255, 0.8);
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    vertical-align: top;
  }

  .timeline-table tr:hover {
    background-color: rgba(255, 255, 255, 0.05);
  }

  .timeline-table tr.active-timeline {
    background-color: rgba(245, 158, 11, 0.1);
    border-left: 3px solid #f59e0b;
  }

  .timeline-id .id-code {
    background-color: rgba(156, 163, 175, 0.1);
    color: #9ca3af;
    padding: 2px 4px;
    border-radius: 3px;
    font-size: 9px;
    border: 1px solid rgba(156, 163, 175, 0.2);
  }

  .source-info {
    min-width: 120px;
  }

  .source-kind {
    display: flex;
    align-items: center;
    gap: 6px;
    font-weight: 500;
    margin-bottom: 4px;
  }

  .source-detail {
    font-size: 10px;
    color: rgba(255, 255, 255, 0.6);
  }

  .source-detail code {
    background-color: rgba(59, 130, 246, 0.1);
    color: #60a5fa;
    padding: 1px 3px;
    border-radius: 2px;
    font-size: 9px;
  }

  .status-badge,
  .layout-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 3px 8px;
    border-radius: 12px;
    font-size: 9px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .status-badge.active {
    background-color: rgba(34, 197, 94, 0.2);
    color: #4ade80;
  }

  .status-badge.inactive {
    background-color: rgba(156, 163, 175, 0.2);
    color: #9ca3af;
  }

  .layout-badge.docked {
    background-color: rgba(59, 130, 246, 0.2);
    color: #60a5fa;
  }

  .layout-badge.floating {
    background-color: rgba(245, 158, 11, 0.2);
    color: #fbbf24;
  }

  .layout-badge.unknown {
    background-color: rgba(239, 68, 68, 0.2);
    color: #f87171;
  }

  .view-details {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 140px;
  }

  .view-item {
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }

  .view-label {
    font-size: 9px;
    color: rgba(255, 255, 255, 0.5);
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .view-value {
    font-family: 'Courier New', monospace;
    font-size: 9px;
    color: rgba(255, 255, 255, 0.8);
  }

  .view-value.selection {
    color: #4ade80;
    font-weight: 600;
  }

  .action-buttons {
    display: flex;
    gap: 4px;
  }

  .btn-micro {
    padding: 2px 6px;
    font-size: 9px;
    border-radius: 3px;
    background-color: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    color: rgba(255, 255, 255, 0.8);
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .btn-micro:hover {
    background-color: rgba(255, 255, 255, 0.2);
  }

  .btn-micro.btn-primary {
    background-color: rgba(59, 130, 246, 0.2);
    border-color: rgba(59, 130, 246, 0.5);
    color: #60a5fa;
  }

  .btn-micro.btn-secondary {
    background-color: rgba(107, 114, 128, 0.2);
    border-color: rgba(107, 114, 128, 0.5);
    color: #9ca3af;
  }

  .btn-micro.btn-info {
    background-color: rgba(14, 165, 233, 0.2);
    border-color: rgba(14, 165, 233, 0.5);
    color: #38bdf8;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 60px 20px;
    color: rgba(156, 163, 175, 0.6);
    text-align: center;
    background-color: rgba(255, 255, 255, 0.02);
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.05);
  }

  .empty-state i {
    font-size: 48px;
    margin-bottom: 16px;
    opacity: 0.3;
  }

  .empty-state p {
    margin: 0 0 8px 0;
    font-style: italic;
  }

  .empty-state small {
    color: rgba(156, 163, 175, 0.4);
  }

  /* Other sections */
  .active-timeline-section h4,
  .store-state-section h4 {
    margin: 0 0 16px 0;
    color: #f59e0b;
    font-size: 16px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 8px;
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
    cursor: pointer;
  }

  .btn-primary {
    background-color: rgba(59, 130, 246, 0.2);
    border-color: rgba(59, 130, 246, 0.5) !important;
    color: #60a5fa;
  }

  .btn-primary:hover {
    background-color: rgba(59, 130, 246, 0.3);
    border-color: rgba(59, 130, 246, 0.7) !important;
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

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
