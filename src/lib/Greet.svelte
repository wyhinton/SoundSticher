<script lang="ts">
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

  import { appState } from './state/state.svelte';
  import Section from './InputDisplay/FileTable.svelte';
  import Plotted from './PlaybackDisplay/Timeline.svelte';
  import Sources from './InputDisplay/Sources.svelte';
  import PlottedInfo from './PlaybackDisplay/PlottedInfo.svelte';
  import type { Event, UnlistenFn } from '@tauri-apps/api/event';
  import Toolbar from './Toolbar.svelte';
  import { onDestroy, onMount } from 'svelte';
  import { invokeWithPerf, updateInputs } from './state/performance';
  import Export from './Export.svelte';
  import Footer from './Footer.svelte';
  import { exportState } from './state/export';
  import { get } from 'svelte/store';
  import { initializeStateSynchronization } from './state/stateSynchronization';
  import ContextMenuWrapper from './components/ContextMenu/ContextMenuWrapper.svelte';
  import MainDebugToolbar from './components/MainDebugToolbar.svelte';

  WebviewWindow.getCurrent()
    .once<null>('initialized', event => {})
    .then(v => {
      console.log(v);
    });

  let filedropEvent: Event<any>;
  let unlisten: UnlistenFn;
  let contextMenuWrapper: ContextMenuWrapper;
  let timelineComponent: any;
  async function onDrop(event) {
    filedropEvent = event;
    if (!filedropEvent) return;
    console.log('ondrop', filedropEvent);
    unlisten();
  }

  const handleSpaceBar = (ev: KeyboardEvent) => {
    if (ev.code === 'Space') {
      ev.preventDefault(); // optional, if you want to prevent default scrolling
      console.log('Spacebar pressed');

      appState.update(s => {
        s.playingCombined = !s.playingCombined;
        if (s.playingCombined) {
          invokeWithPerf('play_timeline_audio');
        } else {
          invokeWithPerf('pause_timeline_audio');
        }
        return s;
      });
    }
  };

  onMount(() => {
    // Initialize state synchronization
    initializeStateSynchronization();

    window.addEventListener('keyup', handleSpaceBar);
    exportState.update(s => {
      s.message = undefined;
      s.progress = undefined;
      s.error = undefined;
      return s;
    });
    updateInputs(get(appState).sections);
  });

  // Sync timeline selection with context menu
  function handleTimelineSelectionChange(event: CustomEvent<Set<number>>) {
    contextMenuWrapper?.updateTimelineSelection(event.detail);
  }

  onDestroy(() => {
    window.removeEventListener('keyup', handleSpaceBar);
  });
</script>

<!-- <Toolbar></Toolbar> -->

<div class="main-content d-flex flex-column">
  {#if import.meta.env.DEV}
    <MainDebugToolbar />
  {/if}

  <div class="content-area flex-grow-1">
    <div class="px-0 d-flex">
      <Sources></Sources>
      <!-- <div class="text-center pixel-font py-2"><b>$</b></div> -->
      <Section sections={$appState.sections}></Section>
    </div>
    <!-- <Waveform></Waveform> -->
    <PlottedInfo></PlottedInfo>
    <Plotted bind:this={timelineComponent} on:selectionChange={handleTimelineSelectionChange}
    ></Plotted>
    <Export></Export>
  </div>

  <!-- Debug Toolbar - Development Only -->

  <Footer></Footer>
</div>

<!-- Context Menu System -->
<ContextMenuWrapper bind:this={contextMenuWrapper} />

<style>
  .main-content {
    height: 100vh;
    overflow: hidden;
  }

  .content-area {
    overflow-y: auto;
    overflow-x: hidden;
  }
</style>
