<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { addSource } from '../state/state.svelte';

  export let selectedRowCount: number = 0;
  export let onSelectAll: (() => void) | undefined = undefined;
  export let onClearSelection: (() => void) | undefined = undefined;
  export let onDeleteSelected: (() => void) | undefined = undefined;

  function handleSelectAll() {
    onSelectAll?.();
  }

  function handleClearSelection() {
    onClearSelection?.();
  }

  function handleDeleteSelected() {
    onDeleteSelected?.();
  }

  async function handleAddSource() {
    try {
      const selected = await open({
        multiple: true,
        directory: true,
        title: 'Select folders to add as source sections',
      });

      if (selected && Array.isArray(selected) && selected.length > 0) {
        // Pass the selected folders to addSource
        await addSource(selected);
      } else if (selected && typeof selected === 'string') {
        // Handle single folder selection
        await addSource([selected]);
      }
    } catch (error) {
      console.error('Error opening folder dialog:', error);
    }
  }
</script>

<div class="toolbar">
  <div class="toolbar-section">
    <button class="toolbar-btn" onclick={handleAddSource} title="Add new source section">
      <i class="fas fa-plus"></i>
      Add Section
    </button>
  </div>

  <div class="toolbar-section">
    <button class="toolbar-btn" onclick={handleSelectAll} title="Select all rows">
      <i class="fas fa-check-square"></i>
      Select All
    </button>

    <!-- {#if selectedRowCount > 0}
      <button class="toolbar-btn" onclick={handleClearSelection} title="Clear selection">
        <i class="fas fa-times"></i>
        Clear Selection
      </button>

      <button
        class="toolbar-btn danger"
        onclick={handleDeleteSelected}
        title="Delete selected rows"
      >
        <i class="fas fa-trash"></i>
        Delete ({selectedRowCount})
      </button>
    {/if} -->
  </div>

  <!-- {#if selectedRowCount > 0}
    <div class="selection-info">
      {selectedRowCount} row{selectedRowCount === 1 ? '' : 's'} selected
    </div>
  {/if} -->
</div>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background: linear-gradient(to bottom, #2c3e50, #34495e);
    border: 1px solid #1a252f;
    border-radius: 4px;
    box-shadow: inset 0 1px 2px rgba(255, 255, 255, 0.1);
    gap: 12px;
    flex-wrap: wrap;
  }

  .toolbar-section {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .toolbar-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    height: 32px;
    border: 1px solid #4a5568;
    background: linear-gradient(to bottom, #4a5568, #2d3748);
    color: #e2e8f0;
    border-radius: 3px;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.1s ease;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
  }

  .toolbar-btn:hover {
    background: linear-gradient(to bottom, #5a6578, #3d4852);
    border-color: #718096;
    transform: translateY(-1px);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.4);
    color: #f7fafc;
  }

  .toolbar-btn:active {
    background: linear-gradient(to bottom, #2d3748, #1a202c);
    transform: translateY(0);
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.4);
  }

  .toolbar-btn.danger {
    background: linear-gradient(to bottom, #e53e3e, #c53030);
    border-color: #9b2c2c;
    color: white;
  }

  .toolbar-btn.danger:hover {
    background: linear-gradient(to bottom, #f56565, #e53e3e);
    border-color: #c53030;
    color: white;
  }

  .toolbar-btn.danger:active {
    background: linear-gradient(to bottom, #c53030, #9b2c2c);
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.4);
  }

  .toolbar-btn i {
    font-size: 11px;
  }

  .selection-info {
    font-size: 12px;
    color: #a0aec0;
    font-weight: 500;
    font-style: italic;
  }

  @media (max-width: 600px) {
    .toolbar {
      flex-direction: column;
      align-items: stretch;
    }

    .toolbar-section {
      justify-content: center;
    }
  }
</style>
