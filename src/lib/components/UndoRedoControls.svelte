<!-- UndoRedoControls.svelte -->
<script lang="ts">
  import { undo, redo, canUndo, canRedo, getUndoRedoLabels } from '$lib/state/undo';

  // Reactive updates
  $: undoAvailable = canUndo();
  $: redoAvailable = canRedo();
  $: labels = getUndoRedoLabels();

  function handleUndo() {
    if (undoAvailable) {
      undo();
    }
  }

  function handleRedo() {
    if (redoAvailable) {
      redo();
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.ctrlKey || event.metaKey) {
      if (event.key === 'z' && !event.shiftKey) {
        event.preventDefault();
        handleUndo();
      } else if (event.key === 'y' || (event.key === 'z' && event.shiftKey)) {
        event.preventDefault();
        handleRedo();
      }
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

<div class="undo-redo-controls">
  <button
    class="undo-btn"
    disabled={!undoAvailable}
    onclick={handleUndo}
    title={labels.undo ? `Undo: ${labels.undo}` : 'Nothing to undo'}
  >
    ↶ Undo
  </button>

  <button
    class="redo-btn"
    disabled={!redoAvailable}
    onclick={handleRedo}
    title={labels.redo ? `Redo: ${labels.redo}` : 'Nothing to redo'}
  >
    ↷ Redo
  </button>

  {#if labels.undo || labels.redo}
    <div class="status-text">
      {#if labels.undo}
        <span class="undo-status">Undo: {labels.undo}</span>
      {/if}
      {#if labels.redo}
        <span class="redo-status">Redo: {labels.redo}</span>
      {/if}
    </div>
  {/if}
</div>

<style>
  .undo-redo-controls {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    backdrop-filter: blur(8px);
  }

  button {
    padding: 6px 12px;
    border: 1px solid rgba(255, 255, 255, 0.2);
    background: rgba(255, 255, 255, 0.1);
    color: white;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    transition: all 0.2s ease;
  }

  button:enabled:hover {
    background: rgba(255, 255, 255, 0.2);
    border-color: rgba(255, 255, 255, 0.3);
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .status-text {
    display: flex;
    flex-direction: column;
    font-size: 11px;
    color: rgba(255, 255, 255, 0.8);
    margin-left: 8px;
  }

  .undo-status,
  .redo-status {
    line-height: 1.2;
  }

  .redo-status {
    margin-top: 2px;
  }
</style>
