<script lang="ts">
  import { toCssRgb } from '../utils/colors';
  import { addToFavorites, isFavorite } from '../state/state.svelte';
  import type { Section } from '../state/state.svelte';
  import EditableInput from './EditableInput.svelte';
  import DropDownActionsButton from '../components/DropDownActionsButton.svelte';

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

  // Define dropdown actions
  $: dropdownActions = [
    {
      id: 'favorites',
      label: isCurrentlyFavorited ? 'Already Favorited' : 'Add to Favorites',
      icon: 'fa-heart',
      iconClasses: isCurrentlyFavorited ? 'text-danger' : '',
      disabled: isCurrentlyFavorited,
      onClick: handleAddToFavorites,
    },
    {
      id: 'delete',
      label: 'Delete',
      icon: 'fa-trash',
      variant: 'danger' as const,
      onClick: handleDelete,
    },
  ];

  function handleAddToFavorites(event: MouseEvent) {
    event.stopPropagation();
    addToFavorites(item.folderPath);
  }

  function handleDelete(event: MouseEvent) {
    event.stopPropagation();
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
      <DropDownActionsButton
        {dropdownId}
        buttonTitle="More actions"
        buttonAriaLabel="More actions"
        actions={dropdownActions}
      />
    </div>
  </td>
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

  td {
    background-color: var(--bs-primary-bg-subtle) !important;
    padding: 0px !important;
    font-size: 12px;
    vertical-align: middle;
  }
</style>
