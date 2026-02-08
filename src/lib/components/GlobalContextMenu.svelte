<script lang="ts">
  import { onMount } from 'svelte';
  import ContextMenu from '../components/ContextMenu/ContextMenu.svelte';
  import { contextMenuManager } from '../utils/contextMenuManager';

  // State management using Svelte 5 runes
  let isVisible = $state(false);
  let config = $state(null);

  // Update reactive state when manager state changes
  $effect(() => {
    isVisible = contextMenuManager.isVisible;
    config = contextMenuManager.config;
  });

  // Close context menu on click outside
  function handleGlobalClick(event: MouseEvent) {
    if (isVisible) {
      contextMenuManager.hideContextMenu();
    }
  }

  // Close context menu on escape key
  function handleGlobalKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && isVisible) {
      contextMenuManager.hideContextMenu();
    }
  }

  onMount(() => {
    // Add global event listeners
    document.addEventListener('click', handleGlobalClick);
    document.addEventListener('keydown', handleGlobalKeydown);

    return () => {
      // Cleanup
      document.removeEventListener('click', handleGlobalClick);
      document.removeEventListener('keydown', handleGlobalKeydown);
    };
  });

  // Handle context menu item selection
  function handleItemSelect(event: CustomEvent) {
    contextMenuManager.hideContextMenu();
  }
</script>

<!-- Global context menu -->
{#if isVisible && config}
  <ContextMenu
    items={config.items}
    position={config.position}
    visible={isVisible}
    on:item-select={handleItemSelect}
    on:close={() => contextMenuManager.hideContextMenu()}
  />
{/if}
