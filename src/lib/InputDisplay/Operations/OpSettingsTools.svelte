<script lang="ts">
  import type { OperationId } from '$lib/state/operation';
  import { type RenderPolicy } from '$lib/state/operation';
  import { appState } from '$lib/state/state.svelte';
  import { setRenderPolicyCommand } from '$lib/state/undo/undo';

  export let operationId: OperationId;
  export let operationName: string;

  // Reactive state for render policy
  $: operation = $appState.operations?.defs?.[operationId];
  $: renderPolicy = (operation?.renderPolicy || 'auto') as RenderPolicy;
  $: isFrozen = renderPolicy === 'frozen';
  $: isAuto = renderPolicy === 'auto';

  // Action handlers
  function handleRender() {
    // Set policy to 'auto' to enable automatic re-rendering
    setRenderPolicyCommand(operationId, 'auto', 'Enable Auto-Render');

    console.log(`🚩 Render operation: ${operationName} (id: ${operationId})`);
    console.log('  → Render policy set to "auto"');
    console.log('  → Operation will now auto-rerender when upstream changes occur');

    // TODO: Trigger actual render/build of the operation output
    // This would call the execution engine to rebuild the playback graph
  }

  function handleFreeze() {
    // Toggle between 'auto' and 'frozen'
    const newPolicy: RenderPolicy = isFrozen ? 'auto' : 'frozen';
    setRenderPolicyCommand(operationId, newPolicy);

    console.log(`❄️ Toggled freeze for: ${operationName} (id: ${operationId})`);
    console.log(`  → New render policy: ${newPolicy}`);

    if (newPolicy === 'frozen') {
      console.log("  → Operation output is now frozen (won't auto-rerender on upstream changes)");
    } else {
      console.log('  → Operation will now auto-rerender when upstream changes occur');
    }
  }
</script>

<div class="op-settings-tools">
  <button
    class="tool-button"
    class:active={isAuto}
    on:click={handleRender}
    title="Render operation - Force rebuild and cache output"
    aria-label="Render operation"
  >
    <span class="tool-icon" class:active={isAuto}>🚩</span>
  </button>

  <button
    class="tool-button"
    class:frozen={isFrozen}
    on:click={handleFreeze}
    title={isFrozen
      ? 'Unfreeze operation - Enable auto-rerendering'
      : 'Freeze operation - Prevent auto-rerendering'}
    aria-label={isFrozen ? 'Unfreeze operation' : 'Freeze operation'}
  >
    <span class="tool-icon" class:frozen={isFrozen}>❄️</span>
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

  .tool-button.frozen {
    background: rgba(96, 165, 250, 0.2);
  }

  .tool-button.frozen:hover {
    background: rgba(96, 165, 250, 0.3);
  }

  .tool-button.active {
    background: rgba(239, 68, 68, 0.15);
  }

  .tool-button.active:hover {
    background: rgba(239, 68, 68, 0.25);
  }

  .tool-icon {
    font-size: 1.25rem;
    line-height: 1;
    filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.7));
    transition:
      filter 0.2s ease,
      opacity 0.2s ease;
    opacity: 0.5; /* Default: half opacity */
  }

  /* Full opacity when active/frozen */
  .tool-icon.active,
  .tool-icon.frozen {
    opacity: 1;
  }

  .tool-button:hover .tool-icon {
    filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.9));
  }

  /* Render button - red glow on hover */
  .tool-button:nth-child(1):hover .tool-icon {
    filter: drop-shadow(0 0 4px rgba(239, 68, 68, 0.6));
  }

  /* Freeze button - blue glow on hover */
  .tool-button:nth-child(2):hover .tool-icon {
    filter: drop-shadow(0 0 4px rgba(96, 165, 250, 0.6));
  }

  /* Active state - brighter red glow */
  .tool-button.active .tool-icon {
    filter: drop-shadow(0 0 6px rgba(239, 68, 68, 0.8));
  }

  /* Frozen state - brighter blue glow */
  .tool-button.frozen .tool-icon {
    filter: drop-shadow(0 0 6px rgba(96, 165, 250, 0.8));
  }
</style>
