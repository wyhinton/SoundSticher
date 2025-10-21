<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { appState, durationSeconds } from '../state/state.svelte';
  import TimelineInfo from './TimelineInfo.svelte';

  let playHeadPosition = 0;

  listen<number>('timeline-progress', event => {
    console.log(event.payload * $durationSeconds);
    playHeadPosition = event.payload * $durationSeconds;
  });

  // Transport control functions
  async function handlePlay() {
    try {
      await invoke('play_timeline_audio', { start_seconds: playHeadPosition });
    } catch (error) {
      console.error('Error playing audio:', error);
    }
  }

  async function handlePause() {
    try {
      await invoke('pause_timeline_audio');
    } catch (error) {
      console.error('Error pausing audio:', error);
    }
  }

  async function handleStop() {
    try {
      await invoke('stop_timeline_audio');
    } catch (error) {
      console.error('Error stopping audio:', error);
    }
  }

  async function handleSkipToStart() {
    try {
      await invoke('set_timeline_play_position', { position: 0.0 });
    } catch (error) {
      console.error('Error skipping to start:', error);
    }
  }

  async function handleSkipToEnd() {
    try {
      await invoke('set_timeline_play_position', { position: 1.0 });
    } catch (error) {
      console.error('Error skipping to end:', error);
    }
  }

  // Loop and record are UI-only for now
  let isLoopEnabled = false;
  function toggleLoop() {
    isLoopEnabled = !isLoopEnabled;
  }

  function handleRecord() {
    // Record functionality not implemented yet
    console.log('Record functionality not implemented yet');
  }
</script>

<!-- Audacity-style Transport Controls -->
<div class="transport-controls d-flex align-items-center gap-2 py-2 px-2">
  <!-- Skip to Start -->
  <button class="btn btn-transport" title="Skip to Start" on:click={handleSkipToStart}>
    <i class="fa-solid fa-backward-step"></i>
  </button>

  <!-- Play -->
  <button
    class="btn btn-transport btn-play"
    class:active={$appState.playingCombined}
    title="Play"
    on:click={handlePlay}
  >
    <i class="fa-solid fa-play"></i>
  </button>

  <!-- Pause -->
  <button
    class="btn btn-transport btn-pause"
    class:active={!$appState.playingCombined && playHeadPosition > 0}
    title="Pause"
    on:click={handlePause}
  >
    <i class="fa-solid fa-pause"></i>
  </button>

  <!-- Stop -->
  <button class="btn btn-transport btn-stop" title="Stop" on:click={handleStop}>
    <i class="fa-solid fa-stop"></i>
  </button>

  <!-- Record -->
  <button class="btn btn-transport btn-record" title="Record" on:click={handleRecord} disabled>
    <i class="fa-solid fa-circle record-icon"></i>
  </button>

  <!-- Skip to End -->
  <button class="btn btn-transport" title="Skip to End" on:click={handleSkipToEnd}>
    <i class="fa-solid fa-forward-step"></i>
  </button>

  <!-- Loop Toggle -->
  <button
    class="btn btn-transport btn-loop"
    class:active={isLoopEnabled}
    title="Loop"
    on:click={toggleLoop}
  >
    <i class="fa-solid fa-repeat"></i>
  </button>
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
</style>
