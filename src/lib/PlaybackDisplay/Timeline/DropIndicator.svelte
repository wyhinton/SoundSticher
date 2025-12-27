<script lang="ts">
  export let isDragging: boolean;
  export let dropIndicatorIndex: number;
  export let dropIndicatorX: number;
  export let arrowHeadY: number = 0;
  export let arrowHeadSize: number = 6;
  export let debugShowDropLine: boolean = false;
</script>

<!-- Drop indicator line -->
{#if (isDragging && dropIndicatorIndex >= 0) || debugShowDropLine}
  <line
    x1={debugShowDropLine ? 100 : dropIndicatorX}
    y1={-20}
    x2={debugShowDropLine ? 100 : dropIndicatorX}
    y2={130}
    stroke="#00BFFF"
    stroke-width={2}
    stroke-dasharray={`4 2`}
    opacity="0.8"
    pointer-events="none"
    class="drop-indicator-line"
  />
  <!-- Drop indicator arrow at top -->
  <polygon
    points={`${(debugShowDropLine ? 100 : dropIndicatorX) - arrowHeadSize},${arrowHeadY} ${(debugShowDropLine ? 100 : dropIndicatorX) + arrowHeadSize},${arrowHeadY} ${debugShowDropLine ? 100 : dropIndicatorX},${arrowHeadY + arrowHeadSize + 1}`}
    fill="#00BFFF"
    opacity="1"
    pointer-events="none"
    class="drop-indicator-arrow"
  />
{/if}

<style>
  .drop-indicator-arrow {
    animation: bob 1.2s ease-in-out infinite;
    transform-origin: center;
  }

  @keyframes bob {
    0%,
    100% {
      transform: translateY(0px);
    }
    50% {
      transform: translateY(3px);
    }
  }
</style>
