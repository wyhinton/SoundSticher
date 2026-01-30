<script lang="ts">
  import type * as d3 from 'd3';
  import { appState } from '../../state/state.svelte';
  import { scale } from 'svelte/transition';

  export let playHeadX: number;
  export let currentTransform: d3.ZoomTransform;
  export let contentScaleY: number;

  let yOff = -4;
  let playHeadWidth = 6;
  let playHeadHeight = 6;
  $: playheadCol = '#8dff2b';
</script>

<!-- CURRENT PLAYHEAD -->
<g class="playhead-indicator">
  <!-- Vertical line -->
  <line
    x1={playHeadX}
    y1={yOff}
    x2={playHeadX}
    y2={80}
    stroke={playheadCol}
    stroke-width={1.5 / currentTransform.k}
  />
  <!-- Arrow head at top -->
  -->
</g>
