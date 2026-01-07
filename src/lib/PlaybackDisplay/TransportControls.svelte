<script lang="ts">
  import { appState, durationSeconds } from '$lib/state/state.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { invokeWithPerf } from '$lib/state/performance';
  import {
    opPlaybackService,
    opPlaybackState,
    opPlaybackProgress,
    opIsPlaying,
    opIsPaused,
  } from '$lib/state/opPlaybackService';
  import { operationDuration } from '$lib/state/waveformCache';

  // Props to control which system to use
  export let useOperationSystem = true;

  let bufferingProgress = 0;
  let playHeadPosition = 0;

  listen<number>('buffering-progress', e => {
    bufferingProgress = e.payload;
  });

  // Listen to legacy timeline progress for the legacy system
  listen<number>('timeline-progress', event => {
    if (!useOperationSystem) {
      playHeadPosition = event.payload * $durationSeconds;
    }
  });

  // Listen to operation playback progress when using operation system
  $: if (useOperationSystem && $opPlaybackProgress !== undefined) {
    playHeadPosition = $opPlaybackState.positionSeconds;
  }

  // Reactive current duration based on active system
  $: currentDuration = useOperationSystem ? $operationDuration : $durationSeconds;

  // Reactive play state
  $: isCurrentlyPlaying = useOperationSystem
    ? $opIsPlaying && !$opIsPaused
    : $appState.playingCombined;

  // Reactive pause state
  $: isCurrentlyPaused = useOperationSystem
    ? $opIsPaused
    : !$appState.playingCombined && playHeadPosition > 0;

  // Reactive loop state
  $: isLoopEnabled = useOperationSystem
    ? $opPlaybackState.loopEnabled
    : $appState.isLoopingTimelineAudio;

  // Transport control functions
  async function handlePlay() {
    try {
      if (useOperationSystem) {
        await opPlaybackService.play(playHeadPosition);
      } else {
        await invoke('play_timeline_audio', { start_seconds: playHeadPosition });
      }
    } catch (error) {
      console.error('Error playing audio:', error);
    }
  }

  async function handlePause() {
    try {
      if (useOperationSystem) {
        await opPlaybackService.pause();
      } else {
        await invoke('pause_timeline_audio');
      }
    } catch (error) {
      console.error('Error pausing audio:', error);
    }
  }

  async function handleResume() {
    try {
      if (useOperationSystem) {
        await opPlaybackService.resume();
      } else {
        // Legacy system doesn't have explicit resume, just play from current position
        await invoke('play_timeline_audio', { start_seconds: playHeadPosition });
      }
    } catch (error) {
      console.error('Error resuming audio:', error);
    }
  }

  async function handleStop() {
    try {
      if (useOperationSystem) {
        await opPlaybackService.stop();
      } else {
        await invoke('stop_timeline_audio');
      }
    } catch (error) {
      console.error('Error stopping audio:', error);
    }
  }

  async function handleSkipToStart() {
    try {
      if (useOperationSystem) {
        await opPlaybackService.seek(0);
      } else {
        await invoke('set_timeline_play_position', { position: 0.0 });
      }
    } catch (error) {
      console.error('Error skipping to start:', error);
    }
  }

  async function handleSkipToEnd() {
    try {
      if (useOperationSystem) {
        await opPlaybackService.seek(currentDuration);
      } else {
        await invoke('set_timeline_play_position', { position: 1.0 });
      }
    } catch (error) {
      console.error('Error skipping to end:', error);
    }
  }

  async function toggleLoop() {
    try {
      if (useOperationSystem) {
        await opPlaybackService.setLoop(!$opPlaybackState.loopEnabled);
      } else {
        // Toggle the frontend state
        appState.update(s => {
          s.isLoopingTimelineAudio = !s.isLoopingTimelineAudio;
          return s;
        });

        // Communicate with backend to toggle the actual LoopingSamplesBuffer
        await invoke('set_timeline_loop_enabled', {
          loop_enabled: $appState.isLoopingTimelineAudio,
        });
      }
    } catch (error) {
      console.error('Error toggling loop:', error);
      // Revert the state change if backend call fails (legacy system only)
      if (!useOperationSystem) {
        appState.update(s => {
          s.isLoopingTimelineAudio = !s.isLoopingTimelineAudio;
          return s;
        });
      }
    }
  }

  // Handle play/pause toggle
  async function handlePlayPause() {
    if (isCurrentlyPlaying) {
      await handlePause();
    } else if (isCurrentlyPaused && useOperationSystem) {
      await handleResume();
    } else {
      await handlePlay();
    }
  }

  listen('audio-playback-ended', e => {
    // Only handle for legacy system - operation system handles looping internally
    if (!useOperationSystem && $appState.isLoopingTimelineAudio) {
      invokeWithPerf('stop_timeline_audio', { start_seconds: 0 }).then(() => {
        invokeWithPerf('play_timeline_audio');
      });
    }
  });
</script>

{#snippet transportButton(
  icon: string,
  title: string,
  onclick: () => void,
  buttonClass: string = '',
  active: boolean = false,
  disabled: boolean = false
)}
  <button
    class="btn btn-transport {buttonClass}"
    class:active
    {title}
    aria-label={title}
    on:click={onclick}
    {disabled}
  >
    <i class="fa-solid {icon}"></i>
  </button>
{/snippet}

<!-- Audacity-style Transport Controls -->
<div class="transport-controls d-flex align-items-center gap-2 py-2 px-2">
  {@render transportButton('fa-backward-step', 'Skip to Start', handleSkipToStart)}
  {@render transportButton('fa-play', 'Play', handlePlayPause, 'btn-play', isCurrentlyPlaying)}
  {@render transportButton(
    'fa-pause',
    'Pause',
    handlePause,
    'btn-pause',
    isCurrentlyPaused
  )}
  {@render transportButton('fa-stop', 'Stop', handleStop, 'btn-stop')}
  {@render transportButton('fa-forward-step', 'Skip to End', handleSkipToEnd)}
  {@render transportButton(
    'fa-repeat',
    'Loop',
    toggleLoop,
    'btn-loop',
    isLoopEnabled
  )}
</div>

<style>
  .transport-controls {
    background: linear-gradient(to bottom, #2c3e50, #34495e);
    border: 1px solid #1a252f;
    border-radius: 4px;
    box-shadow: inset 0 1px 2px rgba(255, 255, 255, 0.1);
  }

  .btn-transport {
    width: 32px;
    height: 32px;
    border: 1px solid #4a5568;
    background: linear-gradient(to bottom, #4a5568, #2d3748);
    color: #e2e8f0;
    border-radius: 3px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    font-size: 14px;
    transition: all 0.1s ease;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
  }

  .btn-transport:hover {
    background: linear-gradient(to bottom, #5a6578, #3d4852);
    border-color: #718096;
    transform: translateY(-1px);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.4);
    color: #f7fafc;
  }

  .btn-transport:active {
    background: linear-gradient(to bottom, #2d3748, #1a202c);
    transform: translateY(0);
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.4);
  }

  .btn-transport.active {
    background: linear-gradient(to bottom, #3182ce, #2c5282);
    color: white;
    border-color: #2a4365;
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.3);
  }

  .btn-play.active {
    background: linear-gradient(to bottom, #38a169, #2f855a);
    border-color: #276749;
  }

  .btn-pause.active {
    background: linear-gradient(to bottom, #d69e2e, #b7791f);
    border-color: #975a16;
    color: white;
  }

  .btn-record {
    background: linear-gradient(to bottom, #4a5568, #2d3748);
  }

  .btn-record:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .record-icon {
    color: #f56565;
  }

  .btn-loop.active {
    background: linear-gradient(to bottom, #319795, #2c7a7b);
    border-color: #234e52;
    color: white;
  }

  .info-panel {
    background: #2d3748;
    border: 1px solid #1a252f;
    border-top: none;
    border-radius: 0 0 4px 4px;
    font-size: 11px;
  }

  .info-divider {
    width: 1px;
    height: 20px;
    background: #4a5568;
    margin: 0 4px;
  }

  .info-item {
    display: flex;
    gap: 4px;
    align-items: center;
  }

  .info-label {
    color: #a0aec0;
    font-weight: 500;
  }

  .info-value {
    color: #e2e8f0;
    font-family: 'Courier New', monospace;
    font-weight: 600;
  }

  .skeleton {
    background: linear-gradient(90deg, #e9ecef 25%, #f8f9fa 50%, #e9ecef 75%);
    background-size: 200% 100%;
    animation: loading 1.5s infinite;
    border-radius: 2px;
  }

  @keyframes loading {
    0% {
      background-position: 200% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }
</style>
