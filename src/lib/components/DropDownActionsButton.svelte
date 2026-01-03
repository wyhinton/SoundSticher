<script lang="ts">
  import { openDropdown, openDropdownExclusive, closeDropdown } from '../state/dropdown.svelte';

  // Props for customization
  export let dropdownId: string;
  export let buttonTitle: string = 'More actions';
  export let buttonAriaLabel: string = 'More actions';
  export let actions: Array<{
    id: string;
    label: string;
    icon: string;
    iconClasses?: string;
    disabled?: boolean;
    variant?: 'default' | 'danger';
    onClick: (event: MouseEvent) => void;
  }> = [];

  // Check if this dropdown should be shown
  $: showDropdown = $openDropdown === dropdownId;

  function toggleDropdown(event: MouseEvent) {
    event.stopPropagation();
    if (showDropdown) {
      closeDropdown(dropdownId);
    } else {
      openDropdownExclusive(dropdownId);
    }
  }

  function closeThisDropdown() {
    closeDropdown(dropdownId);
  }

  function handleActionClick(action: (typeof actions)[0], event: MouseEvent) {
    event.stopPropagation();
    if (!action.disabled) {
      action.onClick(event);
      closeThisDropdown();
    }
  }

  // Close dropdown when clicking outside
  function handleWindowClick(event: Event) {
    if (!showDropdown) return;
    const target = event.target as HTMLElement;
    if (!target.closest('.dropdown-container')) {
      closeThisDropdown();
    }
  }
</script>

<svelte:window onclick={handleWindowClick} />

<div class="dropdown-container">
  <button
    class="action-button actions-dropdown-icon"
    onclick={toggleDropdown}
    title={buttonTitle}
    aria-label={buttonAriaLabel}
  >
    <i class="fas fa-ellipsis-v"></i>
  </button>
  {#if showDropdown}
    <div class="my-dropdown">
      {#each actions as action}
        <button
          class="dropdown-item"
          class:delete-item={action.variant === 'danger'}
          class:disabled={action.disabled}
          onclick={event => handleActionClick(action, event)}
          disabled={action.disabled}
        >
          <i class="fas {action.icon} me-2 {action.iconClasses || ''}"></i>
          {action.label}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .action-button {
    background: none;
    border: none;
    color: #9d9d9d;
    padding: 4px 8px;
    border-radius: 3px;
    cursor: pointer;
    font-size: 12px;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: -1;
  }

  .action-button:hover {
    background-color: rgba(255, 255, 255, 0.1);
    color: #e74c3c;
    transform: scale(1.1);
  }

  .action-button:active {
    transform: scale(0.95);
  }

  .dropdown-container {
    position: relative;
  }

  .my-dropdown {
    position: absolute;
    top: 100%;
    right: 0;
    background: #2a2a2a;
    border: 1px solid #555;
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    z-index: 1000;
    min-width: 150px;
    padding: 4px 0;
    margin-top: 2px;
  }

  .dropdown-item {
    display: flex;
    align-items: center;
    width: 100%;
    padding: 8px 12px;
    background: none;
    border: none;
    color: #ccc;
    font-size: 12px;
    cursor: pointer;
    transition: background-color 0.2s ease;
    text-align: left;
  }

  .dropdown-item:hover:not(.disabled) {
    background-color: rgba(255, 255, 255, 0.1);
    color: #fff;
  }

  .dropdown-item.delete-item:hover:not(.disabled) {
    background-color: rgba(231, 76, 60, 0.2);
    color: #e74c3c;
  }

  .dropdown-item.disabled {
    cursor: default;
    opacity: 0.6;
  }

  .dropdown-item.disabled:hover {
    background-color: rgba(255, 255, 255, 0.05);
    color: #ccc;
  }

  .dropdown-item i {
    width: 16px;
    text-align: center;
  }

  .dropdown-item .text-danger {
    color: #e74c3c !important;
  }
</style>
