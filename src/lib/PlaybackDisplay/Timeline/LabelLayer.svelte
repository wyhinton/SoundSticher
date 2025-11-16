<script lang="ts">
  import type { ZoomTransform } from 'd3-zoom';
  import type { TimelineItem } from '../../state/state.svelte';
  import { formatFileName } from '../../utils/format';
  import * as d3 from 'd3';

  export let items: TimelineItem[] = [];
  export let originalPathWidth: number;
  export let currentTransform: ZoomTransform;
  export let scaleX: number;
  export let xScale: d3.ScaleLinear<number, number, never>;
  export let isDragging: boolean;
  // Estimate monospace char width at 10px font size
  const charWidth = 6.2;
  const fontSize = 11;
  const paddingX = 0;
  const paddingY = 2;
  const borderRadius = 2;
  const textXOffset = 5;

  const headerHeight = fontSize + paddingY * 2;
  // Reactive arrays
  $: labelXArr = items.map(t => {
    const xPos = t.startOffset * originalPathWidth;
    let testX = currentTransform.applyX(t.startOffset * originalPathWidth) + paddingX;
    // console.log(testX)*scale
    // testX/=scaleX;
    // return testX * scaleX;
    return xPos;
  });
  //   $: labelXArr = items.map(t => currentTransform.applyX(t.startOffset * originalPathWidth) + paddingX);
  $: labelYArr = items.map(() => 7); // same Y for all
  $: rectXArr = items.map(t => {
    const rectX = t.startOffset * originalPathWidth * currentTransform.k * scaleX;
    return rectX;
  });
  $: rectWidthArr = items.map(t => t.size * originalPathWidth * scaleX * currentTransform.k);
  $: nameArr = items.map(item => formatFileName(item.fileName));

  const textBgColor = 'rgb(48, 145, 241)';
  // const textBgColor = 'rgba(0, 0, 0, 0.6)';
  const textColor = 'rgba(0, 0, 0, 0.6)';
</script>

<g class="clip-labels">
  <!-- <rect
    cursor="grab"
    x={0}
    y={0}
    width={'100%'}
    height={headerHeight}
    class="lable-header-bar"
    stroke="red"
  ></rect> -->
  {#each items as t, i}
    <g transform={`translate(${currentTransform.x}, 0)`} cursor={isDragging ? 'grabbing' : 'grab'}>
      <clipPath id={`header-clip-${i}`}>
        <path
          d={`
          M ${rectXArr[i]} ${borderRadius}
          Q ${rectXArr[i]} 0 ${rectXArr[i] + borderRadius} 0
          L ${rectXArr[i] + rectWidthArr[i] - borderRadius} 0
          Q ${rectXArr[i] + rectWidthArr[i]} 0 ${rectXArr[i] + rectWidthArr[i]} ${borderRadius}
          L ${rectXArr[i] + rectWidthArr[i]} ${headerHeight}
          L ${rectXArr[i]} ${headerHeight}
          Z
        `}
        />
      </clipPath>

      <!-- NEW BACKGROUND -->
      <rect
        x={rectXArr[i]}
        y={0}
        width={rectWidthArr[i]}
        height={headerHeight}
        stroke="blue"
        fill={textBgColor}
        clip-path={`url(#header-clip-${i})`}
        class="draggable-header"
      />
      <line
        x1={rectXArr[i]}
        y1={0}
        x2={rectXArr[i]}
        y2={80}
        stroke={textColor}
        stroke-width={1.5}
      />
      <clipPath id={`clip-${i}`}>
        <rect x={rectXArr[i]} y={0} width={rectWidthArr[i]} height="80" stroke="red" />
      </clipPath>

      <!-- <rect
        x={rectXArr[i]}
        y={0}
        width={rectWidthArr[i]}
        height="80"
        fill-opacity={0.5}
        stroke="red"
      /> -->
      <!-- Background -->
      <!-- <rect
        x={rectXArr[i] + textXOffset}
        y={labelYArr[i] - fontSize / 2 - paddingY}
        width={nameArr[i].length * charWidth + paddingX * 2}
        height={headerHeight}
        fill={textBgColor}
        clip-path={`url(#clip-${i})`}
        rx="2"
        
      /> -->
      <!-- Text -->
      <text
        x={rectXArr[i] + textXOffset}
        y={labelYArr[i]}
        dominant-baseline="middle"
        fill={textColor}
        font-size={fontSize}
        font-family="monospace"
        font-weight="bold"
        pointer-events="none"
        clip-path={`url(#clip-${i})`}
      >
        {nameArr[i]}
      </text>
    </g>
  {/each}
</g>

<style>
  .draggable-header {
    transition:
      fill 0.2s ease,
      stroke 0.2s ease,
      stroke-width 0.2s ease;
  }

  .draggable-header:hover {
    fill: rgb(58, 165, 255) !important;
    stroke: rgb(255, 255, 255) !important;
    stroke-width: 1.5 !important;
  }
</style>
