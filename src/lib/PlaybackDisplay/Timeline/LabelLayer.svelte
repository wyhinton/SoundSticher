<script lang="ts">
  import type { ZoomTransform } from 'd3-zoom';
  import { selectionService } from '../../state/selection.svelte';
  import type { TimelineItem } from '../../state/state.svelte';
  import {
    getDisplayName,
    getItemColor,
    getItemSize,
    getItemTextColor,
    shouldShowLabel,
  } from '../../utils/timelineHelpers';

  export let items: TimelineItem[] = [];
  export let originalPathWidth: number;
  export let currentTransform: ZoomTransform;
  export let scaleX: number;
  export let isDragging: boolean;
  export let segmentsToMove: number[] = [];

  const fontSize = 11;
  const paddingY = 2;
  const headerHeight = fontSize + paddingY * 2;
  const textXOffset = 5;
  const rowGap = 0;

  const rowHeight = headerHeight + rowGap;

  $: rectXArr = items.map(t => t.startOffset * originalPathWidth * scaleX * currentTransform.k);

  $: rectWidthArr = items.map(
    t => getItemSize(t) * originalPathWidth * scaleX * currentTransform.k
  );

  $: nameArr = items.map(item => getDisplayName(item));
  $: bgColorArr = items.map(item => getItemColor(item));
  $: textColorArr = items.map(item => getItemTextColor(item));
  $: showLabelArr = items.map(item => shouldShowLabel(item));

  // Vertical stacking: child labels sit BELOW their parents
  $: labelYArr = items.map(item => (item.depth + 0) * rowHeight);

  // Handle clicks on labels for selection (only for depth > 1)
  function handleLabelClick(event: MouseEvent, item: TimelineItem, index: number) {
    console.log(`%cHERE LINE :45 %c`, 'color: yellow; font-weight: bold', '');
    console.log(item);

    // Convert item.id to number for selection system (assuming it's numeric)
    const itemId = index;
    console.log(itemId);
    if (isNaN(itemId)) {
      console.warn('Cannot select item with non-numeric ID:', item.id);
      return;
    }

    console.log(itemId);
    // Use the selection service to handle the click with modifier key support
    selectionService.handleClick(itemId, {
      isMultiSelect: event.ctrlKey || event.metaKey,
      isShiftSelect: event.shiftKey,
      source: 'timeline',
    });
  }
</script>

<g class="clip-labels">
  {#each items as t, i}
    {#if showLabelArr[i] && rectXArr[i] !== undefined && rectWidthArr[i] !== undefined}
      <g
        transform={`translate(${currentTransform.x}, 0)`}
        class:dragging={segmentsToMove.includes(i)}
      >
        <clipPath id={`header-clip-${i}`}>
          <path
            d={`
              M ${rectXArr[i]} ${labelYArr[i] + 2}
              Q ${rectXArr[i]} ${labelYArr[i]} ${rectXArr[i] + 2} ${labelYArr[i]}
              L ${rectXArr[i] + rectWidthArr[i] - 2} ${labelYArr[i]}
              Q ${rectXArr[i] + rectWidthArr[i]} ${labelYArr[i]} ${rectXArr[i] + rectWidthArr[i]} ${labelYArr[i] + 2}
              L ${rectXArr[i] + rectWidthArr[i]} ${labelYArr[i] + headerHeight}
              L ${rectXArr[i]} ${labelYArr[i] + headerHeight}
              Z
            `}
          />
        </clipPath>

        <rect
          x={rectXArr[i]}
          y={labelYArr[i]}
          width={rectWidthArr[i]}
          height={headerHeight}
          fill={bgColorArr[i]}
          clip-path={`url(#header-clip-${i})`}
          class="draggable-header"
          class:selectable={t.depth && t.depth > 1}
          style:cursor={t.depth && t.depth > 1
            ? isDragging
              ? 'grabbing'
              : 'pointer'
            : isDragging
              ? 'grabbing'
              : 'grab'}
          on:click={e => handleLabelClick(e, t, i)}
        />

        <text
          x={rectXArr[i] + textXOffset}
          y={labelYArr[i] + headerHeight / 2}
          dominant-baseline="middle"
          fill={textColorArr[i]}
          font-size={fontSize}
          font-family="monospace"
          font-weight="bold"
          pointer-events="none"
        >
          {nameArr[i]}
        </text>
      </g>
    {/if}
  {/each}
</g>

<style>
  .draggable-header {
    transition: fill 0.15s ease;
  }

  .draggable-header:hover {
    fill: rgb(58, 165, 255);
  }

  .draggable-header.selectable:hover {
    fill: rgb(34, 197, 94);
    opacity: 0.9;
  }

  .draggable-header.selectable {
    transition:
      fill 0.15s ease,
      opacity 0.15s ease;
  }
</style>
