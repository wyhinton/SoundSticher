<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount, onDestroy } from 'svelte';
  import PrismWrapper from '$lib/components/Shared/PrismWrapper.svelte';

  interface AudioSpecDebugInfo {
    sampleRate: number;
    channels: number;
  }

  interface PlaybackSessionDebugInfo {
    durationSeconds: number;
    progress: number;
    seekSeconds: number;
    loopPlayback: boolean;
    operationNames: string[];
    operationCount: number;
    spec: AudioSpecDebugInfo;
  }

  interface OpPlaybackStateDebugInfo {
    sessions: { [key: string]: PlaybackSessionDebugInfo };
    activeTimeline: string | null;
    isPlaying: boolean;
    isPaused: boolean;
    totalSessions: number;
  }

  let playbackState: OpPlaybackStateDebugInfo | null = null;
  let error: string | null = null;
  let isRefreshing = true;
  let refreshInterval: number;
  let lastUpdate = 0;
  let sessionHistory: Array<{ timestamp: number; sessionIds: string[] }> = [];

  async function fetchPlaybackState() {
    if (!isRefreshing) return;

    try {
      const state = await invoke<OpPlaybackStateDebugInfo>('get_op_playback_state');
      playbackState = state;
      error = null;
      lastUpdate = Date.now();

      // Track session history for debugging "global" timeline appearance
      const currentSessionIds = Object.keys(state.sessions);
      const lastEntry = sessionHistory[sessionHistory.length - 1];
      if (
        !lastEntry ||
        JSON.stringify(currentSessionIds.sort()) !== JSON.stringify(lastEntry.sessionIds.sort())
      ) {
        sessionHistory = [
          ...sessionHistory.slice(-9),
          { timestamp: lastUpdate, sessionIds: currentSessionIds },
        ];

        // Log warning if "global" timeline is present
        if (state.sessions['global']) {
          console.warn('⚠️ WARNING: "global" timeline session detected in playback state!', {
            timestamp: new Date(lastUpdate).toISOString(),
            globalSession: state.sessions['global'],
            allSessions: Object.keys(state.sessions),
          });
        }
      }
    } catch (err) {
      error = `Failed to fetch playback state: ${err}`;
      console.error('Failed to fetch playback state:', err);
    }
  }

  function toggleRefresh() {
    isRefreshing = !isRefreshing;
    if (isRefreshing) {
      startRefresh();
    } else {
      stopRefresh();
    }
  }

  function startRefresh() {
    if (refreshInterval) {
      clearInterval(refreshInterval);
    }
    refreshInterval = setInterval(fetchPlaybackState, 100); // 100ms refresh
    fetchPlaybackState(); // Initial fetch
  }

  function stopRefresh() {
    if (refreshInterval) {
      clearInterval(refreshInterval);
      refreshInterval = 0;
    }
  }

  function formatTimestamp(timestamp: number): string {
    return new Date(timestamp).toLocaleTimeString();
  }

  onMount(() => {
    if (isRefreshing) {
      startRefresh();
    }
  });

  onDestroy(() => {
    stopRefresh();
  });

  // Computed values for display
  $: hasActiveSessions = playbackState && playbackState.totalSessions > 0;
  $: activeSession = playbackState?.activeTimeline
    ? playbackState.sessions[playbackState.activeTimeline]
    : null;
</script>

<div class="op-playback-debug">
  <div class="controls-header">
    <h3><i class="fa fa-play-circle"></i> Operation Playback State</h3>
    <div class="controls">
      <button
        class="btn btn-sm {isRefreshing ? 'btn-danger' : 'btn-success'}"
        on:click={toggleRefresh}
      >
        <i class="fa {isRefreshing ? 'fa-pause' : 'fa-play'}"></i>
        {isRefreshing ? 'Pause' : 'Resume'} Auto-refresh (100ms)
      </button>

      <button
        class="btn btn-sm btn-secondary"
        on:click={fetchPlaybackState}
        disabled={isRefreshing}
      >
        <i class="fa fa-refresh"></i>
        Manual Refresh
      </button>
    </div>
  </div>

  {#if error}
    <div class="error-message">
      <i class="fa fa-exclamation-triangle"></i>
      {error}
    </div>
  {/if}

  {#if lastUpdate > 0}
    <div class="last-update">
      Last updated: {formatTimestamp(lastUpdate)}
    </div>
  {/if}

  {#if playbackState}
    <!-- Global State Overview -->
    <div class="state-overview">
      <div class="overview-grid">
        <div class="overview-card">
          <div class="card-label">Total Sessions</div>
          <div class="card-value">{playbackState.totalSessions}</div>
        </div>

        <div class="overview-card">
          <div class="card-label">Is Playing</div>
          <div class="card-value {playbackState.isPlaying ? 'status-active' : 'status-inactive'}">
            <i class="fa {playbackState.isPlaying ? 'fa-play' : 'fa-stop'}"></i>
            {playbackState.isPlaying ? 'Yes' : 'No'}
          </div>
        </div>

        <div class="overview-card">
          <div class="card-label">Is Paused</div>
          <div class="card-value {playbackState.isPaused ? 'status-warning' : 'status-inactive'}">
            <i class="fa {playbackState.isPaused ? 'fa-pause' : 'fa-play'}"></i>
            {playbackState.isPaused ? 'Yes' : 'No'}
          </div>
        </div>

        <div class="overview-card">
          <div class="card-label">Active Timeline</div>
          <div
            class="card-value {playbackState.activeTimeline ? 'status-active' : 'status-inactive'}"
          >
            {playbackState.activeTimeline || 'None'}
          </div>
        </div>
      </div>
    </div>

    <!-- Active Session Details -->
    {#if activeSession}
      <div class="active-session">
        <h4><i class="fa fa-volume-up"></i> Active Session: {playbackState.activeTimeline}</h4>
        <div class="session-details">
          <div class="detail-grid">
            <div class="detail-item">
              <span class="detail-label">Duration:</span>
              <span class="detail-value">{activeSession.durationSeconds.toFixed(2)}s</span>
            </div>
            <div class="detail-item">
              <span class="detail-label">Progress:</span>
              <span class="detail-value">{(activeSession.progress * 100).toFixed(1)}%</span>
            </div>
            <div class="detail-item">
              <span class="detail-label">Seek Position:</span>
              <span class="detail-value">{activeSession.seekSeconds.toFixed(2)}s</span>
            </div>
            <div class="detail-item">
              <span class="detail-label">Loop Mode:</span>
              <span
                class="detail-value {activeSession.loopPlayback
                  ? 'status-active'
                  : 'status-inactive'}"
              >
                <i class="fa {activeSession.loopPlayback ? 'fa-repeat' : 'fa-arrow-right'}"></i>
                {activeSession.loopPlayback ? 'Enabled' : 'Disabled'}
              </span>
            </div>
            <div class="detail-item">
              <span class="detail-label">Sample Rate:</span>
              <span class="detail-value">{activeSession.spec.sampleRate}Hz</span>
            </div>
            <div class="detail-item">
              <span class="detail-label">Channels:</span>
              <span class="detail-value">{activeSession.spec.channels}</span>
            </div>
          </div>

          <!-- Progress Bar -->
          <div class="progress-section">
            <div class="progress-label">Playback Progress</div>
            <div class="progress-bar">
              <div class="progress-fill" style="width: {activeSession.progress * 100}%"></div>
              <div class="progress-text">
                {activeSession.seekSeconds.toFixed(1)}s / {activeSession.durationSeconds.toFixed(
                  1
                )}s
              </div>
            </div>
          </div>

          <!-- Operations -->
          {#if activeSession.operationNames.length > 0}
            <div class="operations-section">
              <div class="operations-label">
                Operations ({activeSession.operationCount}):
              </div>
              <div class="operations-list">
                {#each activeSession.operationNames as operation, index}
                  <span class="operation-tag">
                    {index + 1}. {operation}
                  </span>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <!-- All Sessions -->
    {#if hasActiveSessions}
      <div class="all-sessions">
        <h4><i class="fa fa-list"></i> All Sessions ({playbackState.totalSessions})</h4>
        <div class="sessions-grid">
          {#each Object.entries(playbackState.sessions) as [timelineId, session]}
            <div
              class="session-card {timelineId === playbackState.activeTimeline
                ? 'active-session-card'
                : ''}"
            >
              <div class="session-header">
                <div class="session-title">
                  {timelineId}
                  {#if timelineId === playbackState.activeTimeline}
                    <i class="fa fa-volume-up active-indicator" title="Currently Active"></i>
                  {/if}
                </div>
              </div>

              <div class="session-stats">
                <div class="stat">
                  <span class="stat-label">Duration:</span>
                  <span class="stat-value">{session.durationSeconds.toFixed(1)}s</span>
                </div>
                <div class="stat">
                  <span class="stat-label">Progress:</span>
                  <span class="stat-value">{(session.progress * 100).toFixed(1)}%</span>
                </div>
                <div class="stat">
                  <span class="stat-label">Operations:</span>
                  <span class="stat-value">{session.operationCount}</span>
                </div>
                <div class="stat">
                  <span class="stat-label">Loop:</span>
                  <span
                    class="stat-value {session.loopPlayback ? 'status-active' : 'status-inactive'}"
                  >
                    {session.loopPlayback ? 'On' : 'Off'}
                  </span>
                </div>
              </div>

              <div class="session-progress">
                <div class="mini-progress-bar">
                  <div class="mini-progress-fill" style="width: {session.progress * 100}%"></div>
                </div>
              </div>
            </div>
          {/each}
        </div>
      </div>
    {:else}
      <div class="empty-state">
        <i class="fa fa-play-circle"></i>
        <p>No active playback sessions</p>
        <small class="text-muted">
          Create a timeline with operations to see playback state information here.
        </small>
      </div>
    {/if}

    <!-- Raw Data -->
    <div class="raw-data">
      <h4><i class="fa fa-code"></i> Raw Playback State</h4>
      <PrismWrapper data={playbackState} maxHeight="400px" fontSize="10px" />
    </div>

    <!-- Session History for Debugging -->
    {#if sessionHistory.length > 0}
      <div class="session-history">
        <h4><i class="fa fa-history"></i> Session History (Last 10 Updates)</h4>
        <div class="history-list">
          {#each sessionHistory as entry, idx}
            <div class="history-entry {entry.sessionIds.includes('global') ? 'has-global' : ''}">
              <span class="history-time">{new Date(entry.timestamp).toLocaleTimeString()}</span>
              <span class="history-count">Sessions: {entry.sessionIds.length}</span>
              <span class="history-ids">
                {entry.sessionIds.map(id => (id === 'global' ? `🚨 ${id}` : id)).join(', ')}
              </span>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  {:else}
    <div class="loading-state">
      <i class="fa fa-spinner fa-spin"></i>
      <p>Loading playback state...</p>
    </div>
  {/if}
</div>

<style>
  .op-playback-debug {
    display: flex;
    flex-direction: column;
    gap: 20px;
    color: white;
  }

  .controls-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
  }

  .controls-header h3 {
    margin: 0;
    color: #f59e0b;
    font-size: 18px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .controls {
    display: flex;
    gap: 8px;
  }

  .btn {
    border: 1px solid rgba(255, 255, 255, 0.3);
    background-color: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.9);
    transition: all 0.2s ease;
    font-size: 12px;
    padding: 6px 12px;
    border-radius: 4px;
    cursor: pointer;
  }

  .btn-success {
    background-color: rgba(34, 197, 94, 0.2);
    border-color: rgba(34, 197, 94, 0.5);
    color: #4ade80;
  }

  .btn-danger {
    background-color: rgba(239, 68, 68, 0.2);
    border-color: rgba(239, 68, 68, 0.5);
    color: #f87171;
  }

  .btn-secondary {
    background-color: rgba(107, 114, 128, 0.2);
    border-color: rgba(107, 114, 128, 0.5);
    color: #9ca3af;
  }

  .btn:hover:not(:disabled) {
    opacity: 0.8;
    transform: translateY(-1px);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .error-message {
    background-color: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: #f87171;
    padding: 12px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .last-update {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.6);
    text-align: right;
    font-style: italic;
  }

  .state-overview {
    background-color: rgba(255, 255, 255, 0.05);
    padding: 16px;
    border-radius: 8px;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .overview-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 16px;
  }

  .overview-card {
    background-color: rgba(255, 255, 255, 0.05);
    padding: 12px;
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    text-align: center;
  }

  .card-label {
    font-size: 11px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.6);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 8px;
  }

  .card-value {
    font-size: 16px;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
  }

  .status-active {
    color: #4ade80;
  }

  .status-inactive {
    color: rgba(255, 255, 255, 0.5);
  }

  .status-warning {
    color: #fbbf24;
  }

  .active-session {
    background-color: rgba(34, 197, 94, 0.1);
    padding: 16px;
    border-radius: 8px;
    border: 1px solid rgba(34, 197, 94, 0.3);
  }

  .active-session h4 {
    margin: 0 0 16px 0;
    color: #4ade80;
    font-size: 16px;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .session-details {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .detail-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 12px;
  }

  .detail-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px;
    background-color: rgba(255, 255, 255, 0.05);
    border-radius: 4px;
  }

  .detail-label {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.7);
    font-weight: 500;
  }

  .detail-value {
    font-size: 12px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .progress-section {
    margin-top: 8px;
  }

  .progress-label {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.7);
    margin-bottom: 8px;
    font-weight: 500;
  }

  .progress-bar {
    position: relative;
    height: 24px;
    background-color: rgba(255, 255, 255, 0.1);
    border-radius: 12px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, #4ade80, #22c55e);
    transition: width 0.1s ease;
  }

  .progress-text {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    font-size: 10px;
    font-weight: 600;
    color: white;
    text-shadow: 1px 1px 2px rgba(0, 0, 0, 0.7);
  }

  .operations-section {
    margin-top: 8px;
  }

  .operations-label {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.7);
    margin-bottom: 8px;
    font-weight: 500;
  }

  .operations-list {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .operation-tag {
    background-color: rgba(59, 130, 246, 0.2);
    color: #60a5fa;
    padding: 4px 8px;
    border-radius: 12px;
    font-size: 10px;
    font-weight: 500;
    border: 1px solid rgba(59, 130, 246, 0.3);
  }

  .all-sessions h4 {
    margin: 0 0 16px 0;
    color: #60a5fa;
    font-size: 16px;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .sessions-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 16px;
  }

  .session-card {
    background-color: rgba(255, 255, 255, 0.05);
    padding: 12px;
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    transition: transform 0.2s ease;
  }

  .session-card:hover {
    transform: translateY(-2px);
  }

  .active-session-card {
    background-color: rgba(34, 197, 94, 0.1);
    border-color: rgba(34, 197, 94, 0.3);
  }

  .session-header {
    margin-bottom: 12px;
  }

  .session-title {
    font-size: 14px;
    font-weight: 600;
    color: white;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .active-indicator {
    color: #4ade80;
    font-size: 12px;
  }

  .session-stats {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 12px;
  }

  .stat {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 11px;
  }

  .stat-label {
    color: rgba(255, 255, 255, 0.7);
  }

  .stat-value {
    font-weight: 600;
    color: white;
  }

  .session-progress {
    margin-top: 8px;
  }

  .mini-progress-bar {
    height: 4px;
    background-color: rgba(255, 255, 255, 0.1);
    border-radius: 2px;
    overflow: hidden;
  }

  .mini-progress-fill {
    height: 100%;
    background: linear-gradient(90deg, #60a5fa, #3b82f6);
    transition: width 0.1s ease;
  }

  .empty-state,
  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 60px 20px;
    color: rgba(156, 163, 175, 0.6);
    text-align: center;
  }

  .empty-state i,
  .loading-state i {
    font-size: 48px;
    margin-bottom: 16px;
    opacity: 0.3;
  }

  .empty-state p,
  .loading-state p {
    margin: 8px 0;
    font-style: italic;
  }

  .text-muted {
    color: rgba(255, 255, 255, 0.4);
    font-size: 12px;
  }

  .raw-data h4 {
    margin: 0 0 12px 0;
    color: #9ca3af;
    font-size: 16px;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .session-history {
    background-color: rgba(255, 255, 255, 0.03);
    padding: 12px;
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    margin-top: 16px;
  }

  .session-history h4 {
    margin: 0 0 8px 0;
    color: #d1d5db;
    font-size: 14px;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .history-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 200px;
    overflow-y: auto;
  }

  .history-entry {
    display: flex;
    gap: 12px;
    padding: 4px 6px;
    background-color: rgba(255, 255, 255, 0.03);
    border-left: 2px solid rgba(107, 114, 128, 0.3);
    font-size: 11px;
    border-radius: 2px;
  }

  .history-entry.has-global {
    background-color: rgba(239, 68, 68, 0.1);
    border-left-color: rgba(239, 68, 68, 0.5);
  }

  .history-time {
    color: rgba(156, 163, 175, 0.7);
    font-weight: 600;
    min-width: 80px;
  }

  .history-count {
    color: rgba(191, 191, 191, 0.7);
    font-weight: 500;
  }

  .history-ids {
    color: #9ca3af;
    flex: 1;
    word-break: break-all;
  }

  @media (max-width: 768px) {
    .overview-grid {
      grid-template-columns: repeat(2, 1fr);
    }

    .detail-grid {
      grid-template-columns: 1fr;
    }

    .sessions-grid {
      grid-template-columns: 1fr;
    }

    .controls-header {
      flex-direction: column;
      gap: 12px;
      align-items: stretch;
    }

    .controls {
      justify-content: center;
    }
  }
</style>
