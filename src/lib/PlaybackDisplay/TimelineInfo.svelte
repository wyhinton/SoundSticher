<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { appState, currentOperationSources, getAllFiles } from '../state/state.svelte';
  import type { TimelineViewer } from '../state/timeline/TimelineViewer';
  import { formatMilliseconds } from '../utils/format';
  import TimeDisplay from './TimeDisplay.svelte';
  import {
    exportState,
    calculateEstimatedFileSize,
    type EstimatedFileSize,
  } from '$lib/state/export';

  // TimelineViewer prop (or null if not available)
  export let timelineViewer: TimelineViewer | null = null;

  let bufferingProgress = 0;

  // Timeline-specific information
  $: timelineId = timelineViewer?.id ?? '';
  $: sourceStore = timelineViewer?.source;
  $: timelineSource = sourceStore ? $sourceStore : null;
  $: waveformStateStore = timelineViewer?.waveformState;
  $: waveformState = waveformStateStore ? $waveformStateStore : null;
  $: timelineDuration = waveformState?.totalDuration ?? 0;
  $: timelineFilesCount = waveformState?.filePaths?.length ?? 0;
  $: timelineLoadedWaveforms = waveformState?.waveforms?.size ?? 0;

  // Calculate estimated file size for this timeline specifically
  $: estimatedFileSize = calculateEstimatedFileSize($exportState.settings, timelineDuration);
</script>

{#snippet infoItem(label: string, value: string, skeleton: boolean = false)}
  <div class="info-item">
    <span class="info-label">{label}:</span>
    <span class="info-value" class:skeleton>
      {value}
    </span>
  </div>
{/snippet}

<!-- Info Panel -->
<div class="info-panel d-flex justify-content-between align-items-center px-2 py-1">
  <div class="d-flex gap-3 align-items-center">
    {@render infoItem('Timeline ID', timelineId)}
    {#if timelineSource?.kind === 'operation'}
      {@render infoItem('Operation ID', timelineSource.operationId)}
    {/if}
    {@render infoItem(
      'Duration',
      timelineDuration ? formatMilliseconds(timelineDuration * 1000) : '0:00.000'
    )}
    {@render infoItem('Files', `${timelineLoadedWaveforms}/${timelineFilesCount}`)}
    {@render infoItem('Est. Size', estimatedFileSize.formatted)}
  </div>
</div>

<style>
  .info-panel {
    background: #2d3748;
    border: 1px solid #1a252f;
    border-top: none;
    border-radius: 0 0 4px 4px;
    font-size: 11px;
  }

  .info-item {
    display: flex;
    gap: 4px;
    align-items: center;
  }

  .info-label {
    color: #a0aec0;
    font-weight: 500;
  }

  .info-value {
    color: #e2e8f0;
    font-family: 'Courier New', monospace;
    font-weight: 600;
  }

  .skeleton {
    background: linear-gradient(90deg, #e9ecef 25%, #f8f9fa 50%, #e9ecef 75%);
    background-size: 200% 100%;
    animation: loading 1.5s infinite;
    border-radius: 2px;
  }

  @keyframes loading {
    0% {
      background-position: 200% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }
</style>
