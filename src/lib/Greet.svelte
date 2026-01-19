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
  import { TIMELINE_RESIZE } from './config/timelineConfig';

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

  // Resizable timeline state - stored in appState.uiSettings
  let isDraggingDivider = false;

  // Reactive timeline height from appState
  $: timelineHeight =
    $appState.uiSettings?.timelineHeight || TIMELINE_RESIZE.DEFAULT_HEIGHT_PERCENT;

  // Update appState when timeline height changes
  function setTimelineHeight(height: number) {
    appState.update(s => ({
      ...s,
      uiSettings: {
        ...s.uiSettings,
        timelineHeight: height,
      },
    }));
  }

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
    window.addEventListener('mousemove', handleDividerMouseMove);
    window.addEventListener('mouseup', handleDividerMouseUp);
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

  // Handle divider dragging for resizable timeline
  function handleDividerMouseDown(event: MouseEvent) {
    event.preventDefault();
    isDraggingDivider = true;
    document.body.style.cursor = 'ns-resize';
    document.body.style.userSelect = 'none';
  }

  function handleDividerMouseMove(event: MouseEvent) {
    if (!isDraggingDivider) return;

    const viewportHeight = window.innerHeight;
    const mouseY = event.clientY;

    // Calculate new timeline height as percentage
    const newHeightPercent = ((viewportHeight - mouseY) / viewportHeight) * 100;

    // Constrain using config values
    const constrainedHeight = Math.max(
      TIMELINE_RESIZE.MIN_HEIGHT_PERCENT,
      Math.min(TIMELINE_RESIZE.MAX_HEIGHT_PERCENT, newHeightPercent)
    );
    setTimelineHeight(constrainedHeight);
  }

  function handleDividerMouseUp() {
    if (!isDraggingDivider) return;
    isDraggingDivider = false;
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
  }

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeyPress);
    window.removeEventListener('mousemove', handleDividerMouseMove);
    window.removeEventListener('mouseup', handleDividerMouseUp);
    cleanupWaveformService?.();
  });
</script>

<!-- <Toolbar></Toolbar> -->

<div class="main-content d-flex flex-column">
  {#if import.meta.env.DEV}
    <MainDebugToolbar />
  {/if}
  <div
    style:height="{100 - timelineHeight}vh"
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
    <!-- Resizable divider - invisible, just changes cursor on hover -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="timeline-divider"
      class:dragging={isDraggingDivider}
      on:mousedown={handleDividerMouseDown}
      role="separator"
      aria-orientation="horizontal"
      aria-label="Resize timeline"
      tabindex="0"
    ></div>

    <div style:height="{timelineHeight}vh" class="timeline-container">
      <PlottedInfo></PlottedInfo>
      <Plotted bind:this={timelineComponent} on:selectionChange={handleTimelineSelectionChange}
      ></Plotted>
      <!-- <Export></Export> -->
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

  .timeline-container {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  /* Resizable divider */
  .timeline-divider {
    position: relative;
    height: 6px;
    background: var(--bs-border-color);
    cursor: ns-resize;
    display: flex;
    align-items: center;
    justify-content: center;
    user-select: none;
    transition: background-color 0.15s ease;
    z-index: 100;
  }

  .timeline-divider:hover,
  .timeline-divider:focus {
    background: var(--bs-primary);
    outline: none;
  }

  .timeline-divider.dragging {
    background: var(--bs-primary);
  }

  /* Prevent text selection during drag */
  .timeline-divider.dragging ~ * {
    user-select: none;
  }
</style>
