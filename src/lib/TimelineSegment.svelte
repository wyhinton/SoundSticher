<script lang="ts">
import * as d3 from "d3";
  import { hoveredSourceItem, hoveredTimelineItem, type TimelineItemType } from "./state/state.svelte";
  import { tweened } from "svelte/motion";
  import { cubicOut } from "svelte/easing";
  export let index: number;
  export let startOffset: number;
  export let size: number;
  export let label: string;
  export let scaleX: number;
  export let originalPathWidth: number;
  export let zoomTransform: d3.ZoomTransform; // pass currentTransform from parent
  export let itemType: TimelineItemType;

  // Compute zoomed positions
  $: rectX = startOffset * originalPathWidth;
  $: rectWidth = size * originalPathWidth;

  // Adjusted X for text: apply only translate, not scale
  $: textX = zoomTransform.applyX(rectX); // applies translate & scale to position
    //  fill={$hoveredItem==index?'rgba(0, 200, 255, 0.4}':'rgba(0, 200, 255, 0.15}'}0"

      // Animated fill alpha (0 → 0.15 when mounted)
  const fillAlpha = tweened(.15, {
    duration: 400,
    easing: cubicOut
  });

  // Kick off animation on mount
  import { onMount } from "svelte";
  onMount(() => {
    fillAlpha.set(0.0);
  });
</script>

<!-- Rect stays inside zoom-scaled group -->
<g transform={`scale(${scaleX}, 1)`} class="segment-rect">
  <rect
    onmouseenter={()=>hoveredTimelineItem.set(index)}
    onmouseleave={()=>hoveredTimelineItem.set(null)}
    x={rectX}
    y={-20}
    width={rectWidth}
    height="150"
    paint-order="stroke"
    stroke="rgba(0, 200, 255, 0.5)"
    stroke-width=".5"
    fill="rgba(0, 200, 255, 1)"
    stroke-opacity=".2"
    fill-opacity={$hoveredSourceItem == index ? 0.4 : $fillAlpha}

  />
</g>


