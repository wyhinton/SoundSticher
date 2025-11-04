<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { ContextMenuItem, ContextMenuPosition } from './types';

  export let items: ContextMenuItem[] = [];
  export let position: ContextMenuPosition = { x: 0, y: 0 };
  export let visible: boolean = false;

  const dispatch = createEventDispatcher();

  let menuElement: HTMLDivElement;

  $: if (visible && menuElement) {
    adjustPosition();
  }

  function adjustPosition() {
    if (!menuElement) return;

    const rect = menuElement.getBoundingClientRect();
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;

    // Adjust horizontal position if menu would go off-screen
    if (position.x + rect.width > viewportWidth) {
      position.x = viewportWidth - rect.width - 10;
    }

    // Adjust vertical position if menu would go off-screen
    if (position.y + rect.height > viewportHeight) {
      position.y = viewportHeight - rect.height - 10;
    }

    // Ensure menu doesn't go off the left or top edge
    position.x = Math.max(10, position.x);
    position.y = Math.max(10, position.y);
  }

  function handleItemClick(item: ContextMenuItem, event: MouseEvent) {
    event.stopPropagation();

    if (item.disabled) return;

    if (item.action) {
      try {
        item.action();
      } catch (error) {
        console.error('Error executing context menu action:', error);
      }
    }

    dispatch('itemClick', { item, event });
    dispatch('close');
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      dispatch('close');
    }
  }

  function handleClickOutside(event: MouseEvent) {
    if (menuElement && !menuElement.contains(event.target as Node)) {
      dispatch('close');
    }
  }
</script>

<svelte:window
  on:keydown={handleKeyDown}
  on:click={handleClickOutside}
  on:resize={() => dispatch('close')}
/>

{#if visible}
  <div
    bind:this={menuElement}
    class="context-menu"
    style="left: {position.x}px; top: {position.y}px;"
    role="menu"
    tabindex="-1"
  >
    {#each items as item}
      {#if item.type === 'separator'}
        <div class="menu-separator" role="separator"></div>
      {:else}
        <div
          class="menu-item"
          class:disabled={item.disabled}
          class:danger={item.variant === 'danger'}
          role="menuitem"
          tabindex="0"
          on:click={e => handleItemClick(item, e)}
          on:keydown={e => e.key === 'Enter' && handleItemClick(item, e)}
        >
          {#if item.icon}
            <i class="menu-icon {item.icon}"></i>
          {/if}
          <span class="menu-label">{item.label}</span>
          {#if item.shortcut}
            <span class="menu-shortcut">{item.shortcut}</span>
          {/if}
          {#if item.submenu}
            <i class="menu-arrow fas fa-chevron-right"></i>
          {/if}
        </div>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .context-menu {
    position: fixed;
    z-index: 9999;
    background: #2d3748;
    border: 1px solid #4a5568;
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    padding: 4px 0;
    min-width: 180px;
    font-size: 13px;
    color: #e2e8f0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  }

  .menu-item {
    display: flex;
    align-items: center;
    padding: 8px 12px;
    cursor: pointer;
    transition: background-color 0.1s ease;
    gap: 8px;
    user-select: none;
  }

  .menu-item:hover:not(.disabled) {
    background: #4a5568;
  }

  .menu-item:focus:not(.disabled) {
    background: #4a5568;
    outline: none;
  }

  .menu-item.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .menu-item.danger {
    color: #fc8181;
  }

  .menu-item.danger:hover:not(.disabled) {
    background: #742a2a;
    color: #fed7d7;
  }

  .menu-icon {
    width: 16px;
    text-align: center;
    flex-shrink: 0;
    font-size: 12px;
  }

  .menu-label {
    flex: 1;
    white-space: nowrap;
  }

  .menu-shortcut {
    font-size: 11px;
    opacity: 0.7;
    margin-left: auto;
    color: #a0aec0;
  }

  .menu-arrow {
    margin-left: auto;
    font-size: 10px;
    opacity: 0.7;
  }

  .menu-separator {
    height: 1px;
    background: #4a5568;
    margin: 4px 8px;
  }
</style>
