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
  export let label: string;
  export let scaleX: number;
  export let originalPathWidth: number;
  export let zoomTransform: d3.ZoomTransform; // pass currentTransform from parent
  export let itemType: TimelineItemType;
  import { createEventDispatcher, onMount } from 'svelte';
  export let DEBUG_MODE: boolean;
  export let id: string | undefined;

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
</script>

<!-- Timeline segment using SVG group with drag handle -->
<g
  bind:this={gElement}
  transform={`scale(${scaleX}, 1)`}
  class="segment-rect"
  class:dragging={isDragging}
>
  <foreignObject x={rectX} y={-20} width={rectWidth} height="150">
    <div
      bind:this={timelineDiv}
      xmlns="http://www.w3.org/1999/xhtml"
      class="timeline-segment-div"
      class:hovered={$hoveredSourceItem == index}
      class:dragging={isDragging}
      style="
        width: 100%;
        height: 150px;
        background-color: rgba(0, 200, 255, {$hoveredSourceItem == index ? 0.4 : $fillAlpha});
        box-sizing: border-box;
        pointer-events: all;
        cursor: {isDragging ? 'grabbing' : 'grab'};
      "
    >
      <!-- <div
        on:mouseenter={() => hoveredTimelineItem.set(index)}
        on:mouseleave={() => hoveredTimelineItem.set(null)}
        class="segment-head"
      ></div> -->

      {#if DEBUG_MODE}
        dragging: {isDragging}
        id: {id}
      {/if}
    </div>
  </foreignObject>
</g>

<style>
  .segment-head {
    border: 1px solid red;
    width: 100%;
    height: 10px;
    position: absolute;
    top: 0;
    left: 0;
    background: rgb(48, 145, 241);
  }
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
    box-shadow: 0 4px 8px rgba(0, 200, 255, 0.2);
  }

  .timeline-segment-div.hovered {
    background-color: rgba(0, 200, 255, 0.4) !important;
  }

  /* Drag state styling - segment being dragged gets highlighted */
  .timeline-segment-div.dragging {
    background-color: rgba(0, 200, 255, 0.6) !important;
    border-color: rgba(0, 200, 255, 0.8) !important;
    box-shadow: 0 4px 12px rgba(0, 200, 255, 0.4);
    opacity: 0.9;
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
</style>
