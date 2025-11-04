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
  import TimelineSegment from './Timeline/TimelineSegment.svelte';
  import LabelLayer from './Timeline/LabelLayer.svelte';
  import Playhead from './Timeline/Playhead.svelte';
  import DropIndicator from './Timeline/DropIndicator.svelte';
  import { invokeWithPerf, updateInputs } from '../state/performance';
  import { generateProgressChannel, type SortAudioEvent } from '../state/events';
  import { Channel } from '@tauri-apps/api/core';
  import { get } from 'svelte/store';
  import { audioFileStateManager } from '../state/stateSynchronization';

  const dispatch = createEventDispatcher();

  let container: HTMLDivElement;
  let svgEl: SVGSVGElement;
  let axisGroup: SVGGElement;
  let pathGroup: SVGGElement;
  let labelGroup: SVGGElement;

  const height = 120;
  $: if ($appState?.combinedFileLength && width > 0) {
    updateScales();
  }

  const originalPathWidth = 1000;
  let currentTransform = d3.zoomIdentity;
  let width = 0;
  let scaleX = 1;

  let xScale: d3.ScaleLinear<number, number>;
  let playHeadPosition = 0;
  let playHeadX = 0;
  $: playHeadX = xScale?.(playHeadPosition) ?? 0;

  // Drag and drop state
  let isDragging = false;
  let draggedSegmentIndex = -1;
  let dropIndicatorIndex = -1;
  let dropIndicatorX = 0;
  let arrowHeadY = 0; // Y position for the drop indicator arrowhead
  let arrowHeadSize = 6; // Size of the drop indicator arrowhead
  const debugShowDropLine = false;

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

  function updateScales() {
    xScale = d3.scaleLinear().domain([0, $durationSeconds]).range([0, width]);
    scaleX = width / originalPathWidth;
    renderAxis(xScale);
  }

  listen<number>('timeline-progress', event => {
    playHeadPosition = event.payload * $durationSeconds;
  });

  function handleClick(event: MouseEvent) {
    const rect = container.getBoundingClientRect();
    const relativeX = event.clientX - rect.left;

    // Check if click is on a timeline segment
    let clickedOnSegment = false;
    if ($appState?.timelineItems) {
      for (let i = 0; i < $appState.timelineItems.length; i++) {
        const item = $appState.timelineItems[i];
        const itemStartX =
          item.startOffset * originalPathWidth * scaleX * currentTransform.k + currentTransform.x;
        const itemEndX = itemStartX + item.size * originalPathWidth * scaleX * currentTransform.k;

        if (relativeX >= itemStartX && relativeX <= itemEndX) {
          clickedOnSegment = true;
          break;
        }
      }
    }

    // If clicked on empty space, clear selection and set playhead
    if (!clickedOnSegment) {
      handleClearSelection();

      const clickedTime = currentTransform
        .rescaleX(d3.scaleLinear().domain([0, $durationSeconds]).range([0, width]))
        .invert(relativeX);
      console.log(clickedTime);
      const newPlayPosition = Math.max(0, Math.min(clickedTime, $durationSeconds));
      console.log(newPlayPosition);
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

      // Get the IDs of the selected segments
      const selectedIds: string[] = [];
      if ($appState?.timelineItems) {
        Array.from(selectedSegments).forEach(index => {
          if (index < $appState.timelineItems.length) {
            selectedIds.push($appState.timelineItems[index].id);
          }
        });
      }

      if (selectedIds.length > 0) {
        console.log('Deactivating selected segments:', selectedIds);

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

  function renderAxis(scale: d3.ScaleLinear<number, number>) {
    const axis = d3
      .axisBottom(scale)
      .ticks(Math.floor(width / 60))
      .tickFormat((d: number) => {
        const m = Math.floor(d / 60);
        const s = Math.floor(d % 60);
        return `${m}:${s.toString().padStart(2, '0')}`;
      });

    d3.select(axisGroup).call(axis);

    d3.select(axisGroup)
      .call(axis)
      .selectAll('text')
      .style('font-family', 'monospace')
      .style('font-size', '10px'); // optional

    d3.select(axisGroup)
      .call(axis)
      .selectAll('text')
      .style('font-family', 'monospace')
      .style('font-size', '10px'); // optional

    const ticks = d3.selectAll('g.tick');

    ticks
      .filter((_, i, nodes) => i === 0)
      .attr('text-anchor', 'start')
      .attr('dx', '0.5em');
    //   .attr('color', 'red')

    ticks
      .filter((_, i, nodes) => i === nodes.length - 1)
      .attr('text-anchor', 'end')
      .attr('dx', '-0.5em');
    //   .attr('color', 'red')

    ticks.filter((_, i, nodes) => i !== 0).attr('color', 'white');
  }

  function setupZoom() {
    const pathGroupD3 = d3.select(pathGroup);

    const labelGroupD3 = d3.select(labelGroup);

    d3.select(svgEl).call(
      d3
        .zoom<SVGSVGElement, unknown>()
        .scaleExtent([1, 10])
        .translateExtent([
          [0, 0],
          [width, 0],
        ])
        .extent([
          [0, 0],
          [width, 0],
        ])
        // .extent([[0, 0], [width, height]])
        .on('zoom', event => {
          currentTransform = event.transform;
          pathGroupD3.attr(
            'transform',
            `translate(${event.transform.x}, 0) scale(${event.transform.k}, 1)`
          );
          const newXScale = currentTransform.rescaleX(
            d3.scaleLinear().domain([0, $durationSeconds]).range([0, width])
          );
          renderAxis(newXScale);
        })
    );
  }

  onMount(() => {
    const resizeObserver = new ResizeObserver(() => {
      width = container.clientWidth;
      updateScales();
      setupZoom();
    });

    const ticks = d3.selectAll('.x-axis .tick text');
    const t = d3.selectAll('g.tick');
    ticks
      .filter((_, i, nodes) => i === 0)
      .attr('text-anchor', 'start')
      .attr('dx', '0.5em')
      .attr('color', 'red');

    ticks
      .filter((_, i, nodes) => i === nodes.length - 1)
      .attr('text-anchor', 'end')
      .attr('dx', '-0.5em');

    d3.selectAll('g.tick')
      .filter(function (d) {
        return d == 50;
      })
      //only ticks that returned true for the filter will be included
      //in the rest of the method calls:
      .select('line') //grab the tick line
      .attr('class', 'quadrantBorder') //style with a custom class and CSS
      .style('stroke-width', 5);

    resizeObserver.observe(container);
    return () => resizeObserver.disconnect();
  });

  function handleDragStart(
    event: CustomEvent<{ index: number; startPos: { x: number; y: number }; segmentId: number }>
  ) {
    console.log(`%cHERE LINE :182 %c`, 'color: yellow; font-weight: bold', '');

    const { index, startPos, segmentId } = event.detail;
    console.log(index);
    isDragging = true;
    draggedSegmentIndex = index;
    dropIndicatorIndex = -1;

    console.log(`Timeline: Started dragging segment ${index}`);
  }

  function handleDragMove(
    event: CustomEvent<{
      index: number;
      mousePos: { x: number; y: number };
      dragDistance: number;
      event: d3.D3DragEvent<SVGGElement, unknown, d3.SubjectPosition>;
    }>
  ) {
    console.log(`%cHERE LINE :196 %c`, 'color: yellow; font-weight: bold', '');

    if (!isDragging) return;

    const { index, mousePos, dragDistance } = event.detail;

    // Calculate which segment position the mouse is over
    const containerRect = container.getBoundingClientRect();
    const relativeX = mousePos.x - containerRect.left;

    // Apply zoom transform to get the correct timeline position
    const timelineX = currentTransform.invert
      ? currentTransform.invertX(relativeX)
      : relativeX / currentTransform.k - currentTransform.x / currentTransform.k;

    // Find which segment this position corresponds to
    let targetIndex = -1;
    let targetX = 0;

    if ($appState?.timelineItems) {
      const items = $appState.timelineItems;

      for (let i = 0; i < items.length; i++) {
        const itemStartX = items[i].startOffset * originalPathWidth * scaleX;
        const itemEndX = itemStartX + items[i].size * originalPathWidth * scaleX;

        if (timelineX >= itemStartX && timelineX <= itemEndX) {
          // Mouse is over this segment
          const midPoint = itemStartX + (itemEndX - itemStartX) / 2;

          if (timelineX < midPoint) {
            // Drop before this segment
            targetIndex = i;
            targetX = itemStartX;
          } else {
            // Drop after this segment
            targetIndex = i + 1;
            targetX = itemEndX;
          }
          break;
        }
      }

      // If no segment found, place at the end
      if (targetIndex === -1 && items.length > 0) {
        targetIndex = items.length;
        const lastItem = items[items.length - 1];
        targetX = (lastItem.startOffset + lastItem.size) * originalPathWidth * scaleX;
      }
    }

    dropIndicatorIndex = targetIndex;
    // Convert back to screen coordinates for rendering
    // targetX is in timeline coordinates, convert to screen coordinates
    dropIndicatorX = targetX * currentTransform.k + currentTransform.x;
  }

  function handleDragEnd(
    event: CustomEvent<{
      index: number;
      endPos: { x: number; y: number };
      dragDistance: number;
      event: d3.D3DragEvent<SVGGElement, unknown, d3.SubjectPosition>;
    }>
  ) {
    console.log(`%cHERE LINE :252 %c`, 'color: yellow; font-weight: bold', '');

    if (!isDragging) return;

    const { index, endPos, dragDistance } = event.detail;

    console.log(`Timeline: Ended dragging segment ${index} to position ${dropIndicatorIndex}`);

    // Perform the reorder if we have a valid drop position
    if (
      dropIndicatorIndex >= 0 &&
      dropIndicatorIndex !== index &&
      dropIndicatorIndex !== index + 1 &&
      $appState?.timelineItems
    ) {
      console.log(`Reordering: moving segment ${index} to position ${dropIndicatorIndex}`);

      // Create a copy of the timeline items array and perform the reorder
      const items = [...$appState.timelineItems];
      const draggedItem = items[index];

      // Remove the dragged item from its current position
      items.splice(index, 1);

      // Insert it at the new position (adjust index if moving forward)
      const insertIndex = dropIndicatorIndex > index ? dropIndicatorIndex - 1 : dropIndicatorIndex;
      items.splice(insertIndex, 0, draggedItem);

      // Build array for Rust backend: { id, index }
      const updates = items.map((item, newIndex) => ({
        id: item.id, // Assuming timeline items have an id field
        index: newIndex,
      }));

      console.log('Timeline reorder updates:', updates);

      // Create progress channel for the reorder operation
      const onEvent = generateProgressChannel<SortAudioEvent>(Channel, {
        started: data => {
          console.log('Timeline reorder started');
        },
        progress: data => {
          console.log('Timeline reorder progress:', data);
        },
        finished: data => {
          // appState.update((state)=>{
          //   const allFiles = getAllFiles(state.sections);
          //   allFiles.forEach((f)=>{
          //     const newIndex = data.value
          //   })

          // })
          console.log('Timeline reorder finished');
        },
      });

      // Call backend update_sorting function similar to Sources.svelte
      invokeWithPerf<[string, number][]>('update_sorting', { updates, onEvent })
        .then(newOrder => {
          console.log('Received new order from backend:', newOrder);

          // Update inputs after state change
          updateInputs($appState.sections);

          // Use the reusable index syncing function
          if (newOrder.ok && newOrder.value) {
            applySyncIndexes(newOrder.value);
          }
        })
        .catch(error => {
          console.error('Failed to reorder timeline items:', error);
        });
      console.log('Final sections after update:', get(appState).sections[0].files);
      // console.log('Timeline reorder completed:', newOrder);
    }

    // Reset drag state
    isDragging = false;
    draggedSegmentIndex = -1;
    dropIndicatorIndex = -1;
    dropIndicatorX = 0;
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
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    on:click={e => {
      handleClick(e);
    }}
    on:keydown={handleKeyDown}
    bind:this={container}
    style="width: 100%;"
    tabindex="0"
    role="region"
    aria-label="Timeline"
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
                  size={timelineItem.size}
                  label={formatFileName(timelineItem.fileName)}
                  {originalPathWidth}
                  zoomTransform={currentTransform}
                  itemType={timelineItem.type}
                  id={timelineItem.id}
                  active={timelineItem.active ?? true}
                  isSelected={selectedSegments.has(i)}
                  onSegmentSelect={selectSegment}
                  onSegmentToggle={toggleSegmentSelection}
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
          {xScale}
          {scaleX}
          items={$appState?.timelineItems}
          {originalPathWidth}
          {currentTransform}
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
            {formatFileName(item.fileName || 'Unknown')} | Start: {(item.startOffset * 100).toFixed(
              1
            )}% | Size: {(item.size * 100).toFixed(1)}% | X: {(
              item.startOffset *
              originalPathWidth *
              scaleX
            ).toFixed(0)}-{((item.startOffset + item.size) * originalPathWidth * scaleX).toFixed(
              0
            )}px
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
