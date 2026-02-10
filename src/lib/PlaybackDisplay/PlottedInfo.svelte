<script lang="ts">
  import type { TimelineViewer } from '../state/timeline/TimelineViewer';
  import { isTimelinePaused, isTimelinePlaying } from '../state/timelinePlaybackService';
  import TimeDisplay from './TimeDisplay.svelte';
  import TimelineInfo from './TimelineInfo.svelte';
  import TransportControls from './TransportControls.svelte';
  import { appState } from '$lib/state/state.svelte';
  import {
    timelinePlaybackState,
    timelinePlayhead,
  } from '$lib/state/timeline/timelinePlaybackState';

  export let timelineViewer: TimelineViewer;

  // Use timeline-specific waveform state for progress/loading indicators
  $: waveformStateStore = timelineViewer.waveformState;
  $: waveformState = $waveformStateStore;
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
  $: timelineItemsStore = timelineViewer.items;
  $: timelineItems = $timelineItemsStore;
  $: transportDisabled = (timelineItems?.length || 0) === 0;

  // Timeline-specific playback state for TimeDisplay
  $: playheadStore = timelinePlayhead(timelineViewer.id);
  $: currentPositionSeconds = $playheadStore;
  $: totalDurationSeconds = waveformState?.totalDuration ?? 0;

  // Check if THIS timeline is the active one playing
  $: isThisTimelineActive = $appState.timelines?.activeTimelineId === timelineViewer.id;
  $: isPlaying = isThisTimelineActive && $isTimelinePlaying;
  $: isPaused = isThisTimelineActive && $isTimelinePaused;
  $: isCurrentlyPlaying = isPlaying && !isPaused;
</script>

<div class="d-flex text-success">
  <!-- Timeline header with operation info -->
  <div class="timeline-header">
    <span class="timeline-operation-name px-2">
      {$appState.operations?.defs?.[timelineViewer.operationId]?.name ?? 'Timeline'}
    </span>
    {#if isLoading}
      <!-- <span class="loading-indicator">Loading...</span> -->
    {:else if hasError}
      <span class="error-indicator">Error: {waveformState?.error}</span>
    {/if}
  </div>

  <!-- Show progress bar while loading waveforms -->
  <!-- {#if isLoading && operationProgress < 1}
    <Progress value={operationProgress}></Progress>
  {/if} -->

  <div class="d-flex">
    <TransportControls disabled={transportDisabled} timelineId={timelineViewer.id} />
    <TimeDisplay
      compact={true}
      {currentPositionSeconds}
      {totalDurationSeconds}
      isPlaying={isCurrentlyPlaying}
    />
  </div>
  <TimelineInfo {timelineViewer} />
</div>

<style>
  div {
    font-size: 12px;
  }

  .timeline-header {
    border-bottom: 1px solid var(--bs-border-color);
    font-size: 14px;
    background-color: black;
  }

  .timeline-operation-name {
    font-weight: 500;
    font-size: 14px;
    vertical-align: sub;
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
