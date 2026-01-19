<script lang="ts">
  import type { OperationId } from '$lib/state/operation';

  export let operationId: OperationId;
  export let operationName: string;

  // Action handlers (TODO: implement functionality)
  function handleRender() {
    console.log(`🚩 Render operation: ${operationName} (id: ${operationId})`);
    // TODO: Implement render functionality
    // This could trigger building the playback graph or exporting the operation
  }

  function handleFreeze() {
    console.log(`❄️ Freeze operation: ${operationName} (id: ${operationId})`);
    // TODO: Implement freeze functionality
    // This could cache the operation output or prevent further modifications
  }
</script>

<div class="op-settings-tools">
  <button
    class="tool-button"
    on:click={handleRender}
    title="Render operation - Build and cache output"
    aria-label="Render operation"
  >
    <span class="tool-icon">🚩</span>
  </button>

  <button
    class="tool-button"
    on:click={handleFreeze}
    title="Freeze operation - Lock and prevent changes"
    aria-label="Freeze operation"
  >
    <span class="tool-icon">❄️</span>
  </button>
</div>

<style>
  .op-settings-tools {
    position: absolute;
    top: 4px;
    right: 4px;
    display: flex;
    align-items: center;
    gap: 0px;
    z-index: 10;
  }

  .tool-button {
    background: transparent;
    border: none;
    padding: 4px 2px;
    border-radius: 6px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    outline: none;
    transition: all 0.2s ease;
    backdrop-filter: blur(4px);
  }

  .tool-button:hover {
    background: rgba(0, 0, 0, 0.5);
    transform: translateY(-1px) scale(1.05);
  }

  .tool-button:active {
    transform: translateY(0) scale(0.95);
  }

  .tool-button:focus {
    box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.5);
  }

  .tool-icon {
    font-size: 1.25rem;
    line-height: 1;
    filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.7));
    transition: filter 0.2s ease;
  }

  .tool-button:hover .tool-icon {
    filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.9));
  }

  /* Optional: Add specific hover effects for each button */
  .tool-button:nth-child(1):hover .tool-icon {
    /* Render button - slight glow effect */
    filter: drop-shadow(0 0 4px rgba(239, 68, 68, 0.6));
  }

  .tool-button:nth-child(2):hover .tool-icon {
    /* Freeze button - slight glow effect */
    filter: drop-shadow(0 0 4px rgba(96, 165, 250, 0.6));
  }
</style>
