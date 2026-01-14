<script lang="ts">
  import { draggable } from '$lib/attachments/draggable';
  import { dragStore } from '$lib/state/dragStore';
  import type { OperationInfo } from '$lib/state/operation';
  import { setSelectedOperationName, addOpAsSource } from '$lib/state/state.svelte';
  import ContextMenu from '$lib/components/ContextMenu/ContextMenu.svelte';
  import type { ContextMenuItem, ContextMenuPosition } from '$lib/components/ContextMenu/types';
  import { get } from 'svelte/store';

  export let operationName: string;
  export let isSelected: boolean = false;
  export let opInfo: OperationInfo | undefined;
  export let showDebugInfo: boolean = false;
  export let debugInfo: { x: number; y: number; zoom: number } = { x: 0, y: 0, zoom: 1 };

  // Context menu state
  let contextMenuVisible = false;
  let contextMenuPosition: ContextMenuPosition = { x: 0, y: 0 };
  let contextMenuItems: ContextMenuItem[] = [];

  function handleClick() {
    // Call setSelectedOperationName directly instead of dispatching event
    setSelectedOperationName(operationName);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.ctrlKey && event.shiftKey && event.code === 'Space') {
      event.preventDefault();
      showDebugInfo = !showDebugInfo;

      // Dispatch to parent
      const toggleEvent = new CustomEvent('toggleDebug', {
        detail: { showDebugInfo },
        bubbles: true,
      });
      dispatchEvent(toggleEvent);
    }
  }

  function handleContextMenu(event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();

    contextMenuPosition = {
      x: event.clientX,
      y: event.clientY,
    };

    contextMenuItems = [
      {
        type: 'item',
        label: 'Add as source to current operation',
        icon: 'fas fa-plus-circle',
        action: () => addAsSource(),
      },
      {
        type: 'separator',
      },
      {
        type: 'item',
        label: 'Copy operation name',
        icon: 'fas fa-copy',
        action: () => copyOperationName(),
      },
      {
        type: 'item',
        label: 'View operation details',
        icon: 'fas fa-info-circle',
        action: () => viewOperationDetails(),
      },
      {
        type: 'separator',
      },
      {
        type: 'item',
        label: 'Toggle debug info',
        icon: 'fas fa-bug',
        shortcut: 'Ctrl+Shift+Space',
        action: () => toggleDebugInfo(),
      },
    ];

    contextMenuVisible = true;
  }

  function addAsSource() {
    addOpAsSource(operationName);
  }

  function copyOperationName() {
    navigator.clipboard
      .writeText(operationName)
      .then(() => {
        console.log(`Copied "${operationName}" to clipboard`);
      })
      .catch(err => {
        console.error('Failed to copy operation name:', err);
      });
  }

  function viewOperationDetails() {
    console.log('Operation details:', { operationName, opInfo, debugInfo });

    // Dispatch event to show operation details
    const viewDetailsEvent = new CustomEvent('viewDetails', {
      detail: {
        operationName,
        opInfo,
        debugInfo,
      },
      bubbles: true,
    });
    dispatchEvent(viewDetailsEvent);
  }

  function toggleDebugInfo() {
    showDebugInfo = !showDebugInfo;

    const toggleEvent = new CustomEvent('toggleDebug', {
      detail: { showDebugInfo },
      bubbles: true,
    });
    dispatchEvent(toggleEvent);
  }

  function closeContextMenu() {
    contextMenuVisible = false;
  }

  //     on:dragstart={() => {
  //     dragStore.set({
  //       item: {
  //         type: 'operation',
  //         payload: undefined,
  //         sourceId: operationName,
  //       },
  //       overTargetId: null,
  //     });
  //   }}
  //   on:dragend={() => {
  //     dragStore.set({ item: null, overTargetId: null });
  //   }}
</script>

<div
  class="flow-header"
  class:selected={isSelected}
  tabindex="0"
  on:click={handleClick}
  on:keydown={handleKeydown}
  on:contextmenu={handleContextMenu}
  use:draggable={{
    type: 'sample',
    data: operationName,
    sourceId: 'library',
  }}
  role="button"
  aria-label="Operation flow header - Click to select, Right-click for context menu, Press Ctrl+Shift+Space to toggle debug info"
>
  <span class="operation-icon">{opInfo?.icon || '🔗'}</span>
  <span class="operation-name fira font-size-12px">{operationName}</span>

  {#if showDebugInfo}
    <div class="debug-info">
      <span class="debug-label">Debug:</span>
      <span class="debug-value">x: {debugInfo.x}</span>
      <span class="debug-value">y: {debugInfo.y}</span>
      <span class="debug-value">zoom: {debugInfo.zoom}</span>
    </div>
  {/if}
</div>

<!-- Context Menu -->
<ContextMenu
  bind:visible={contextMenuVisible}
  position={contextMenuPosition}
  items={contextMenuItems}
  on:close={closeContextMenu}
  on:itemClick={closeContextMenu}
/>

<style>
  .flow-header {
    position: absolute;
    top: 4px;
    left: 4px;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(4px);
    padding: 2px 2px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    gap: 2px;
    z-index: 10;
    outline: none;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .flow-header:hover {
    background: rgba(0, 0, 0, 0.8);
    transform: translateY(-1px);
  }

  .flow-header:focus {
    box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.5);
  }

  .flow-header.selected {
    background: rgba(59, 130, 246, 0.3);
    border: 1px solid rgba(59, 130, 246, 0.6);
  }

  .flow-header.selected:hover {
    background: rgba(59, 130, 246, 0.4);
  }

  .debug-info {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-left: 8px;
    padding: 2px 4px;
    background: rgba(59, 130, 246, 0.2);
    border-radius: 4px;
    font-family: 'Fira Code', monospace;
    font-size: 10px;
  }

  .debug-label {
    color: #60a5fa;
    font-weight: bold;
  }

  .debug-value {
    color: #e5e7eb;
    font-weight: normal;
  }

  .operation-icon {
    font-size: 1rem;
  }

  .operation-name {
    color: #ffffff;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.7);
    white-space: nowrap;
  }
</style>
