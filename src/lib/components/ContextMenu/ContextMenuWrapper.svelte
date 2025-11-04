<script lang="ts">
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import ContextMenu from './ContextMenu.svelte';
  import { contextMenuState, contextMenuManager } from './contextMenuStore';
  import {
    timelineSegmentProvider,
    fileTableRowProvider,
    sourcesTableProvider,
    exportPanelProvider,
    generalProvider,
  } from './providers';
  import { appState } from '../../state/state.svelte';

  // Context for passing to providers
  let selectedTimelineSegments: Set<number> = new Set();
  let selectedSourceRows: Set<number> = new Set();

  // Register providers in order of priority (first match wins)
  onMount(() => {
    contextMenuManager.registerProvider('timeline-segment', timelineSegmentProvider);
    contextMenuManager.registerProvider('file-table-row', fileTableRowProvider);
    contextMenuManager.registerProvider('sources-table', sourcesTableProvider);
    contextMenuManager.registerProvider('export-panel', exportPanelProvider);
    contextMenuManager.registerProvider('general', generalProvider); // Fallback

    return () => {
      // Cleanup providers on unmount
      contextMenuManager.unregisterProvider('timeline-segment');
      contextMenuManager.unregisterProvider('file-table-row');
      contextMenuManager.unregisterProvider('sources-table');
      contextMenuManager.unregisterProvider('export-panel');
      contextMenuManager.unregisterProvider('general');
    };
  });

  function handleContextMenu(event: MouseEvent) {
    event.preventDefault();

    const context = {
      target: event.target as HTMLElement,
      event,
      appState: get(appState),
      selectedItems: getSelectedItemsForTarget(event.target as HTMLElement),
    };

    contextMenuManager.show(context);
  }

  function getSelectedItemsForTarget(target: HTMLElement): any[] {
    // Determine what type of element was clicked and return appropriate selection
    if (target.closest('[data-timeline-segment]')) {
      return Array.from(selectedTimelineSegments);
    } else if (target.closest('[data-source-row]')) {
      return Array.from(selectedSourceRows);
    }
    return [];
  }

  function handleClose() {
    contextMenuManager.hide();
  }

  // Export functions for components to update selection state
  export function updateTimelineSelection(selection: Set<number>) {
    selectedTimelineSegments = selection;
  }

  export function updateSourceSelection(selection: Set<number>) {
    selectedSourceRows = selection;
  }
</script>

<svelte:window on:contextmenu={handleContextMenu} />

<ContextMenu
  visible={$contextMenuState.visible}
  items={$contextMenuState.config?.items || []}
  position={$contextMenuState.config?.position || { x: 0, y: 0 }}
  on:close={handleClose}
/>
