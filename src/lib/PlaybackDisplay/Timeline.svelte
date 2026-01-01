<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
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
    getItemSvgPath,
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
  import { selectionService, selectedIds } from '../state/selection.svelte';
  import { D3TimelineManager, type TimelineItem } from './Timeline/D3TimelineManager';
  import { debugState, timelineDebugMode } from '../state/debug.svelte';
  import TimelineDebugPanel from './Timeline/TimelineDebugPanel.svelte';

  import {
    DragDropManager,
    type DragStartEvent,
    type DragMoveEvent,
    type DragEndEvent,
    type DragDropState, // <-- add this type export in DragDropManager.ts
    DEFAULT_DD, // <-- export this from DragDropManager.ts (recommended)
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
  let dragDropManager: DragDropManager | null = null;

  // subscription cleanup
  let unsubscribeDragDrop: null | (() => void) = null;

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

  // Drag and drop state
  let arrowHeadY = 0;
  let arrowHeadSize = 6;
  const debugShowDropLine = false;

  // ✅ reactive drag-drop state (now driven by store subscription)
  let dragDropState: DragDropState = DEFAULT_DD;

  // These locals drive your template
  let isDragging = false;
  let draggedSegmentIndex = -1;
  let dropIndicatorIndex = -1;
  let dropIndicatorX = 0;
  let segmentsToMove: number[] = [];

  // keep locals in sync with dragDropState
  $: ({ isDragging, draggedSegmentIndex, dropIndicatorIndex, dropIndicatorX, segmentsToMove } =
    dragDropState);

  const DEBUG_MODE = false;
  const timelineXAxisBg = '#1d1c23';

  // Selection state - now derived from the selection service
  $: selectedSegments = $selectedIds;
  let lastSelectedIndex: number | null = null;

  function handleSegmentSelection(
    segmentIndex: number,
    isMultiSelect: boolean = false,
    isShiftSelect: boolean = false
  ) {
    // Use the selection service to handle the click
    selectionService.handleClick(segmentIndex, {
      isMultiSelect,
      isShiftSelect,
      lastSelectedIndex,
      source: 'timeline',
    });

    // Update last selected index for shift-select operations
    lastSelectedIndex = segmentIndex;

    // Update the drag drop manager with the new selection
    if (dragDropManager) {
      dragDropManager.setSelectedSegments(selectedSegments);
    }

    dispatch('selectionChange', selectedSegments);
  }

  function toggleSegmentSelection(segmentIndex: number) {
    handleSegmentSelection(segmentIndex, true);
  }

  function selectSegment(segmentIndex: number, isShiftSelect: boolean = false) {
    handleSegmentSelection(segmentIndex, false, isShiftSelect);
  }

  function handleClearSelection() {
    selectionService.clear('timeline');
    lastSelectedIndex = null;

    // Update the drag drop manager with the cleared selection
    if (dragDropManager) {
      dragDropManager.setSelectedSegments(selectedSegments);
    }

    dispatch('selectionChange', selectedSegments);
  }

  function initializeManagers() {
    if (!svgEl || !axisGroup || !pathGroup || !container) return;

    // Clean up existing managers
    if (d3Manager) d3Manager.destroy();

    if (dragDropManager) dragDropManager.destroy();

    // ✅ Clean up previous subscription (important since you recreate the manager)
    unsubscribeDragDrop?.();
    unsubscribeDragDrop = null;

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

    d3Manager.initialize(svgEl, axisGroup, pathGroup);

    // Create new drag drop manager
    dragDropManager = new DragDropManager(appState);
    dragDropManager.initialize(d3Manager, container);

    // Initialize with current selection
    dragDropManager.setSelectedSegments(selectedSegments);

    // ✅ Subscribe to manager's state store (Option A)
    unsubscribeDragDrop = dragDropManager.state.subscribe(s => {
      dragDropState = s;
    });

    // Update values
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
    d3Manager.updateOptions({ width, durationSeconds: $durationSeconds });
    scaleX = d3Manager.getScaleX();
  }

  listen<number>('timeline-progress', event => {
    playHeadPosition = event.payload * $durationSeconds;
  });

  function handleClick(event: MouseEvent) {
    if (!d3Manager) return;

    const rect = container.getBoundingClientRect();
    const relativeX = event.clientX - rect.left;
    const relativeY = event.clientY - rect.top;

    // Check if click is in the x-axis area (bottom 20px of the timeline)
    const isXAxisClick = relativeY >= height - 20;

    if (isXAxisClick) {
      // Click is in the x-axis area - set playhead position and clear selection
      handleClearSelection();
      const clickedTime = d3Manager.clickToTime(relativeX);
      invokeWithPerf('set_timeline_play_position', { position: clickedTime });
      return;
    }

    // Check for segment clicks only if not in x-axis area
    const clickedSegmentIndex = $appState?.timelineItems
      ? d3Manager.findClickedSegment(relativeX, $appState.timelineItems as TimelineItem[])
      : null;

    if (clickedSegmentIndex === null) {
      handleClearSelection();
      const clickedTime = d3Manager.clickToTime(relativeX);
      invokeWithPerf('set_timeline_play_position', { position: clickedTime });
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    // Toggle timeline debug mode in dev mode with Ctrl+Shift+Space
    if (
      typeof import.meta !== 'undefined' &&
      typeof (import.meta as any).env !== 'undefined' &&
      (import.meta as any).env.DEV &&
      event.ctrlKey &&
      event.shiftKey &&
      event.code === 'Space'
    ) {
      event.preventDefault();
      event.stopPropagation();
      timelineDebugMode.toggle();
      console.log('🔧 Timeline: Toggled debug mode:', !$debugState.timelineDebugMode);
      return;
    }

    if (event.key === 'Delete' && selectedSegments.size > 0) {
      if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement)
        return;
      event.preventDefault();

      const selectedIds: string[] = [];
      if ($appState?.timelineItems) {
        Array.from(selectedSegments).forEach(index => {
          if (index < $appState.timelineItems.length) {
            const item = $appState.timelineItems[index];
            if (item && isAudioFileItem(item)) selectedIds.push(item.id);
          }
        });
      }

      if (selectedIds.length > 0) {
        audioFileStateManager
          .setFilesActive(selectedIds, false)
          .then(() => handleClearSelection())
          .catch(error => console.error('Failed to deactivate segments:', error));
      }
    }
  }

  onMount(() => {
    const resizeObserver = new ResizeObserver(() => {
      width = container.clientWidth;
    });

    resizeObserver.observe(container);

    return () => {
      resizeObserver.disconnect();
      if (d3Manager) d3Manager.destroy();
      if (dragDropManager) dragDropManager.destroy();
      unsubscribeDragDrop?.();
    };
  });

  onDestroy(() => {
    // in case the component is destroyed without onMount cleanup firing as expected
    unsubscribeDragDrop?.();
  });

  function handleDragStart(event: CustomEvent<DragStartEvent>) {
    if (!dragDropManager) return;

    // If the dragged segment is not in the current selection, clear the selection
    if (!selectedSegments.has(event.detail.index)) {
      handleClearSelection();
    }

    dragDropManager.handleDragStart(event.detail);
  }

  function handleDragMove(event: CustomEvent<DragMoveEvent>) {
    if (!dragDropManager) return;
    dragDropManager.handleDragMove(event.detail);
  }

  function handleDragEnd(event: CustomEvent<DragEndEvent>) {
    if (!dragDropManager) return;
    dragDropManager.handleDragEnd(event.detail);
    handleClearSelection();
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
            opacity="0"
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
                  isBeingDragged={isDragging && draggedSegmentIndex === i}
                  onSegmentSelect={selectSegment}
                  onSegmentToggle={toggleSegmentSelection}
                  canBeDragged={canItemBeDragged(timelineItem)}
                  itemColor={getItemColor(timelineItem)}
                  svgPath={getItemSvgPath(timelineItem)}
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
          {segmentsToMove}
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
  {#if $debugState.timelineDebugMode}
    <TimelineDebugPanel
      {isDragging}
      {draggedSegmentIndex}
      {dropIndicatorIndex}
      {dropIndicatorX}
      {width}
      {scaleX}
      {playHeadPosition}
      {currentTransform}
      timelineItems={$appState?.timelineItems || []}
      {originalPathWidth}
      {selectedSegments}
      {lastSelectedIndex}
      {segmentsToMove}
    />
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
</style>
