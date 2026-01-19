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
  import { initializeStatusPublishers } from './state/status-publishers';
  import ContextMenuWrapper from './components/ContextMenu/ContextMenuWrapper.svelte';
  import MainDebugToolbar from './components/MainDebugToolbar.svelte';
  import OperationsFlowPanel from './InputDisplay/Operations/OperationsFlowPanel.svelte';
  import MainLeftPanel from './InputDisplay/MainLeftPanel.svelte';
  import { opPlaybackService } from './state/opPlaybackService';
  import { undo, redo, canUndo, canRedo } from './state/undo';

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

  const handleKeyPress = (ev: KeyboardEvent) => {
    // Handle spacebar for play/pause
    if (ev.code === 'Space' && !ev.shiftKey && !ev.ctrlKey && !ev.metaKey) {
      // Only handle spacebar if not focused on an input element
      if (ev.target instanceof HTMLInputElement || ev.target instanceof HTMLTextAreaElement) {
        return;
      }

      ev.preventDefault(); // Prevent default scrolling
      // Use the operation playback service
      opPlaybackService.togglePlayPause().catch(err => {
        console.error('Error toggling playback:', err);
      });
      return;
    }

    // Handle undo/redo shortcuts
    if ((ev.ctrlKey || ev.metaKey) && !ev.altKey) {
      if (ev.key === 'z' && !ev.shiftKey) {
        // Ctrl+Z or Cmd+Z for undo
        ev.preventDefault();
        if (canUndo()) {
          undo();
          console.log('🔄 Undo triggered via keyboard shortcut');
        }
        return;
      }

      if (ev.key === 'y' || (ev.key === 'z' && ev.shiftKey)) {
        // Ctrl+Y or Ctrl+Shift+Z or Cmd+Y or Cmd+Shift+Z for redo
        ev.preventDefault();
        if (canRedo()) {
          redo();
          console.log('🔄 Redo triggered via keyboard shortcut');
        }
        return;
      }
    }
  };

  onMount(() => {
    // Initialize state synchronization
    initializeStateSynchronization();

    // Initialize subscriptions to avoid circular dependency issues
    initializeGroupsSubscription();
    initializeOperationsSubscription();

    // Initialize automatic status publishers (buffering, etc.)
    initializeStatusPublishers();

    // document.addEventListener('dragover', event => {
    //   event.preventDefault();
    // });
    // Initialize waveform service (handles loading waveforms when operation changes)
    cleanupWaveformService = initWaveformService();

    window.addEventListener('keydown', handleKeyPress);
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
    window.removeEventListener('keydown', handleKeyPress);
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
          <!-- <Sources></Sources> -->
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
