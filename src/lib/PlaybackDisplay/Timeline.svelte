<script lang="ts">
  import { onMount } from 'svelte';
  import * as d3 from 'd3';
  import { createEventDispatcher } from 'svelte';
  import {
    appState,
    getAllFiles,
    triggerFileAnimation,
    applySyncIndexes,
    durationSeconds,
  } from '../state/state.svelte';
  import { listen, TauriEvent } from '@tauri-apps/api/event';
  import { formatFileName } from '../utils/format';
  import {
    getDisplayName,
    getItemSize,
    isItemActive,
    canItemBeDragged,
    getItemColor,
    isAudioFileItem,
  } from '../utils/timelineHelpers';
  import TimelineSegment from './Timeline/TimelineSegment.svelte';
  import LabelLayer from './Timeline/LabelLayer.svelte';
  import Playhead from './Timeline/Playhead.svelte';
  import DropIndicator from './Timeline/DropIndicator.svelte';
  import { invokeWithPerf, updateInputs } from '../state/performance';
  import { generateProgressChannel, type SortAudioEvent } from '../state/events';
  import { Channel } from '@tauri-apps/api/core';
  import { get } from 'svelte/store';
  import { audioFileStateManager } from '../state/stateSynchronization';
  import { D3TimelineManager, type TimelineItem } from './Timeline/D3TimelineManager';
  import { debugState } from '../state/debug.svelte';

  // Subscribe to debug state
  import {
    DragDropManager,
    type DragStartEvent,
    type DragMoveEvent,
    type DragEndEvent,
  } from './Timeline/DragDropManager';

  const dispatch = createEventDispatcher();

  let container: HTMLDivElement;
  let svgEl: SVGSVGElement;
  let axisGroup: SVGGElement;
  let pathGroup: SVGGElement;

  const height = 120;
  let width = 0;

  // D3 Manager instance
  let d3Manager: D3TimelineManager | null = null;

  // Drag Drop Manager instance
  let dragDropManager: DragDropManager;

  const originalPathWidth = 1000;

  // Reactive properties from D3 manager
  let currentTransform = d3.zoomIdentity;
  let scaleX = 1;

  let xScale: d3.ScaleLinear<number, number>;

  let playHeadPosition = 0;
  let playHeadX = 0;

  // Initialize managers when dependencies change
  $: if (width > 0 && $durationSeconds > 0) {
    initializeManagers();
  }

  // Drag and drop state - now managed by DragDropManager
  let arrowHeadY = 0; // Y position for the drop indicator arrowhead
  let arrowHeadSize = 6; // Size of the drop indicator arrowhead
  const debugShowDropLine = false;

  // Reactive values from drag drop manager
  $: dragDropState = dragDropManager?.getState() ?? {
    isDragging: false,
    draggedSegmentIndex: -1,
    dropIndicatorIndex: -1,
    dropIndicatorX: 0,
  };

  $: ({ isDragging, draggedSegmentIndex, dropIndicatorIndex, dropIndicatorX } = dragDropState);

  const DEBUG_MODE = false;
  const timelineXAxisBg = '#1d1c23';

  // Selection state (similar to Sources.svelte)
  let selectedSegments: Set<number> = new Set();
  let lastSelectedIndex: number | null = null;

  function handleSegmentSelection(
    segmentIndex: number,
    isMultiSelect: boolean = false,
    isShiftSelect: boolean = false
  ) {
    if (isShiftSelect && lastSelectedIndex !== null) {
      // Shift-select: select range from lastSelectedIndex to segmentIndex
      const start = Math.min(lastSelectedIndex, segmentIndex);
      const end = Math.max(lastSelectedIndex, segmentIndex);

      // Add all indices in the range to selection
      for (let i = start; i <= end; i++) {
        selectedSegments.add(i);
      }
    } else if (isMultiSelect) {
      // Toggle selection for multi-select
      if (selectedSegments.has(segmentIndex)) {
        selectedSegments.delete(segmentIndex);
      } else {
        selectedSegments.add(segmentIndex);
      }
    } else {
      // Single selection - clear others and select this one
      selectedSegments.clear();
      selectedSegments.add(segmentIndex);
    }

    // Update last selected index for future shift-selects
    lastSelectedIndex = segmentIndex;

    // Trigger reactivity
    selectedSegments = new Set(selectedSegments);

    // Dispatch selection change for context menu
    dispatch('selectionChange', selectedSegments);
  }

  function toggleSegmentSelection(segmentIndex: number) {
    handleSegmentSelection(segmentIndex, true);
  }

  function selectSegment(segmentIndex: number, isShiftSelect: boolean = false) {
    handleSegmentSelection(segmentIndex, false, isShiftSelect);
  }

  function handleClearSelection() {
    selectedSegments.clear();
    selectedSegments = new Set(selectedSegments);
    lastSelectedIndex = null;

    // Dispatch selection change for context menu
    dispatch('selectionChange', selectedSegments);
  }

  function initializeManagers() {
    if (!svgEl || !axisGroup || !pathGroup || !container) return;

    // Clean up existing managers
    if (d3Manager) {
      d3Manager.destroy();
    }
    if (dragDropManager) {
      dragDropManager.destroy();
    }

    // Create new D3 manager
    d3Manager = new D3TimelineManager({
      width,
      height,
      durationSeconds: $durationSeconds,
      originalPathWidth,
      onTransformChange: transform => {
        currentTransform = transform;
      },
      onAxisUpdate: scale => {
        xScale = scale;
        playHeadX = d3Manager?.getPlayheadX(playHeadPosition) ?? 0;
      },
    });

    // Initialize D3 manager with DOM elements
    d3Manager.initialize(svgEl, axisGroup, pathGroup);

    // Create new drag drop manager
    dragDropManager = new DragDropManager(appState);
    dragDropManager.initialize(d3Manager, container);

    // Update reactive values
    scaleX = d3Manager.getScaleX();
    xScale = d3Manager.getXScale() ?? xScale;
    playHeadX = d3Manager.getPlayheadX(playHeadPosition);
  }

  // Update playhead position when it changes
  $: if (d3Manager) {
    playHeadX = d3Manager.getPlayheadX(playHeadPosition);
  }

  // Update manager options when width or duration changes
  $: if (d3Manager && (width > 0 || $durationSeconds > 0)) {
    d3Manager.updateOptions({
      width,
      durationSeconds: $durationSeconds,
    });
    scaleX = d3Manager.getScaleX();
  }

  listen<number>('timeline-progress', event => {
    playHeadPosition = event.payload * $durationSeconds;
  });

  function handleClick(event: MouseEvent) {
    if (!d3Manager) return;

    const rect = container.getBoundingClientRect();
    const relativeX = event.clientX - rect.left;

    // Check if click is on a timeline segment using the manager
    const clickedSegmentIndex = $appState?.timelineItems
      ? d3Manager.findClickedSegment(relativeX, $appState.timelineItems as TimelineItem[])
      : null;

    // If clicked on empty space, clear selection and set playhead
    if (clickedSegmentIndex === null) {
      handleClearSelection();

      const clickedTime = d3Manager.clickToTime(relativeX);
      console.log(clickedTime);
      invokeWithPerf('set_timeline_play_position', { position: clickedTime });
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    // Check if Delete key was pressed and we have selected segments
    if (event.key === 'Delete' && selectedSegments.size > 0) {
      // Don't trigger if user is typing in an input field
      if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) {
        return;
      }

      event.preventDefault();

      // Get the IDs of selected audio file items only (spacers might not be deletable)
      const selectedIds: string[] = [];
      if ($appState?.timelineItems) {
        Array.from(selectedSegments).forEach(index => {
          if (index < $appState.timelineItems.length) {
            const item = $appState.timelineItems[index];
            // Only allow deletion of audio files
            if (item && isAudioFileItem(item)) {
              selectedIds.push(item.id);
            }
          }
        });
      }

      if (selectedIds.length > 0) {
        console.log('Deactivating selected audio segments:', selectedIds);

        // Use the state manager for optimistic updates and automatic sync
        audioFileStateManager
          .setFilesActive(selectedIds, false)
          .then(() => {
            console.log('Successfully deactivated selected segments');
            // Clear selection after successful deactivation
            handleClearSelection();
          })
          .catch(error => {
            console.error('Failed to deactivate segments:', error);
          });
      }
    }
  }

  onMount(() => {
    const resizeObserver = new ResizeObserver(() => {
      width = container.clientWidth;
      // The managers will be updated through reactive statements
    });

    resizeObserver.observe(container);

    return () => {
      resizeObserver.disconnect();
      // Clean up managers
      if (d3Manager) {
        d3Manager.destroy();
      }
      if (dragDropManager) {
        dragDropManager.destroy();
      }
    };
  });

  function handleDragStart(event: CustomEvent<DragStartEvent>) {
    if (!dragDropManager) return;
    dragDropManager.handleDragStart(event.detail);
  }

  function handleDragMove(event: CustomEvent<DragMoveEvent>) {
    if (!dragDropManager) return;
    dragDropManager.handleDragMove(event.detail);
  }

  function handleDragEnd(event: CustomEvent<DragEndEvent>) {
    if (!dragDropManager) return;
    dragDropManager.handleDragEnd(event.detail);
  }

  const tempYCenter = 35;
</script>

<div class="svg-container position-relative">
  <div class="position-absolute" style="font-size: 10px; color: #9d9d9d !important; bottom:20px">
    {currentTransform.k.toFixed(2)}x
  </div>
  <!-- No Active Samples Message -->
  {#if $appState?.hasNoActiveSamples}
    <div class="no-active-samples-message">
      <div class="message-content">No active samples to display</div>
      <div class="message-subtitle">
        Activate audio files from the Sources panel to see them here
      </div>
    </div>
  {/if}

  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div
    on:click={e => {
      handleClick(e);
    }}
    on:keydown={handleKeyDown}
    bind:this={container}
    role="application"
    aria-label="Timeline"
    tabindex="0"
    style="
    width: 100%;
    cursor: {isDragging ? 'grabbing' : 'default'};
    "
  >
    <svg class="waveform-svg-parent" bind:this={svgEl} {height} viewBox={`0 0 ${width} ${height}`}>
      <g transform={`translate(0, ${20})`}>
        <g bind:this={pathGroup} transform={``}>
          <!-- Zero level baseline -->
          <line
            x1="0"
            y1={tempYCenter}
            x2={width}
            y2={tempYCenter}
            stroke="white"
            stroke-width="1"
            opacity="0.3"
            pointer-events="none"
          />

          <path
            d={$appState?.combinedFile?.svgPath}
            stroke="#3091f1"
            fill="none"
            stroke-width="2"
            transform={`scale(${scaleX}, 1) `}
            pointer-events="none"
            id="waveform-path"
          />

          <Playhead {playHeadX} {currentTransform} />

          <!-- {#if $appState?.timelneItems} -->
          <g class="timeline-segments">
            {#if $appState?.timelineItems.length > 0}
              {#each $appState?.timelineItems as timelineItem, i}
                <TimelineSegment
                  {scaleX}
                  index={i}
                  startOffset={timelineItem.startOffset}
                  size={getItemSize(timelineItem)}
                  {originalPathWidth}
                  zoomTransform={currentTransform}
                  id={timelineItem.id}
                  active={isItemActive(timelineItem)}
                  isSelected={selectedSegments.has(i)}
                  onSegmentSelect={selectSegment}
                  onSegmentToggle={toggleSegmentSelection}
                  canBeDragged={canItemBeDragged(timelineItem)}
                  itemColor={getItemColor(timelineItem)}
                  {DEBUG_MODE}
                  on:dragStart={handleDragStart}
                  on:dragMove={handleDragMove}
                  on:dragEnd={handleDragEnd}
                />
                <!-- <text
                  x={(timelineItem.startOffset * originalPathWidth) + 4}
                  y={40}
                  dominant-baseline="middle"
                  fill="white"
                  font-size="10"
                  font-family="monospace"
                  pointer-events="none"
                >{formatFileName(timelineItem.fileName)}</text>
                <rect
                  x={timelineItem.startOffset * originalPathWidth}
                  y={0}
                  width={timelineItem.size*originalPathWidth}
                  height="80"
                  fill="rgba(0, 200, 255, 0.15)"
                  stroke="rgba(0, 200, 255, 0.5)"
                  stroke-width="0.5"
                /> -->
              {/each}
            {/if}
          </g>
        </g>
      </g>
      {#if $appState?.timelineItems.length > 0}
        <LabelLayer
          {scaleX}
          items={$appState?.timelineItems}
          {originalPathWidth}
          {currentTransform}
          {isDragging}
        ></LabelLayer>
      {/if}

      <DropIndicator
        {isDragging}
        {dropIndicatorIndex}
        {dropIndicatorX}
        {arrowHeadY}
        {arrowHeadSize}
        {debugShowDropLine}
      />

      <g> </g>
      <!-- TIMELINE BACKGROUND -->
      <rect x="0" y={100} {width} height="20" fill={timelineXAxisBg} />
      <g bind:this={axisGroup} transform={`translate(0, ${height - 20})`} />
    </svg>
  </div>

  <!-- Debug Panel -->
  {#if DEBUG_MODE}
    <div class="debug">
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
      {#if $appState?.timelineItems && $appState.timelineItems.length > 0}
        <div><b>Items ({$appState.timelineItems.length}):</b></div>
        {#each $appState.timelineItems as item, i}
          <div class="item" class:dragged={i === draggedSegmentIndex}>
            <b>#{i}</b>
            <span class="item-type">[{item.type}]</span>
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
          </div>
        {/each}
      {:else}
        <div>No items</div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .waveform-svg-parent {
    /* Timeline waveform container */
  }
  .svg-container {
    background-color: var(--bs-primary-bg-subtle);
  }
  svg {
    width: 100%;
    height: auto;
  }

  g.axis text {
    font-family: monospace;
    font-size: 10px; /* optional: adjust as needed */
  }

  /* No Active Samples Message */
  .no-active-samples-message {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    text-align: center;
    color: var(--bs-secondary);
    z-index: 10;
    pointer-events: none;
  }

  .message-content {
    font-size: 16px;
    font-weight: 500;
    margin-bottom: 8px;
    color: var(--bs-secondary);
  }

  .message-subtitle {
    font-size: 14px;
    color: var(--bs-secondary);
    opacity: 0.8;
  }

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

  .debug .item.dragged {
    background: #333;
    color: #ff0;
  }
</style>
