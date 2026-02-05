<script lang="ts">
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

  import { appState } from './state/state.svelte';
  import InputsOutputsTable from './InputDisplay/InputsOutputsTable.svelte';
  import Timeline from './PlaybackDisplay/Timeline.svelte';
  import PlottedInfo from './PlaybackDisplay/PlottedInfo.svelte';
  import { onDestroy, onMount } from 'svelte';
  import Footer from './StatusFooter.svelte';
  import { initializeFrontend } from './state/initializeFrontend';
  import ContextMenuWrapper from './components/ContextMenu/ContextMenuWrapper.svelte';
  import MainDebugToolbar from './components/debug/MainDebugToolbar.svelte';
  import OperationsFlowPanel from './InputDisplay/Operations/OperationsFlowPanel.svelte';
  import MainLeftPanel from './InputDisplay/MainLeftPanel.svelte';
  import { TIMELINE_RESIZE } from './config/timelineConfig';
  import { type IPaneSizingEvent, Pane, Splitpanes } from 'svelte-splitpanes';
  import { operationTimelines } from './state/timeline/timelines';

  WebviewWindow.getCurrent()
    .once<null>('initialized', event => {})
    .then(v => {
      console.log(v);
    });

  // let filedropEvent: Event<any>;
  // let unlisten: UnlistenFn;
  let contextMenuWrapper: ContextMenuWrapper;
  let timelineComponent: any;
  let operationsPanelHeight: number = 0;
  let cleanupFrontend: (() => void) | null = null;

  // Reactive timeline height from appState
  $: timelineHeight = TIMELINE_RESIZE.DEFAULT_HEIGHT_PERCENT;

  // Update appState when timeline height changes

  // async function onDrop(event) {
  //   filedropEvent = event;
  //   if (!filedropEvent) return;
  //   console.log('ondrop', filedropEvent);
  //   unlisten();
  // }

  onMount(() => {
    // Initialize all frontend systems (state, subscriptions, services, keyboard shortcuts)
    cleanupFrontend = initializeFrontend();
  });

  // Sync timeline selection with context menu
  function handleTimelineSelectionChange(event: CustomEvent<Set<number>>) {
    contextMenuWrapper?.updateTimelineSelection(event.detail);
  }

  //TODO: ID BASED GET PANEL
  // Handle resize of operations panel
  function handleOperationsPanelResize(event: CustomEvent<number>) {
    if (event.detail) {
      console.log(`%cHERE LINE :68 %c`, 'color: yellow; font-weight: bold', '');

      operationsPanelHeight = (event.detail as any as IPaneSizingEvent[])[0].size;
    }
  }

  onDestroy(() => {
    cleanupFrontend?.();
  });
</script>

<!-- <Toolbar></Toolbar> -->

<div class="main-content">
  <MainDebugToolbar />
  <Splitpanes theme="modern-theme" horizontal style="height: 100vh">
    <!-- Debug Toolbar - Development Only (fixed size when visible) -->
    <!-- {#if import.meta.env.DEV}
      <Pane size={5} minSize={5} maxSize={5}></Pane>
    {/if} -->

    <!-- Main content area (resizable) -->
    <Pane>
      <Splitpanes theme="modern-theme" horizontal on:resize={e => handleOperationsPanelResize(e)}>
        <!-- Top content area with left panel and operations/files -->
        <Pane minSize={20}>
          <Splitpanes theme="modern-theme">
            <!-- Left Panel -->
            <Pane size={20} minSize={15} maxSize={30}>
              <MainLeftPanel />
            </Pane>

            <!-- Right side: Operations and File Table -->
            <Pane>
              <Splitpanes
                theme="modern-theme"
                horizontal
                on:resize={e => handleOperationsPanelResize(e)}
              >
                <!-- Operations Flow Panel -->
                <Pane size={50} minSize={15} maxSize={50} class="operations-flow-pane">
                  <OperationsFlowPanel panelHeight={operationsPanelHeight} />
                </Pane>

                <!-- File Table -->
                <Pane size={50}>
                  <InputsOutputsTable operationId={$appState.uiSettings?.selectedOperationId} />
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
            <Splitpanes theme="modern-theme" horizontal={true}>
              {#each $operationTimelines as timeline (timeline.id)}
                <Pane>
                  <PlottedInfo {timeline} />
                  <Timeline {timeline} on:selectionChange={handleTimelineSelectionChange} />
                </Pane>
              {:else}
                <div class="no-timelines">
                  <p>No operation timelines visible</p>
                  <small>Use the eye button on operations to show their timelines</small>
                </div>
              {/each}
            </Splitpanes>
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
