<script lang="ts">
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

  import { appState, currentOperationSources } from './state/state.svelte';
  import FileTable from './InputDisplay/FileTable.svelte';
  import Plotted from './PlaybackDisplay/Timeline.svelte';
  import Sources from './InputDisplay/Sources.svelte';
  import PlottedInfo from './PlaybackDisplay/PlottedInfo.svelte';
  import { onDestroy, onMount } from 'svelte';
  import { invokeWithPerf, updateInputs } from './state/performance';
  import Export from './Export.svelte';
  import Footer from './StatusFooter.svelte';
  import { exportState } from './state/export';
  import { get } from 'svelte/store';
  import { initializeStateSynchronization } from './state/stateSynchronization';
  import { initWaveformService } from './state/waveformCache';
  import { initializeGroupsSubscription } from './state/groups';
  import { initializeOperationsSubscription } from './state/operation';
  import ContextMenuWrapper from './components/ContextMenu/ContextMenuWrapper.svelte';
  import MainDebugToolbar from './components/MainDebugToolbar.svelte';
  import OperationsFlowPanel from './InputDisplay/Operations/OperationsFlowPanel.svelte';
  import MainLeftPanel from './InputDisplay/MainLeftPanel.svelte';
  import { opPlaybackService } from './state/opPlaybackService';
  import TestDrag from './components/TestDrag.svelte';
  import DragTest2 from './components/DragTest2.svelte';

  WebviewWindow.getCurrent()
    .once<null>('initialized', event => {})
    .then(v => {
      console.log(v);
    });

  // let filedropEvent: Event<any>;
  // let unlisten: UnlistenFn;
  let contextMenuWrapper: ContextMenuWrapper;
  let timelineComponent: any;
  let cleanupWaveformService: (() => void) | null = null;

  // async function onDrop(event) {
  //   filedropEvent = event;
  //   if (!filedropEvent) return;
  //   console.log('ondrop', filedropEvent);
  //   unlisten();
  // }

  const handleSpaceBar = (ev: KeyboardEvent) => {
    if (ev.code === 'Space') {
      ev.preventDefault(); // optional, if you want to prevent default scrolling
      // Use the operation playback service
      opPlaybackService.togglePlayPause().catch(err => {
        console.error('Error toggling playback:', err);
      });
    }
  };

  onMount(() => {
    // Initialize state synchronization
    initializeStateSynchronization();

    // Initialize subscriptions to avoid circular dependency issues
    initializeGroupsSubscription();
    initializeOperationsSubscription();

    // document.addEventListener('dragover', event => {
    //   event.preventDefault();
    // });
    // Initialize waveform service (handles loading waveforms when operation changes)
    cleanupWaveformService = initWaveformService();

    window.addEventListener('keyup', handleSpaceBar);
    // exportState.update(s => {
    //   s.message = undefined;
    //   s.progress = undefined;
    //   s.error = undefined;
    //   return s;
    // });
    // updateInputs(get(appState).sections); // Legacy code - no longer needed
  });

  // Sync timeline selection with context menu
  function handleTimelineSelectionChange(event: CustomEvent<Set<number>>) {
    contextMenuWrapper?.updateTimelineSelection(event.detail);
  }

  onDestroy(() => {
    window.removeEventListener('keyup', handleSpaceBar);
    cleanupWaveformService?.();
  });
</script>

<!-- <Toolbar></Toolbar> -->

<div class="main-content d-flex flex-column">
  {#if import.meta.env.DEV}
    <MainDebugToolbar />
  {/if}
  <div
    style:height="70vh"
    class="content-area flex-grow-1 d-flex justify-content-between flex-column"
  >
    <div class="px-0 d-flex h-fill-available">
      <MainLeftPanel></MainLeftPanel>
      <div class="d-flex flex-column w-100">
        <div class="d-flex flex-column w-100">
          <OperationsFlowPanel></OperationsFlowPanel>
        </div>
        <div class="d-flex w-100 h-fill-available">
          <Sources></Sources>
          <FileTable></FileTable>
        </div>
      </div>
    </div>
    <div style:height="30vh">
      <PlottedInfo></PlottedInfo>
      <Plotted bind:this={timelineComponent} on:selectionChange={handleTimelineSelectionChange}
      ></Plotted>
      <Export></Export>
    </div>
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
