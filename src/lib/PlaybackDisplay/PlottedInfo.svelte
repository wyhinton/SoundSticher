<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import Progress from '../Progress.svelte';
  import { type Timeline } from '../state/timeline/timelines';
  import {
    timelinePlaybackState,
    isTimelinePlaying,
    isTimelinePaused,
  } from '../state/timelinePlaybackService';
  import TimeDisplay from './TimeDisplay.svelte';
  import TimelineInfo from './TimelineInfo.svelte';
  import TransportControls from './TransportControls.svelte';
  import { timelinePlayhead } from '$lib/state/timeline/timelinePlaybackState';

  export let timeline: Timeline;

  // Use timeline-specific waveform state for progress/loading indicators
  $: waveformStateStore = timeline.waveformState;
  $: waveformState = waveformStateStore ? $waveformStateStore : null;
  $: isLoading = waveformState?.loading || waveformState?.loadingWaveforms || false;
  $: hasError = !!waveformState?.error;

  // Calculate progress based on loaded waveforms vs total files
  $: operationProgress = (() => {
    if (!waveformState || waveformState.filePaths.length === 0) return 1;
    const totalFiles = waveformState.filePaths.length;
    const loadedWaveforms = waveformState.waveforms.size;
    return loadedWaveforms / totalFiles;
  })();

  // Get timeline items to check if transport should be disabled
  $: timelineItemsStore = timeline.items;
  $: timelineItems = $timelineItemsStore;
  $: transportDisabled = (timelineItems?.length || 0) === 0;

  // Timeline-specific playback state for TimeDisplay
  $: playheadStore = timelinePlayhead(timeline.id);
  $: currentPositionSeconds = $playheadStore;
  $: totalDurationSeconds = waveformState?.totalDuration ?? 0;

  // Check if THIS timeline is the active one playing
  $: isThisTimelineActive = $timelinePlaybackState.activeTimelineId === timeline.id;
  $: isPlaying = isThisTimelineActive && $isTimelinePlaying;
  $: isPaused = isThisTimelineActive && $isTimelinePaused;
  $: isCurrentlyPlaying = isPlaying && !isPaused;
</script>

<div class="d-flex text-success">
  <!-- Timeline header with operation info -->
  <div class="timeline-header">
    <span class="timeline-operation-name">
      {timeline.source.kind === 'operation'
        ? `Operation: ${timeline.source.operationId}`
        : 'Timeline'}
    </span>
    {#if isLoading}
      <span class="loading-indicator">Loading...</span>
    {:else if hasError}
      <span class="error-indicator">Error: {waveformState?.error}</span>
    {/if}
  </div>

  <!-- Show progress bar while loading waveforms -->
  {#if isLoading && operationProgress < 1}
    <Progress value={operationProgress}></Progress>
  {/if}

  <div class="d-flex gap-1">
    <TransportControls disabled={transportDisabled} timelineId={timeline.id} />
    <TimeDisplay
      compact={true}
      {currentPositionSeconds}
      {totalDurationSeconds}
      isPlaying={isCurrentlyPlaying}
    />
  </div>
  <TimelineInfo></TimelineInfo>
</div>

<style>
  div {
    font-size: 12px;
  }

  .timeline-header {
    background-color: var(--bs-secondary-bg);
    border-bottom: 1px solid var(--bs-border-color);
    font-size: 14px;
  }

  .timeline-operation-name {
    font-weight: 500;
    color: var(--bs-secondary);
    font-size: 11px;
  }

  .loading-indicator {
    color: var(--bs-info);
    font-size: 10px;
    margin-left: 8px;
  }

  .error-indicator {
    color: var(--bs-danger);
    font-size: 10px;
    margin-left: 8px;
  }
</style>
