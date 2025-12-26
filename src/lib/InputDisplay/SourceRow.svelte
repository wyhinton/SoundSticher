<script lang="ts">
  import { toCssRgb } from '../utils/colors';
  import { updatePath, deleteSection } from '../state/state.svelte';
  import type { Section } from '../state/state.svelte';
  import EditableInput from './EditableInput.svelte';

  export let item: Section;
  export let sectionIndex: number;
  export let inputsUnderMouse: number[] = [];
  export let isSelected: boolean = false;
  export let onRowSelect: ((index: number, isShiftSelect?: boolean) => void) | undefined =
    undefined;
  export let onRowToggle: ((index: number) => void) | undefined = undefined;

  function handleRowClick(event: MouseEvent) {
    // Don't select when clicking on input or button
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLButtonElement) {
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
      <button
        class="action-button"
        onclick={() => deleteSection(sectionIndex)}
        title="Delete section"
        aria-label="Delete section"
      >
        <i class="fas fa-ellipsis-v"></i>
      </button>
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
</style>
