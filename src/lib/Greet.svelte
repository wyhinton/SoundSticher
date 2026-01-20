<script lang="ts">
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

  import { appState } from './state/state.svelte';
  import FileTable from './InputDisplay/FileTable.svelte';
  import Plotted from './PlaybackDisplay/Timeline.svelte';
  import PlottedInfo from './PlaybackDisplay/PlottedInfo.svelte';
  import { onDestroy, onMount } from 'svelte';
  import Footer from './StatusFooter.svelte';
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
  import { Pane, Splitpanes } from 'svelte-splitpanes';

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

<div class="main-content">
  <MainDebugToolbar />
  <Splitpanes theme="modern-theme" horizontal style="height: 100vh">
    <!-- Debug Toolbar - Development Only (fixed size when visible) -->
    {#if import.meta.env.DEV}
      <Pane size={5} minSize={5} maxSize={5}></Pane>
    {/if}

    <!-- Main content area (resizable) -->
    <Pane>
      <Splitpanes theme="modern-theme" horizontal>
        <!-- Top content area with left panel and operations/files -->
        <Pane minSize={20}>
          <Splitpanes theme="modern-theme">
            <!-- Left Panel -->
            <Pane size={20} minSize={15} maxSize={30}>
              <MainLeftPanel />
            </Pane>

            <!-- Right side: Operations and File Table -->
            <Pane>
              <Splitpanes theme="modern-theme" horizontal>
                <!-- Operations Flow Panel -->
                <Pane size={30} minSize={15} maxSize={50}>
                  <OperationsFlowPanel />
                </Pane>

                <!-- File Table -->
                <Pane>
                  <FileTable />
                </Pane>
              </Splitpanes>
            </Pane>
          </Splitpanes>
        </Pane>

        <!-- Timeline area (resizable) -->
        <Pane
          size={timelineHeight}
          minSize={TIMELINE_RESIZE.MIN_HEIGHT_PERCENT}
          maxSize={TIMELINE_RESIZE.MAX_HEIGHT_PERCENT}
        >
          <div class="timeline-container">
            <PlottedInfo />
            <Plotted
              bind:this={timelineComponent}
              on:selectionChange={handleTimelineSelectionChange}
            />
            <!-- <Export></Export> -->
          </div>
        </Pane>
      </Splitpanes>
    </Pane>

    <!-- Footer (fixed size) -->
    <Pane size={3} minSize={3} maxSize={3}>
      <Footer />
    </Pane>
  </Splitpanes>
</div>

<!-- Context Menu System -->
<ContextMenuWrapper bind:this={contextMenuWrapper} />

<style>
  .main-content {
    height: 100vh;
    overflow: hidden;
  }

  .timeline-container {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    height: 100%;
    width: 100%;
  }

  /* Modern theme for splitpanes */
  :global(.splitpanes.modern-theme) {
    :global(.splitpanes__pane) {
      background-color: transparent;
    }

    :global(.splitpanes__splitter) {
      background-color: var(--bs-border-color);
      position: relative;

      &:before {
        content: '';
        position: absolute;
        left: 0;
        top: 0;
        transition:
          opacity 0.4s,
          background-color 0.15s ease;
        background-color: var(--bs-primary);
        opacity: 0;
        z-index: 1;
      }

      &:hover:before {
        opacity: 1;
      }

      &:global(.splitpanes__splitter__active) {
        z-index: 2;

        &:before {
          opacity: 1;
        }
      }
    }
  }

  /* Vertical splitters */
  :global(.modern-theme.splitpanes--vertical > .splitpanes__splitter:before) {
    left: -3px;
    right: -3px;
    height: 100%;
    cursor: col-resize;
  }

  /* Horizontal splitters */
  :global(.modern-theme.splitpanes--horizontal > .splitpanes__splitter:before) {
    top: -3px;
    bottom: -3px;
    width: 100%;
    cursor: row-resize;
  }
</style>
