<script lang="ts">
  import { getDisplayName, getItemSize, isItemActive } from '../../utils/timelineHelpers';
  import { durationSeconds, type TimelineItem } from '../../state/state.svelte';
  // import { type TimlineItem}
  // Props passed from Timeline component
  export let isDragging: boolean;
  export let draggedSegmentIndex: number;
  export let dropIndicatorIndex: number;
  export let dropIndicatorX: number;
  export let width: number;
  export let scaleX: number;
  export let playHeadPosition: number;
  export let currentTransform: d3.ZoomTransform;
  export let timelineItems: TimelineItem[] = [];
  export let originalPathWidth: number;
  export let selectedSegments: Set<number>;
  export let lastSelectedIndex: number | null;
  export let segmentsToMove: number[] = [];
</script>

<div class="debug">
  <div>
    <b>Drag:</b>
    {isDragging} |
    <b>Segment:</b>
    {draggedSegmentIndex >= 0 ? draggedSegmentIndex : '-'} |
    <b>Drop:</b>
    {dropIndicatorIndex >= 0 ? dropIndicatorIndex : '-'} |
    <b>DropX:</b>
    {dropIndicatorX.toFixed(0)}px
  </div>
  <div>
    <b>W:</b>
    {width} |
    <b>ScaleX:</b>
    {scaleX.toFixed(2)} |
    <b>Dur:</b>
    {$durationSeconds.toFixed(1)}s |
    <b>PlayPos:</b>
    {playHeadPosition.toFixed(1)}s |
    <b>Zoom:</b>
    {currentTransform.k.toFixed(1)}x |
    <b>Pan:</b>
    {currentTransform.x.toFixed(0)}px
  </div>
  <div>
    <b>Selected ({selectedSegments.size}):</b>
    {#if selectedSegments.size > 0}
      [{Array.from(selectedSegments).join(', ')}] |
    {:else}
      none |
    {/if}
    <b>LastSel:</b>
    {lastSelectedIndex !== null ? lastSelectedIndex : '-'} |
    <b>ToMove:</b>
    {#if segmentsToMove.length > 0}
      [{segmentsToMove.join(', ')}]
    {:else}
      none
    {/if}
  </div>
  {#if timelineItems && timelineItems.length > 0}
    <div><b>Items ({timelineItems.length}):</b></div>
    {#each timelineItems as item, i}
      <div
        class="item d-flex gap-2"
        class:dragged={i === draggedSegmentIndex}
        class:selected={selectedSegments.has(i)}
      >
        <b>#{i}</b>
        <span class="item-type">[{item.type}]</span>
        {getDisplayName(item)} | Start: {(item.startOffset * 100).toFixed(1)}% | Size: {(
          getItemSize(item) * 100
        ).toFixed(1)}% | Active: {isItemActive(item)} | X: {(
          item.startOffset *
          originalPathWidth *
          scaleX
        ).toFixed(0)}-{(
          (item.startOffset + getItemSize(item)) *
          originalPathWidth *
          scaleX
        ).toFixed(0)}px
        {#if item.type === 'audio-file'}
          <div>
            {#if item.svgPath}
              {item.svgPath}
            {:else}
              <span style="color: red;">undefined</span>
            {/if}
          </div>
        {/if}
      </div>
    {/each}
  {:else}
    <div>No items</div>
  {/if}
</div>

<style>
  /* Debug Panel Styles */
  .debug {
    margin-top: 10px;
    background: #000;
    border: 1px solid #333;
    padding: 8px;
    font-family: monospace;
    font-size: 11px;
    color: #fff;
  }

  .debug div {
    margin: 2px 0;
  }

  .debug .item {
    padding: 1px 2px;
    border-radius: 2px;
  }

  .debug .item.dragged {
    background: #333;
    color: #ff0;
  }

  .debug .item.selected {
    background: #006600;
    color: #00ff00;
  }

  .debug .item.dragged.selected {
    background: #663300;
    color: #ffaa00;
  }

  .debug .item-type {
    color: #888;
    font-style: italic;
  }
</style>
