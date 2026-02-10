<script lang="ts">
  import * as d3 from 'd3';
  import { createEventDispatcher, onDestroy, onMount } from 'svelte';
  import { opPlaybackService } from '../state/opPlaybackService';
  import {
    previewActive,
    previewIds,
    selectedIds,
    selectionService,
  } from '../state/selection.svelte';
  import { appState, timelineDebugMode, type AudioFileTimelineItem } from '../state/state.svelte';
  import {
    buildHierarchyMaps,
    getIndicesToMoveOnDrag,
    type FlattenedTimelineItem,
  } from '../state/timeline/timelineGraph';
  import {
    canItemBeDragged,
    getItemColor,
    getItemSize,
    getItemSvgPath,
    isItemActive,
  } from '../utils/timelineHelpers';
  import { D3TimelineManager } from './Timeline/D3TimelineManager';
  import {
    DEFAULT_DD,
    DragDropManager,
    type DragDropState,
    type DragEndEvent,
    type DragMoveEvent,
    type DragStartEvent,
  } from './Timeline/DragDropManager';
  import DropIndicator from './Timeline/DropIndicator.svelte';
  import LabelLayer from './Timeline/LabelLayer.svelte';
  import Playhead from './Timeline/Playhead.svelte';
  import TimelineDebugPanel from './Timeline/TimelineDebugPanel.svelte';
  import TimelineSegment from './Timeline/TimelineSegment.svelte';
  import { dropzone } from '$lib/attachments/droppable';
  import { TIMELINE_DERIVED, TIMELINE_LAYOUT } from '$lib/config/timelineConfig';
  import { type OperationId } from '$lib/state/operation';
  import { timelinePlayhead } from '$lib/state/timeline/timelinePlaybackState';
  import {
    activeTimelineId,
    setActiveTimeline,
    type TimelineId,
  } from '$lib/state/timeline/timelines';
  // Import operation playback service
  import { TimelineViewer } from '$lib/state/timeline/TimelineViewer';
  import timelinePlaybackService from '$lib/state/timelinePlaybackService';
  import { removeOperationSourcesFromCurrentOpCommand } from '$lib/state/undo/undo';

  const dispatch = createEventDispatcher();

  export let timelineViewer: TimelineViewer | null = null;
  export let timelineId: TimelineId | null = null; // Keep for backward compatibility

  let container: HTMLDivElement;
  let svgEl: SVGSVGElement;
  let axisGroup: SVGGElement;
  let pathGroup: SVGGElement;

  // SVG Layout constants from centralized config
  const topPadding = TIMELINE_LAYOUT.TOP_PADDING;
  const axisHeight = TIMELINE_LAYOUT.AXIS_HEIGHT;
  const baseContentHeight = TIMELINE_LAYOUT.BASE_CONTENT_HEIGHT;

  // Reactive dimensions - now tracks both width and height
  let width = 0;
  let height: number = TIMELINE_LAYOUT.DEFAULT_HEIGHT; // Default height, will be updated by ResizeObserver

  // Computed scalable region dimensions
  $: contentHeight = height - topPadding - axisHeight;
  $: contentScaleY = contentHeight / baseContentHeight;
  $: tempYCenter = TIMELINE_DERIVED.CENTER_Y; // Center line in design space
  $: currentActiveTimelineId = $activeTimelineId;
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

  // Initialize managers once when component mounts and DOM is ready
  $: if (width > 0 && currentDuration > 0 && !d3Manager) {
    initializeManagers();
  }

  // Update managers when props change (instead of re-initializing)
  $: if (d3Manager && (width > 0 || currentDuration > 0)) {
    updateManagers();
  }

  // Drag and drop state
  let arrowHeadY = 0;
  let arrowHeadSize = 6;
  const debugShowDropLine = false;

  // Track if we're currently scrolling to prevent keyboard events during scroll
  let isScrolling = false;
  let scrollTimeout: number | null = null;

  // Reactive drag-drop state (driven by store subscription)
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

  // ============================================================================
  // TIMELINE ITEMS - Operation-based system (now using TimelineViewer)
  // ============================================================================

  // Use TimelineViewer as the primary interface
  $: effectiveTimelineId = timelineViewer?.id ?? timelineId ?? null;

  // Determine if this is an operation-based timeline
  $: isOperationTimeline = timelineViewer != null;

  // Reactive timeline items from TimelineViewer
  $: timelineItemsStore = timelineViewer?.items ?? null;
  $: timelineItems = timelineItemsStore ? $timelineItemsStore : [];

  // Use timeline's own waveform state from TimelineViewer
  $: waveformStateStore = timelineViewer?.waveformState ?? null;
  $: waveformState = waveformStateStore ? $waveformStateStore : null;
  $: currentDuration = waveformState?.totalDuration || 30; // Default duration fallback
  $: isLoadingWaveforms = waveformState?.loading || waveformState?.loadingWaveforms || false;

  // Timeline-scoped hierarchy derived from timeline's own items
  $: timelineHierarchy = (() => {
    if (!timelineItems || timelineItems.length === 0) return null;

    // Convert TimelineItems to FlattenedTimelineItems for hierarchy building
    const flattenedItems: FlattenedTimelineItem[] = timelineItems.map(item => {
      const audioItem = item as AudioFileTimelineItem;
      return {
        kind: (audioItem.kind || 'sample') as 'sample' | 'merge',
        id: audioItem.id,
        fileName: audioItem.fileName,
        svgPath: audioItem.svgPath,
        startOffset: audioItem.startOffset,
        size: audioItem.size,
        active: audioItem.active,
        duration: audioItem.duration,
        children: audioItem.children || [],
        parentId: audioItem.parentId,
        depth: audioItem.depth ?? 0,
        isGroup: audioItem.isGroup ?? false,
        operationName: audioItem.operationName || '',
        rootGroupId: undefined,
      };
    });

    return buildHierarchyMaps(flattenedItems);
  })();

  // Check if we have no active samples
  $: hasNoActiveSamples = (timelineItems?.length || 0) === 0 && !isLoadingWaveforms;

  // Selection state - derived from the selection service
  $: selectedSegments = $selectedIds;
  $: previewSegments = $previewIds;
  $: isPreviewActive = $previewActive;
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

    // DEBUG: Log initialization
    console.log('🔧 Timeline: Initializing managers with:', {
      currentDuration,
      width,
      timelineItems: timelineItems?.length || 0,
      isLoadingWaveforms,
    });

    // Create new D3 manager
    d3Manager = new D3TimelineManager({
      width,
      height,
      durationSeconds: currentDuration,
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

    // ✅ Subscribe to manager's state store
    unsubscribeDragDrop = dragDropManager.state.subscribe(s => {
      dragDropState = s;
    });

    // Update values
    scaleX = d3Manager.getScaleX();
    xScale = d3Manager.getXScale() ?? xScale;
    playHeadX = d3Manager.getPlayheadX(playHeadPosition);
  }

  function updateManagers() {
    if (!d3Manager) return;

    // Update D3 manager options
    d3Manager.updateOptions({ width, durationSeconds: currentDuration });
    scaleX = d3Manager.getScaleX();
    xScale = d3Manager.getXScale() ?? xScale;
    playHeadX = d3Manager.getPlayheadX(playHeadPosition);

    // Update drag drop manager if needed
    if (dragDropManager) {
      dragDropManager.setSelectedSegments(selectedSegments);
    }
  }

  // Create timeline-specific playhead store derived from effective timeline ID
  $: timelinePlayheadStore = effectiveTimelineId ? timelinePlayhead(effectiveTimelineId) : null;
  $: timelinePlayheadTime = timelinePlayheadStore ? $timelinePlayheadStore : 0;

  // Update playhead position when it changes
  $: if (d3Manager) {
    playHeadX = d3Manager.getPlayheadX(playHeadPosition);
  }

  // Update playHeadPosition from timeline-specific playhead store (guaranteed to be a number)
  $: {
    const newPlayHeadPos = timelinePlayheadTime ?? 0;
    if (newPlayHeadPos !== playHeadPosition) {
      console.log(
        `🎵 Timeline playhead update: ${playHeadPosition.toFixed(2)}s -> ${newPlayHeadPos.toFixed(2)}s`
      );
      playHeadPosition = newPlayHeadPos;
    }
  }

  function handleClick(event: MouseEvent) {
    if (!d3Manager) return;

    // Set this timeline as the active timeline when clicked
    if (effectiveTimelineId) {
      setActiveTimeline(effectiveTimelineId);
    }

    const rect = container.getBoundingClientRect();
    const relativeX = event.clientX - rect.left;
    const relativeY = event.clientY - rect.top;

    // Check if click is in the x-axis area (footer region)
    const isXAxisClick = relativeY >= height - axisHeight;

    if (isXAxisClick) {
      if (timelineSource?.kind !== 'operation' || !effectiveTimelineId) {
        return;
      }
      // Click is in the x-axis area - set playhead position and clear selection
      handleClearSelection();
      const clickedTime = d3Manager.clickToTime(relativeX);
      console.log(clickedTime);
      // Use operation playback service for seeking
      timelinePlaybackService
        .seekTimeline(effectiveTimelineId, clickedTime)
        .catch((err: unknown) => console.error('Failed to seek:', err));
      return;
    }

    // Check for segment clicks only if not in x-axis area
    const clickedSegmentIndex =
      (timelineItems?.length || 0) > 0 && timelineItems
        ? d3Manager.findClickedSegment(relativeX, timelineItems as any)
        : null;

    if (clickedSegmentIndex === null) {
      if (timelineSource?.kind !== 'operation' || !effectiveTimelineId) {
        return;
      }
      handleClearSelection();
      const clickedTime = d3Manager.clickToTime(relativeX);

      // Use operation playback service for seeking
      opPlaybackService
        .seekTimeline(effectiveTimelineId, clickedTime)
        .catch((err: unknown) => console.error('Failed to seek:', err));
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    // Prevent repeated calls when key is held down
    if (event.repeat) {
      return;
    }

    // Don't process keyboard events if we're currently dragging or scrolling
    if (isDragging || isScrolling) {
      return;
    }

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
      console.log('🔧 Timeline: Toggled debug mode:', !$timelineDebugMode);
      return;
    }

    if (event.key === 'Delete' && selectedSegments.size > 0) {
      if (timelineSource?.kind !== 'operation') {
        return;
      }
      if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement)
        return;
      event.preventDefault();

      // Collect unique operation IDs from selected timeline items
      const operationIdsToRemove = new Set<OperationId>();
      if ((timelineItems?.length || 0) > 0) {
        Array.from(selectedSegments).forEach(index => {
          if (timelineItems && index < timelineItems.length) {
            const item = timelineItems[index];
            if (item && (item as any).operationId) {
              operationIdsToRemove.add((item as any).operationId);
            }
          }
        });
      }
      console.log('Removing operations from current op:', Array.from(operationIdsToRemove));
      if (operationIdsToRemove.size > 0) {
        removeOperationSourcesFromCurrentOpCommand(Array.from(operationIdsToRemove));
        handleClearSelection();
      }
    }
  }

  function handleWheel(event: WheelEvent) {
    // Set scrolling flag when wheel events occur
    isScrolling = true;

    // Clear any existing timeout
    if (scrollTimeout !== null) {
      clearTimeout(scrollTimeout);
    }

    // Reset scrolling flag after a short delay
    scrollTimeout = setTimeout(() => {
      isScrolling = false;
      scrollTimeout = null;
    }, 150) as unknown as number;
  }

  onMount(() => {
    // DEBUG: Log component mount state
    console.log('🔧 Timeline: Component mounted with:', {
      timelineItems: timelineItems?.length || 0,
      isLoadingWaveforms,
    });

    // Initialize waveform service
    console.log('🔧 Timeline: Initializing waveform service...');
    try {
      console.log('🔧 Timeline: Waveform service initialized');
    } catch (error) {
      console.error('🔧 Timeline: Failed to initialize waveform service:', error);
    }

    // Initialize the operation playback progress listener
    opPlaybackService.initProgressListener().catch(err => {
      console.error('🔧 Timeline: Failed to initialize op playback progress listener:', err);
    });

    const resizeObserver = new ResizeObserver(() => {
      width = container.clientWidth;
      height = container.clientHeight || TIMELINE_LAYOUT.DEFAULT_HEIGHT; // Fallback to default if height is 0
    });

    resizeObserver.observe(container);

    return () => {
      resizeObserver.disconnect();

      // Clean up managers
      if (d3Manager) {
        d3Manager.destroy();
        d3Manager = null;
      }

      if (dragDropManager) {
        dragDropManager.destroy();
        dragDropManager = null;
      }

      // Clean up subscriptions
      if (unsubscribeDragDrop) {
        unsubscribeDragDrop();
        unsubscribeDragDrop = null;
      }

      // Clean up operation playback listener
      opPlaybackService.cleanupProgressListener();
    };
  });

  onDestroy(() => {
    // Clean up managers if component is destroyed without onMount cleanup firing
    if (d3Manager) {
      d3Manager.destroy();
      d3Manager = null;
    }

    if (dragDropManager) {
      dragDropManager.destroy();
      dragDropManager = null;
    }

    // Clean up subscriptions
    if (unsubscribeDragDrop) {
      unsubscribeDragDrop();
      unsubscribeDragDrop = null;
    }

    // Clean up operation playback listener
    opPlaybackService.cleanupProgressListener();

    // Clear any pending scroll timeout
    if (scrollTimeout !== null) {
      clearTimeout(scrollTimeout);
      scrollTimeout = null;
    }
  });

  function handleDragStart(event: CustomEvent<DragStartEvent>) {
    if (!dragDropManager || !timelineItems) return;

    const draggedIndex = event.detail.index;
    const draggedItem = timelineItems[draggedIndex] as any; // Cast to access hierarchy props

    // If the dragged segment is not in the current selection, clear the selection
    if (!selectedSegments.has(draggedIndex)) {
      handleClearSelection();
    }

    // If this is a MergeOp (group), we need to drag all its descendants too
    if (draggedItem?.kind === 'merge' && draggedItem?.isGroup && timelineHierarchy) {
      // Get all indices that should move with this group
      const indicesToMove = getIndicesToMoveOnDrag(
        draggedIndex,
        timelineHierarchy.flatItems,
        timelineHierarchy
      );
      // Set these on the drag manager
      dragDropManager.setSegmentsToMove(new Set(indicesToMove));
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
</script>

<div class="svg-container position-relative">
  <div class="position-absolute" style="font-size: 10px; color: #9d9d9d !important; bottom:20px">
    {currentTransform.k.toFixed(2)}x
  </div>
  <!-- No Active Samples Message -->
  {#if hasNoActiveSamples}
    <div class="no-active-samples-message">
      <div class="message-content">
        {#if isLoadingWaveforms}
          Loading waveforms...
        {:else}
          No active samples to display
        {/if}
      </div>
      <div class="message-subtitle">
        {#if !isLoadingWaveforms}
          Activate audio files from the Sources panel to see them here
        {/if}
      </div>
    </div>
  {/if}

  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    use:dropzone={{
      accepts: ['sample'],
      on_drop: ({ data, sourceId }) => {
        console.log('Dropped sample:', data, sourceId);
      },
    }}
    on:click={e => {
      handleClick(e);
    }}
    on:keydown={handleKeyDown}
    on:wheel={handleWheel}
    bind:this={container}
    aria-label="Timeline"
    tabindex="0"
    class="w-100 h-fill-available timeline-svg-wrapper"
    style="
    cursor: {isDragging ? 'grabbing' : 'default'};
    "
  >
    <svg class="waveform-svg-parent" bind:this={svgEl} viewBox={`0 0 ${width} ${height}`}>
      <!-- Fixed header region (top padding) -->
      <g class="fixed-header" transform="translate(0, 0)">
        <!-- Reserved space for future header content -->
      </g>

      <!-- Scalable content region (waveforms, segments, playhead) -->
      <g
        class="scalable-content"
        transform={`translate(0, ${topPadding}) scale(1, ${contentScaleY})`}
      >
        <g bind:this={pathGroup} transform="">
          <!-- Zero level baseline (in design space coordinates) -->
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

          <!-- Timeline segments - uses reactive timelineItems (operation-based or legacy) -->
          <g class="timeline-segments">
            {#if (timelineItems?.length || 0) > 0}
              {#each timelineItems as timelineItem, i}
                {@const audioItem = timelineItem as any}
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
                  isInPreview={previewSegments.has(i)}
                  {isPreviewActive}
                  isBeingDragged={isDragging && draggedSegmentIndex === i}
                  onSegmentSelect={selectSegment}
                  onSegmentToggle={toggleSegmentSelection}
                  canBeDragged={canItemBeDragged(timelineItem)}
                  itemColor={getItemColor(timelineItem)}
                  svgPath={getItemSvgPath(timelineItem)}
                  kind={audioItem.kind || 'sample'}
                  depth={audioItem.depth ?? 0}
                  isGroup={audioItem.isGroup ?? false}
                  childCount={audioItem.children?.length ?? 0}
                  {DEBUG_MODE}
                  on:dragStart={handleDragStart}
                  on:dragMove={handleDragMove}
                  on:dragEnd={handleDragEnd}
                />
              {/each}
            {/if}
          </g>
          <Playhead {playHeadX} {currentTransform} {contentScaleY} />
        </g>
      </g>

      <!-- Fixed label layer (positioned outside scalable content) -->
      {#if (timelineItems?.length || 0) > 0 && timelineItems}
        <LabelLayer
          {scaleX}
          items={timelineItems}
          {originalPathWidth}
          {currentTransform}
          {isDragging}
          {segmentsToMove}
        />
      {/if}

      <!-- Drop indicator (positioned outside scalable content) -->
      <DropIndicator
        {isDragging}
        {dropIndicatorIndex}
        {dropIndicatorX}
        {arrowHeadY}
        {arrowHeadSize}
        {debugShowDropLine}
      />

      <!-- Fixed footer region (x-axis) -->
      <g class="fixed-footer">
        <rect x="0" y={height - axisHeight} {width} height={axisHeight} fill={timelineXAxisBg} />
        <g bind:this={axisGroup} transform={`translate(0, ${height - axisHeight})`} />
      </g>
    </svg>
  </div>
</div>
<!-- Debug Panel -->
{#if $timelineDebugMode}
  <TimelineDebugPanel
    {isDragging}
    {draggedSegmentIndex}
    {dropIndicatorIndex}
    {dropIndicatorX}
    {width}
    {scaleX}
    {playHeadPosition}
    {currentTransform}
    {timelineItems}
    {originalPathWidth}
    {selectedSegments}
    {lastSelectedIndex}
    {segmentsToMove}
  />
{/if}

<style>
  .timeline-svg-wrapper:focus-visible {
    outline: 0px !important;
  }
  .svg-container {
    background-color: var(--bs-primary-bg-subtle);
    overflow: hidden;
    flex: 1; /* Take up remaining space in flex container */
    min-height: 0; /* Allow flexbox to shrink below content size */
  }
  svg {
    width: 100%;
    height: 100%;
    display: block;
  }

  :global(g.axis text) {
    font-family: monospace;
    font-size: 10px; /* optional: adjust as needed */
  }

  :global(.splitpanes__pane) {
    transition: none !important;
  }

  /* No Active Samples Message */
  .no-active-samples-message {
    position: absolute;
    top: 39%;
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
    color: var(--bs-secondary);
  }

  .message-subtitle {
    font-size: 14px;
    color: var(--bs-secondary);
    opacity: 0.8;
  }
</style>
