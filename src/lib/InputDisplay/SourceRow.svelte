<script lang="ts">
  import { toCssRgb } from '../utils/colors';
  import { updatePath, deleteSection, addToFavorites, isFavorite } from '../state/state.svelte';
  import type { Section } from '../state/state.svelte';
  import EditableInput from './EditableInput.svelte';
  import { openDropdown, openDropdownExclusive, closeDropdown } from '../state/dropdown.svelte';

  export let item: Section;
  export let sectionIndex: number;
  export let inputsUnderMouse: number[] = [];
  export let isSelected: boolean = false;
  export let onRowSelect: ((index: number, isShiftSelect?: boolean) => void) | undefined =
    undefined;
  export let onRowToggle: ((index: number) => void) | undefined = undefined;

  // Create unique dropdown ID for this row
  const dropdownId = `source-row-${sectionIndex}`;

  // Check if current item is favorited
  $: isCurrentlyFavorited = isFavorite(item.folderPath);

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

  function handleAddToFavorites(event: MouseEvent) {
    event.stopPropagation();
    addToFavorites(item.folderPath);
    closeThisDropdown();
  }

  function handleDelete(event: MouseEvent) {
    event.stopPropagation();
    deleteSection(sectionIndex);
    closeThisDropdown();
  }

  // Close dropdown when clicking outside
  function handleWindowClick(event: Event) {
    if (!showDropdown) return;
    const target = event.target as HTMLElement;
    if (!target.closest('.dropdown-container')) {
      closeThisDropdown();
    }
  }

  function handleRowClick(event: MouseEvent) {
    // Don't select when clicking on input, button, or dropdown
    if (
      event.target instanceof HTMLInputElement ||
      event.target instanceof HTMLButtonElement ||
      (event.target as HTMLElement).closest('.dropdown-container')
    ) {
      return;
    }

    if (event.ctrlKey || event.metaKey) {
      // Multi-select with Ctrl/Cmd
      onRowToggle?.(sectionIndex);
    } else if (event.shiftKey) {
      // Range select with Shift
      onRowSelect?.(sectionIndex, true);
    } else {
      // Single select
      onRowSelect?.(sectionIndex, false);
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      if (event.ctrlKey || event.metaKey) {
        onRowToggle?.(sectionIndex);
      } else if (event.shiftKey) {
        onRowSelect?.(sectionIndex, true);
      } else {
        onRowSelect?.(sectionIndex, false);
      }
    }
  }
</script>

<svelte:window onclick={handleWindowClick} />

<tr
  class="source-row"
  class:selected={isSelected}
  style:height="28px"
  onclick={handleRowClick}
  onkeydown={handleKeyDown}
  role="button"
  tabindex="0"
  aria-label="Source row {sectionIndex + 1}"
>
  <td>
    <div
      class:under-drag={inputsUnderMouse.includes(sectionIndex)}
      class="d-flex justify-content-start align-items-center"
    >
      <i class="fas fa-folder my-0 mx-2"></i>
      <EditableInput
        bind:fullPath={item.folderPath}
        on:change={e => updatePath(sectionIndex, e.detail)}
      />
    </div>
    <div class="d-flex"></div>
  </td>
  <td>
    <div class="stat text-center">
      <div>{item.files.length}</div>
    </div>
  </td>
  <td>
    <div class="d-flex justify-content-center">
      <div class="dropdown-container">
        <button
          class="action-button actions-dropdown-icon"
          onclick={toggleDropdown}
          title="More actions"
          aria-label="More actions"
        >
          <i class="fas fa-ellipsis-v"></i>
        </button>
        {#if showDropdown}
          <div class="my-dropdown">
            <button class="dropdown-item" onclick={handleAddToFavorites}>
              {#if isCurrentlyFavorited}
                <i class="fas fa-heart me-2 text-danger"></i>
                Already Favorited
              {:else}
                <i class="fas fa-heart me-2"></i>
                Add to Favorites
              {/if}
            </button>
            <button class="dropdown-item delete-item" onclick={handleDelete}>
              <i class="fas fa-trash me-2"></i>
              Delete
            </button>
          </div>
        {/if}
      </div>
    </div>
  </td>
  <!-- {#if showDropdown}
    <div class="my-dropdown">
      <button class="dropdown-item" onclick={handleAddToFavorites}>
        <i class="fas fa-heart me-2"></i>
        Add to Favorites
      </button>
      <button class="dropdown-item delete-item" onclick={handleDelete}>
        <i class="fas fa-trash me-2"></i>
        Delete
      </button>
    </div>
  {/if} -->
</tr>

<style>
  .source-row {
    border-bottom: 1px solid #535353;
    cursor: pointer;
    transition: background-color 0.2s ease;
  }

  .source-row:hover {
    background-color: rgba(255, 255, 255, 0.05);
  }

  .source-row.selected {
    background-color: rgba(59, 130, 246, 0.15) !important;
    border-color: #3b82f6;
  }
  .source-row.selected > td {
    background-color: rgba(59, 130, 246, 0.15) !important;
  }

  .source-row.selected:hover {
    background-color: rgba(59, 130, 246, 0.25) !important;
  }

  .under-drag {
    border: 2px solid green;
  }

  .folder-input {
    width: 200px;
    border-radius: 2px;
    border: 0px;
  }

  .folder-input,
  td {
    font-size: 13px;
  }

  td {
    background-color: var(--bs-primary-bg-subtle) !important;
    padding: 0px !important;
    font-size: 12px;
    vertical-align: middle;
  }

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

  .dropdown-item:hover {
    background-color: rgba(255, 255, 255, 0.1);
    color: #fff;
  }

  .dropdown-item.delete-item:hover {
    background-color: rgba(231, 76, 60, 0.2);
    color: #e74c3c;
  }

  .dropdown-item i {
    width: 16px;
    text-align: center;
  }

  .dropdown-item .text-danger {
    color: #e74c3c !important;
  }

  .dropdown-item:has(.text-danger) {
    cursor: default;
  }

  .dropdown-item:has(.text-danger):hover {
    background-color: rgba(255, 255, 255, 0.05);
    color: #ccc;
  }
</style>
