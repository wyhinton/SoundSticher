<script lang="ts">
  import { getDisplayName, getItemSize, isItemActive } from '../../utils/timelineHelpers';
  import {
    durationSeconds,
    type TimelineItem,
    appState,
    toggleShowFullSvgPath,
  } from '../../state/state.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { onMount, onDestroy } from 'svelte';

  // Props passed from Timeline component
  export let isDragging: boolean;
  export let draggedSegmentIndex: number;
  export let dropIndicatorIndex: number;
  export let dropIndicatorX: number;
  export let width: number;
  export let scaleX: number;
  export let playHeadPosition: number;
  export let currentTransform: d3.ZoomTransform;
  export let timelineItems: TimelineItem[] = [];
  export let originalPathWidth: number;
  export let selectedSegments: Set<number>;
  export let lastSelectedIndex: number | null;
  export let segmentsToMove: number[] = [];

  // Reactive access to showFullSvgPath setting
  $: showFullSvgPath = $appState.uiSettings?.showFullSvgPath ?? false;

  // OpPlaybackState debugging
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

  let opPlaybackState: OpPlaybackStateDebugInfo | null = null;
  let opPlaybackError: string | null = null;
  let refreshInterval: number;

  async function fetchOpPlaybackState() {
    try {
      const state = await invoke<OpPlaybackStateDebugInfo>('get_op_playback_state');
      opPlaybackState = state;
      opPlaybackError = null;
    } catch (err) {
      opPlaybackError = `Error: ${err}`;
      console.error('Failed to fetch op playback state:', err);
    }
  }

  onMount(() => {
    // Start 200ms refresh cycle
    refreshInterval = setInterval(fetchOpPlaybackState, 200);
    fetchOpPlaybackState(); // Initial fetch
  });

  onDestroy(() => {
    if (refreshInterval) {
      clearInterval(refreshInterval);
    }
  });
</script>

<div class="debug">
  <div class="controls-row">
    <label>
      <input type="checkbox" checked={showFullSvgPath} on:change={() => toggleShowFullSvgPath()} />
      Show full SVG path
    </label>
  </div>
  <div>
    <b>Drag:</b>
    {isDragging} |
    <b>Segment:</b>
    {draggedSegmentIndex >= 0 ? draggedSegmentIndex : '-'} |
    <b>Drop:</b>
    {dropIndicatorIndex >= 0 ? dropIndicatorIndex : '-'} |
    <b>DropX:</b>
    {dropIndicatorX.toFixed(0)}px
  </div>
  <div>
    <b>W:</b>
    {width} |
    <b>ScaleX:</b>
    {scaleX.toFixed(2)} |
    <b>Dur:</b>
    {$durationSeconds.toFixed(1)}s |
    <b>PlayPos:</b>
    {playHeadPosition.toFixed(1)}s |
    <b>Zoom:</b>
    {currentTransform.k.toFixed(1)}x |
    <b>Pan:</b>
    {currentTransform.x.toFixed(0)}px
  </div>
  <div>
    <b>Selected ({selectedSegments.size}):</b>
    {#if selectedSegments.size > 0}
      [{Array.from(selectedSegments).join(', ')}] |
    {:else}
      none |
    {/if}
    <b>LastSel:</b>
    {lastSelectedIndex !== null ? lastSelectedIndex : '-'} |
    <b>ToMove:</b>
    {#if segmentsToMove.length > 0}
      [{segmentsToMove.join(', ')}]
    {:else}
      none
    {/if}
  </div>

  <!-- OpPlaybackState Debug Info -->
  <div class="op-playback-section">
    <b>Op Playback State:</b>
    {#if opPlaybackError}
      <span style="color: red;">{opPlaybackError}</span>
    {:else if opPlaybackState}
      <div class="op-playback-info">
        <div>
          <b>Sessions:</b>
          {opPlaybackState.totalSessions} |
          <b>Active:</b>
          {opPlaybackState.activeTimeline || 'None'} |
          <b>Playing:</b>
          {opPlaybackState.isPlaying ? 'Yes' : 'No'} |
          <b>Paused:</b>
          {opPlaybackState.isPaused ? 'Yes' : 'No'}
        </div>
        {#if opPlaybackState.activeTimeline && opPlaybackState.sessions[opPlaybackState.activeTimeline]}
          {@const activeSession = opPlaybackState.sessions[opPlaybackState.activeTimeline]}
          <div class="active-session-info">
            <b>Active Session:</b>
            Progress: {(activeSession.progress * 100).toFixed(1)}% | Seek: {activeSession.seekSeconds.toFixed(
              2
            )}s | Duration: {activeSession.durationSeconds.toFixed(2)}s | Ops: {activeSession.operationCount}
            | Loop: {activeSession.loopPlayback ? 'On' : 'Off'}
          </div>
        {/if}
      </div>
    {:else}
      <span style="color: yellow;">Loading...</span>
    {/if}
  </div>

  {#if timelineItems && timelineItems.length > 0}
    <div><b>Items ({timelineItems.length}):</b></div>
    {#each timelineItems as item, i}
      <div
        class="item d-flex gap-2"
        class:dragged={i === draggedSegmentIndex}
        class:selected={selectedSegments.has(i)}
      >
        <b>#{i}</b>
        <span class="item-type">[{item.kind}]</span>
        {getDisplayName(item)} | Start: {(item.startOffset * 100).toFixed(1)}% | Size: {(
          getItemSize(item) * 100
        ).toFixed(1)}% | Active: {isItemActive(item)} | X: {(
          item.startOffset *
          originalPathWidth *
          scaleX
        ).toFixed(0)}-{(
          (item.startOffset + getItemSize(item)) *
          originalPathWidth *
          scaleX
        ).toFixed(0)}px
        {#if item.kind === 'sample' || item.kind === 'merge'}
          <div>
            {#if item.svgPath}
              {#if showFullSvgPath}
                <div class="svg-path">{item.svgPath}</div>
              {:else}
                <span class="svg-path-short">
                  SVG: {item.svgPath.length} chars
                  {#if item.svgPath.length > 50}
                    - {item.svgPath.substring(0, 50)}...
                  {:else}
                    - {item.svgPath}
                  {/if}
                </span>
              {/if}
            {:else}
              <span style="color: red;">undefined</span>
            {/if}
          </div>
        {/if}
      </div>
    {/each}
  {:else}
    <div>No items</div>
  {/if}
</div>

<style>
  /* Debug Panel Styles */
  .debug {
    margin-top: 10px;
    background: #000;
    border: 1px solid #333;
    padding: 8px;
    font-family: monospace;
    font-size: 11px;
    color: #fff;
  }

  .debug div {
    margin: 2px 0;
  }

  .controls-row {
    margin: 4px 0;
    padding: 4px 0;
    border-bottom: 1px solid #333;
  }

  .controls-row label {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
    color: #ccc;
    cursor: pointer;
  }

  .controls-row input[type='checkbox'] {
    margin: 0;
  }

  .debug .item {
    padding: 1px 2px;
    border-radius: 2px;
  }

  .debug .item.dragged {
    background: #333;
    color: #ff0;
  }

  .debug .item.selected {
    background: #006600;
    color: #00ff00;
  }

  .debug .item.dragged.selected {
    background: #663300;
    color: #ffaa00;
  }

  .debug .item-type {
    color: #888;
    font-style: italic;
  }

  .svg-path {
    color: #88ddff;
    font-size: 10px;
    word-break: break-all;
    margin-top: 2px;
    padding: 2px;
    background: #111;
    border-radius: 2px;
  }

  .svg-path-short {
    color: #88ddff;
    font-size: 10px;
  }

  .op-playback-section {
    margin: 4px 0;
    padding: 4px 0;
    border-top: 1px solid #444;
  }

  .op-playback-info {
    margin-left: 8px;
    font-size: 10px;
  }

  .active-session-info {
    margin-top: 2px;
    margin-left: 8px;
    color: #4ade80;
    font-size: 10px;
  }
</style>
