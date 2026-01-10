<script lang="ts">
  import type { ZoomTransform } from 'd3-zoom';
  import type { TimelineItem } from '../../state/state.svelte';
  import {
    getDisplayName,
    getItemSize,
    getItemColor,
    getItemTextColor,
    shouldShowLabel,
  } from '../../utils/timelineHelpers';
  import * as d3 from 'd3';

  export let items: TimelineItem[] = [];
  export let originalPathWidth: number;
  export let currentTransform: ZoomTransform;
  export let scaleX: number;
  export let isDragging: boolean;
  export let segmentsToMove: number[] = [];
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
    return xPos;
  });
  $: labelYArr = items.map(() => 7); // same Y for all
  $: rectXArr = items.map(t => {
    const rectX = t.startOffset * originalPathWidth * currentTransform.k * scaleX;
    return rectX;
  });
  $: rectWidthArr = items.map(
    t => getItemSize(t) * originalPathWidth * scaleX * currentTransform.k
  );
  $: nameArr = items.map(item => getDisplayName(item));
  $: bgColorArr = items.map(item => getItemColor(item));
  $: textColorArr = items.map(item => getItemTextColor(item));
  $: showLabelArr = items.map(item => shouldShowLabel(item));

  // Function to convert RGB color and darken it
  function darkenColor(rgbColor: string, factor: number = 0.6): string {
    // Parse RGB values from string like "rgb(255, 0, 0)" or "#ffffff"
    let r, g, b;

    if (rgbColor.startsWith('#')) {
      // Handle hex format
      const hex = rgbColor.replace('#', '');
      r = parseInt(hex.substr(0, 2), 16) / 255;
      g = parseInt(hex.substr(2, 2), 16) / 255;
      b = parseInt(hex.substr(4, 2), 16) / 255;
    } else if (rgbColor.startsWith('rgb')) {
      // Handle rgb(r, g, b) format
      const match = rgbColor.match(/rgb\((\d+),\s*(\d+),\s*(\d+)\)/);
      if (match) {
        r = parseInt(match[1]) / 255;
        g = parseInt(match[2]) / 255;
        b = parseInt(match[3]) / 255;
      } else {
        return rgbColor; // Return original if can't parse
      }
    } else {
      return rgbColor; // Return original if unknown format
    }

    // Convert RGB to HSV
    const max = Math.max(r, g, b);
    const min = Math.min(r, g, b);
    const v = max;
    const s = max === 0 ? 0 : (max - min) / max;

    let h = 0;
    if (max !== min) {
      switch (max) {
        case r:
          h = (g - b) / (max - min);
          break;
        case g:
          h = 2 + (b - r) / (max - min);
          break;
        case b:
          h = 4 + (r - g) / (max - min);
          break;
      }
      h = h * 60;
      if (h < 0) h += 360;
    }

    // Darken by reducing value (brightness) significantly
    const newV = Math.max(0, v * (1 - factor));

    // Convert HSV back to RGB for better control
    const c = newV * s;
    const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
    const m = newV - c;

    let r2, g2, b2;
    if (h >= 0 && h < 60) {
      r2 = c;
      g2 = x;
      b2 = 0;
    } else if (h >= 60 && h < 120) {
      r2 = x;
      g2 = c;
      b2 = 0;
    } else if (h >= 120 && h < 180) {
      r2 = 0;
      g2 = c;
      b2 = x;
    } else if (h >= 180 && h < 240) {
      r2 = 0;
      g2 = x;
      b2 = c;
    } else if (h >= 240 && h < 300) {
      r2 = x;
      g2 = 0;
      b2 = c;
    } else {
      r2 = c;
      g2 = 0;
      b2 = x;
    }

    const finalR = Math.round((r2 + m) * 255);
    const finalG = Math.round((g2 + m) * 255);
    const finalB = Math.round((b2 + m) * 255);

    return `rgb(${finalR}, ${finalG}, ${finalB})`;
  }
</script>

<g class="clip-labels">
  {#each items as t, i}
    {#if showLabelArr[i] && rectXArr[i] !== undefined && rectWidthArr[i] !== undefined}
      <g
        transform={`translate(${currentTransform.x}, 0)`}
        cursor={isDragging ? 'grabbing' : 'grab'}
        class:dragging={segmentsToMove.includes(i)}
      >
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
          fill={segmentsToMove.includes(i) ? darkenColor(bgColorArr[i]) : bgColorArr[i]}
          clip-path={`url(#header-clip-${i})`}
          class="draggable-header"
        />
        <line
          x1={rectXArr[i]}
          y1={0}
          x2={rectXArr[i]}
          y2={80}
          stroke={textColorArr[i]}
          stroke-width={1.5}
        />
        <clipPath id={`clip-${i}`}>
          <rect x={rectXArr[i]} y={0} width={rectWidthArr[i]} height="80" stroke="red" />
        </clipPath>

        <!-- Text -->
        <text
          x={rectXArr[i] + textXOffset}
          y={labelYArr[i]}
          dominant-baseline="middle"
          fill={textColorArr[i]}
          font-size={fontSize}
          font-family="monospace"
          font-weight="bold"
          pointer-events="none"
          clip-path={`url(#clip-${i})`}
        >
          {nameArr[i]}
        </text>
      </g>
    {/if}
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
