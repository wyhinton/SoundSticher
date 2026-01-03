<script lang="ts">
  import { appState } from '../state/state.svelte';

  export let isOpen = false;
  export let onClose: () => void;
  export let maxHeight = '300px';
  export let minWidth = '200px';

  // Get menu z-index from theme
  $: menuZIndex = $appState.uiSettings?.theme?.zIndexes?.menu || 1000;

  function handleClickOutside(event: MouseEvent) {
    if (isOpen && event.target instanceof Element) {
      const dropdown = event.target.closest('.dropdown-container');
      if (!dropdown) {
        onClose();
      }
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'Escape' && isOpen) {
      onClose();
    }
  }
</script>

<svelte:window on:click={handleClickOutside} on:keydown={handleKeyDown} />

<div class="dropdown-container">
  {#if isOpen}
    <div
      class="dropdown-menu"
      style="max-height: {maxHeight}; min-width: {minWidth}; z-index: {menuZIndex};"
      role="menu"
      aria-label="Dropdown menu"
      tabindex="-1"
    >
      <slot />
    </div>
  {/if}
</div>

<style>
  .dropdown-container {
    position: relative;
  }

  .dropdown-menu {
    position: absolute;
    top: 100%;
    left: 0;
    background: #2a2a2a;
    border: 1px solid #444;
    border-radius: 6px;
    padding: 8px;
    margin-top: 4px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    overflow-y: auto;
  }

  .dropdown-menu::-webkit-scrollbar {
    width: 6px;
  }

  .dropdown-menu::-webkit-scrollbar-track {
    background: #1a1a1a;
  }

  .dropdown-menu::-webkit-scrollbar-thumb {
    background: #444;
    border-radius: 3px;
  }

  .dropdown-menu::-webkit-scrollbar-thumb:hover {
    background: #555;
  }
</style>
