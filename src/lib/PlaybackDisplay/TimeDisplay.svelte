<script lang="ts">
  import { formatMilliseconds } from '../utils/format';
  import { opPlaybackState, opIsPlaying, opIsPaused } from '../state/opPlaybackService';
  import { operationDuration } from '../state/waveformCache';

  let playHeadPosition = 0;

  // Update playhead position from operation playback state
  $: playHeadPosition = $opPlaybackState.positionSeconds;

  // Reactive current duration from operation system
  $: currentDuration = $operationDuration;

  // Reactive total length from operation system
  $: totalLength = $opPlaybackState.durationSeconds;

  // Reactive play state from operation system
  $: isCurrentlyPlaying = $opIsPlaying && !$opIsPaused;
</script>

<!-- Current Time Display -->
<div class="time-display my-1">
  <div class="current-time" class:playing={isCurrentlyPlaying}>
    {formatMilliseconds(playHeadPosition * 1000)}
  </div>
  <div class="time-separator">/</div>
  <div class="total-time">
    {totalLength > 0 ? formatMilliseconds(totalLength * 1000) : '0:00.000'}
  </div>
</div>

<style>
  .time-display {
    display: flex;
    align-items: center;
    gap: 6px;
    background: #1a202c;
    border: 1px solid #4a5568;
    border-radius: 4px;
    padding: 4px 8px;
    font-family: 'Courier New', monospace;
    font-weight: 700;
    font-size: 24px;
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.3);
  }

  .current-time {
    color: #cbd5e0;
    min-width: 60px;
    text-align: right;
    transition: color 0.2s ease;
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
</style>
