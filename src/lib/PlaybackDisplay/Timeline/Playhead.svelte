<script lang="ts">
  import type * as d3 from 'd3';
  import { appState } from '../../state/state.svelte';
  import { scale } from 'svelte/transition';

  export let playHeadX: number;
  export let currentTransform: d3.ZoomTransform;
  export let contentScaleY: number;

  $: playheadCol = $appState.playingCombined ? '#68d391' : 'white';
</script>

<!-- CURRENT PLAYHEAD -->
<g class="playhead-indicator">
  <!-- Vertical line -->
  <line
    x1={playHeadX}
    y1={0}
    x2={playHeadX}
    y2={80}
    stroke={playheadCol}
    stroke-width={1.5 / currentTransform.k}
  />
  <!-- Arrow head at top -->
  <path
    d={`M ${playHeadX - 6 / currentTransform.k} 0 
        L ${playHeadX} 12  
        L ${playHeadX + 6 / currentTransform.k} 0 
        Z`}
    transform={`scale(1, ${1 / contentScaleY})`}
    fill={playheadCol}
  />
</g>
