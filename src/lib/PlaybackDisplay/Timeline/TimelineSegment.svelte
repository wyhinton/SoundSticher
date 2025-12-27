<script lang="ts">
  import * as d3 from 'd3';
  import {
    hoveredSourceItem,
    hoveredTimelineItem,
    type TimelineItemType,
  } from '../../state/state.svelte';
  import { tweened } from 'svelte/motion';
  import { cubicOut } from 'svelte/easing';
  export let index: number;
  export let startOffset: number;
  export let size: number;
  export let scaleX: number;
  export let originalPathWidth: number;
  export let zoomTransform: d3.ZoomTransform; // pass currentTransform from parent
  import { createEventDispatcher, onMount } from 'svelte';
  export let DEBUG_MODE: boolean;
  export let id: string | undefined;
  export let isSelected: boolean = false;
  export let isBeingDragged: boolean = false;
  export let active: boolean = true;
  export let canBeDragged: boolean = true;
  export let itemColor: string = '#3091f1';
  export let svgPath: string | null = null;
  export let onSegmentSelect: ((index: number, isShiftSelect?: boolean) => void) | undefined =
    undefined;
  export let onSegmentToggle: ((index: number) => void) | undefined = undefined;

  const dispatch = createEventDispatcher();

  // Compute zoomed positions
  $: rectX = startOffset * originalPathWidth;
  $: rectWidth = size * originalPathWidth;

  // Adjusted X for text: apply only translate, not scale
  $: textX = zoomTransform.applyX(rectX); // applies translate & scale to position

  // Animated fill alpha (0 → 0.15 when mounted)
  const fillAlpha = tweened(0.15, {
    duration: 400,
    easing: cubicOut,
  });

  // Drag state
  let isDragging = false;
  let gElement: SVGGElement;
  let timelineDiv: HTMLDivElement;

  // Set up d3 drag behavior
  onMount(() => {
    fillAlpha.set(0.0);

    // Configure d3 drag behavior - on the entire timeline div
    const drag = d3
      .drag<HTMLDivElement, unknown>()
      .on('start', function (event) {
        isDragging = true;
        console.log(`Started dragging segment ${index} with d3.drag`);

        // Dispatch drag start event
        dispatch('dragStart', {
          index,
          startPos: { x: event.sourceEvent.clientX, y: event.sourceEvent.clientY },
          segmentId: index,
        });
      })
      .on('drag', function (event) {
        if (!isDragging) return;

        // Dispatch drag move event with current mouse position
        dispatch('dragMove', {
          index,
          mousePos: { x: event.sourceEvent.clientX, y: event.sourceEvent.clientY },
          dragDistance: event.dx,
          event: event,
        });
      })
      .on('end', function (event) {
        if (!isDragging) return;

        console.log(`Ended dragging segment ${index} with d3.drag`);
        isDragging = false;

        // Dispatch drag end event
        dispatch('dragEnd', {
          index,
          endPos: { x: event.sourceEvent.clientX, y: event.sourceEvent.clientY },
          dragDistance: event.dx,
          event: event,
        });
      });

    // Apply drag behavior to the entire timeline div
    d3.select(timelineDiv).call(drag);
  });

  function handleSegmentClick(event: MouseEvent) {
    // Prevent event from bubbling to timeline container
    event.stopPropagation();

    if (event.ctrlKey || event.metaKey) {
      // Multi-select with Ctrl/Cmd
      onSegmentToggle?.(index);
    } else if (event.shiftKey) {
      // Range select with Shift
      onSegmentSelect?.(index, true);
    } else {
      // Single select
      onSegmentSelect?.(index, false);
    }
  }

  function handleSegmentKeyDown(event: KeyboardEvent) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      if (event.ctrlKey || event.metaKey) {
        onSegmentToggle?.(index);
      } else if (event.shiftKey) {
        onSegmentSelect?.(index, true);
      } else {
        onSegmentSelect?.(index, false);
      }
    }
  }
</script>

<!-- Timeline segment using SVG group with drag handle -->
<g
  bind:this={gElement}
  transform={`scale(${scaleX}, 1)`}
  class="segment-rect"
  class:dragging={isDragging}
>
  <!-- Render waveform path if available -->
  {#if svgPath}
    <path
      d={svgPath}
      fill="none"
      stroke="#3091f1"
      stroke-width="1"
      transform={`translate(${rectX}, 0)`}
      class="waveform-path"
      class:dragging={isDragging}
    />
  {/if}

  <foreignObject x={rectX} y={-20} width={rectWidth} height="150">
    <div
      bind:this={timelineDiv}
      xmlns="http://www.w3.org/1999/xhtml"
      class="timeline-segment-div"
      class:hovered={$hoveredSourceItem == index}
      class:selected={isSelected}
      class:inactive={!active}
      class:dragging={isDragging}
      class:non-draggable={!canBeDragged}
      data-timeline-segment
      data-segment-index={index}
      data-segment-id={id}
      data-segment-active={active}
      on:click={handleSegmentClick}
      on:keydown={handleSegmentKeyDown}
      tabindex="0"
      aria-label="Timeline segment {index + 1}"
      style="
        width: 100%;
        height: 150px;
        background-color: {itemColor
        .replace('rgb(', 'rgba(')
        .replace(')', `, ${isSelected ? 0.3 : $hoveredSourceItem == index ? 0.4 : $fillAlpha})`)};
        opacity: {active ? 1.0 : 0.4};
        box-sizing: border-box;
        pointer-events: all;
      "
    ></div>
  </foreignObject>
</g>

<style>
  .timeline-segment-div {
    transition:
      background-color 0.2s ease,
      opacity 0.2s ease,
      transform 0.2s ease,
      box-shadow 0.2s ease;
    position: relative;
    /* border-radius: 4px; */
    display: flex;
    align-items: center;
    justify-content: flex-start;
    padding: 8px;
    font-family: monospace;
    font-size: 12px;
    color: white;
    font-weight: 500;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
  }

  .timeline-segment-div:hover {
    opacity: 0.9;
    transform: translateY(-1px);
  }

  .timeline-segment-div.hovered {
    background-color: rgba(0, 200, 255, 0.4) !important;
  }

  .timeline-segment-div.selected {
    background-color: rgba(59, 130, 246, 0.3) !important;
  }

  .timeline-segment-div.selected:hover {
    background-color: rgba(59, 130, 246, 0.4) !important;
  }

  /* Inactive state styling */
  .timeline-segment-div.inactive {
    background-color: rgba(255, 69, 0, 0.2) !important;
    border: 2px solid rgba(255, 69, 0, 0.6) !important;
    opacity: 0.6;
  }

  .timeline-segment-div.inactive:hover {
    background-color: rgba(255, 69, 0, 0.3) !important;
    opacity: 0.7;
  }

  /* Drag state styling - segment being dragged gets highlighted */
  .timeline-segment-div.dragging {
    background-color: rgba(0, 200, 255, 0.6) !important;
    opacity: 0.9;
  }

  /* Non-draggable items styling */
  .timeline-segment-div.non-draggable {
    opacity: 0.7;
  }

  .timeline-segment-div.non-draggable:hover {
    transform: none; /* Don't lift non-draggable items */
    box-shadow: none;
  }

  /* SVG group drag styling */
  .segment-rect.dragging {
    opacity: 0.8;
  }

  .segment-rect {
    transition: opacity 0.2s ease;
  }

  /* Ensure the foreignObject content renders properly */
  foreignObject {
    overflow: visible;
  }

  /* Waveform path styling */
  :global(.waveform-path) {
    transition: stroke 0.2s ease;
    pointer-events: none;
  }

  :global(.waveform-path.dragging) {
    stroke: rgba(0, 200, 255, 0.9);
  }
</style>
