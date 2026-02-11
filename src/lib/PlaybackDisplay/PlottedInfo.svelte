<script lang="ts">
  import type { TimelineViewer } from '../state/timeline/TimelineViewer';
  import { isTimelinePaused, isTimelinePlaying } from '../state/timelinePlaybackService';
  import TimeDisplay from './TimeDisplay.svelte';
  import TimelineInfo from './TimelineInfo.svelte';
  import TransportControls from './TransportControls.svelte';
  import { appState } from '$lib/state/state.svelte';
  import {
    timelineLooping,
    timelinePlaybackState,
    timelinePlayhead,
  } from '$lib/state/timeline/timelinePlaybackState';
  import { setActiveTimeline } from '$lib/state/timeline/timelines';
  import timelinePlaybackService from '$lib/state/timelinePlaybackService';

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
  $: currentPositionSeconds = timelineViewer.playheadPositionSec;
  $: totalDurationSeconds = timelineViewer.totalDuration;

  // Check if THIS timeline is the active one playing
  $: isThisTimelineActive = $appState.timelines?.activeTimelineId === timelineViewer.id;
  $: isPlaying = isThisTimelineActive && $isTimelinePlaying;
  $: isPaused = isThisTimelineActive && $isTimelinePaused;
  $: isCurrentlyPlaying = isPlaying && !isPaused;

  // Get playhead position for play command
  $: playheadPositionStore = timelineViewer.playheadPositionSec;
  $: playHeadPosition = $playheadPositionStore;

  // Get duration for skip to end
  $: totalDuration = $totalDurationSeconds;

  // Get loop state for toggle loop
  $: loopingStore = timelineViewer.id ? timelineLooping(timelineViewer.id) : null;
  $: currentLoopState = loopingStore ? $loopingStore : false;

  // Transport event handlers
  async function handlePlay() {
    try {
      if (!timelineViewer.id) {
        console.error('No timeline ID - cannot play.');
        return;
      }
      setActiveTimeline(timelineViewer.id);
      await timelinePlaybackService.playTimeline(timelineViewer.id, playHeadPosition);
    } catch (error) {
      console.error('Error playing audio:', error);
    }
  }

  async function handlePause() {
    try {
      if (!timelineViewer.id) {
        console.error('No timeline ID - cannot pause.');
        return;
      }
      setActiveTimeline(timelineViewer.id);
      await timelinePlaybackService.pauseTimeline(timelineViewer.id);
    } catch (error) {
      console.error('Error pausing audio:', error);
    }
  }

  async function handleResume() {
    try {
      if (!timelineViewer.id) {
        console.error('No timeline ID - cannot resume.');
        return;
      }
      setActiveTimeline(timelineViewer.id);
      await timelinePlaybackService.resumeTimeline(timelineViewer.id);
    } catch (error) {
      console.error('Error resuming audio:', error);
    }
  }

  async function handleStop() {
    try {
      if (!timelineViewer.id) {
        console.error('No timeline ID - cannot stop.');
        return;
      }
      setActiveTimeline(timelineViewer.id);
      await timelinePlaybackService.stopTimeline(timelineViewer.id);
    } catch (error) {
      console.error('Error stopping audio:', error);
    }
  }

  async function handleSkipToStart() {
    try {
      if (!timelineViewer.id) {
        console.error('No timeline ID - cannot seek.');
        return;
      }
      setActiveTimeline(timelineViewer.id);
      await timelinePlaybackService.seekTimeline(timelineViewer.id, 0);
    } catch (error) {
      console.error('Error skipping to start:', error);
    }
  }

  async function handleSkipToEnd() {
    try {
      if (!timelineViewer.id) {
        console.error('No timeline ID - cannot seek.');
        return;
      }
      setActiveTimeline(timelineViewer.id);
      await timelinePlaybackService.seekTimeline(timelineViewer.id, totalDuration);
    } catch (error) {
      console.error('Error skipping to end:', error);
    }
  }

  async function handleToggleLoop() {
    try {
      if (!timelineViewer.id) {
        console.error('No timeline ID - cannot toggle loop.');
        return;
      }
      setActiveTimeline(timelineViewer.id);
      await timelinePlaybackService.setTimelineLoop(timelineViewer.id, !currentLoopState);
    } catch (error) {
      console.error('Error toggling loop:', error);
    }
  }
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
    <TransportControls
      disabled={transportDisabled}
      timelineId={timelineViewer.id}
      on:play={handlePlay}
      on:pause={handlePause}
      on:resume={handleResume}
      on:stop={handleStop}
      on:skipToStart={handleSkipToStart}
      on:skipToEnd={handleSkipToEnd}
      on:toggleLoop={handleToggleLoop}
    />
    <TimeDisplay
      compact={true}
      currentPositionSeconds={$currentPositionSeconds}
      totalDurationSeconds={$totalDurationSeconds}
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

  .error-indicator {
    color: var(--bs-danger);
    font-size: 10px;
    margin-left: 8px;
  }
</style>
