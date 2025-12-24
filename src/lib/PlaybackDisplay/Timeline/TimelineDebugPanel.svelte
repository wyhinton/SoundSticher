<script lang="ts">
  import { getDisplayName, getItemSize, isItemActive } from '../../utils/timelineHelpers';
  import type { TimelineItem } from './D3TimelineManager';
  import { durationSeconds } from '../../state/state.svelte';

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
  {#if timelineItems && timelineItems.length > 0}
    <div><b>Items ({timelineItems.length}):</b></div>
    {#each timelineItems as item, i}
      <div class="item" class:dragged={i === draggedSegmentIndex}>
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

  .debug .item-type {
    color: #888;
    font-style: italic;
  }
</style>
