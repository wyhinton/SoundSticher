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

  export let items: TimelineItem[] = [];
  export let originalPathWidth: number;
  export let currentTransform: ZoomTransform;
  export let scaleX: number;
  export let isDragging: boolean;
  export let segmentsToMove: number[] = [];

  /* ============================================================================
   * Layout constants
   * ============================================================================
   */

  const fontSize = 11;
  const paddingY = 2;
  const headerHeight = fontSize + paddingY * 2;
  const textXOffset = 5;
  const rowGap = 0;

  const rowHeight = headerHeight + rowGap;

  /* ============================================================================
   * CHILD LABEL DERIVED ARRAYS (mostly unchanged)
   * ============================================================================
   */

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

  /* ============================================================================
   * PARENT LABEL DERIVATION (NEW)
   * ============================================================================
   */

  type ParentLabel = {
    id: string;
    name: string;
    startOffset: number;
    endOffset: number;
    depth: number;
  };

  $: parentLabels = (() => {
    const map = new Map<string, ParentLabel>();

    for (const item of items) {
      if (!item.parentId) continue;

      const parent = items.find(i => i.id === item.parentId);
      if (!parent || parent.kind !== 'merge') continue;

      let entry = map.get(parent.id);

      const start = item.startOffset;
      const end = item.startOffset + getItemSize(item);

      if (!entry) {
        map.set(parent.id, {
          id: parent.id,
          name: getDisplayName(parent),
          startOffset: start,
          endOffset: end,
          depth: parent.depth,
        });
      } else {
        entry.startOffset = Math.min(entry.startOffset, start);
        entry.endOffset = Math.max(entry.endOffset, end);
      }
    }

    return [...map.values()];
  })();

  $: parentRects = parentLabels.map(p => ({
    ...p,
    x: p.startOffset * originalPathWidth * scaleX * currentTransform.k,
    width: (p.endOffset - p.startOffset) * originalPathWidth * scaleX * currentTransform.k,
    y: p.depth * rowHeight,
  }));

  /* ============================================================================
   * UTIL
   * ============================================================================
   */

  function isParentDragging(parentId: string): boolean {
    return segmentsToMove.some(i => items[i]?.parentId === parentId);
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
</style>
