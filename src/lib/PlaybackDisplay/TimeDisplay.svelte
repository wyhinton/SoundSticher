<script lang="ts">
  import { formatMilliseconds } from '../utils/format';

  export let compact: boolean = false;
  export let currentPositionSeconds: number = 0;
  export let totalDurationSeconds: number = 0;
  export let isPlaying: boolean = false;
</script>

<!-- Current Time Display -->
<div class="time-display" class:compact>
  <div class="current-time" class:playing={isPlaying}>
    {formatMilliseconds(currentPositionSeconds * 1000)}
  </div>
  <div class="time-separator">/</div>
  <div class="total-time">
    {totalDurationSeconds > 0 ? formatMilliseconds(totalDurationSeconds * 1000) : '0:00.000'}
  </div>
</div>

<style>
  .time-display {
    display: flex;
    align-items: center;
    gap: 6px;
    background: #1a202c;
    border: 1px solid #4a5568;
    padding: 4px 8px;
    font-family: 'Courier New', monospace;
    font-weight: 700;
    font-size: 24px;
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.3);
  }

  .time-display.compact {
    font-size: 12px;
    padding: 2px 6px;
    gap: 3px;
  }

  .current-time {
    color: #cbd5e0;
    min-width: 60px;
    text-align: right;
    transition: color 0.2s ease;
  }

  .time-display.compact .current-time {
    min-width: 40px;
    font-size: 11px;
  }

  .current-time.playing {
    color: #68d391;
  }

  .time-separator {
    color: #a0aec0;
    font-weight: 400;
  }

  .total-time {
    color: #cbd5e0;
    min-width: 60px;
  }

  .time-display.compact .total-time {
    min-width: 40px;
    font-size: 11px;
  }
</style>
