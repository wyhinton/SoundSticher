<script lang="ts">
  import type { OperationId } from '$lib/state/operation';
  import { type RenderPolicy } from '$lib/state/operation';
  import { appState } from '$lib/state/state.svelte';
  import {
    isOperationTimelineVisible,
    toggleTimelineVisibilityByOpId,
  } from '$lib/state/timeline/timelines';
  import { setRenderPolicyCommand } from '$lib/state/undo/undo';

  export let operationId: OperationId;
  export let operationName: string;

  // Reactive state for render policy
  $: operation = $appState.operations?.defs?.[operationId];
  $: renderPolicy = (operation?.renderPolicy || 'auto') as RenderPolicy;
  $: isFrozen = renderPolicy === 'frozen';
  $: isAuto = renderPolicy === 'auto';

  // Reactive state for timeline visibility
  $: hasVisibleTimeline = isOperationTimelineVisible(operationId);

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

  function handleTimelineToggle() {
    toggleTimelineVisibilityByOpId(operationId);

    console.log(`👁️ Toggled timeline for: ${operationName} (id: ${operationId})`);
    console.log(`  → Timeline is now ${hasVisibleTimeline ? 'hidden' : 'visible'}`);
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
    <i class="tool-icon fa-solid fa-flag" class:active={isAuto}></i>
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
    <i class="tool-icon fa-solid fa-snowflake" class:frozen={isFrozen}></i>
  </button>

  <button
    class="tool-button"
    class:visible={$hasVisibleTimeline}
    on:click={handleTimelineToggle}
    title={$hasVisibleTimeline ? 'Hide operation timeline' : 'Show operation timeline'}
    aria-label={$hasVisibleTimeline ? 'Hide timeline' : 'Show timeline'}
  >
    <i
      class="tool-icon fa-solid {$hasVisibleTimeline ? 'fa-eye' : 'fa-eye-slash'}"
      class:visible={$hasVisibleTimeline}
    ></i>
  </button>
</div>

<style>
  .op-settings-tools {
    display: flex;
    align-items: center;
    gap: 0px;
    z-index: 10;
  }

  .tool-button {
    background: rgb(54 54 54);
    border: 1px solid rgb(43, 43, 43);
    padding: 2px 4px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    outline: none;
    transition: all 0.2s ease;
  }

  .tool-button:hover {
    background: rgba(0, 0, 0, 0.6);
    border-color: rgba(255, 255, 255, 0.2);
  }

  .tool-icon {
    font-size: 1rem;
    line-height: 1;
    transition:
      color 0.2s ease,
      filter 0.2s ease;
    color: rgba(255, 255, 255, 0.5); /* Default: muted white */
  }

  /* Icon color changes when active/frozen/visible */
  .tool-icon.active {
    color: rgb(239, 68, 68); /* Red for active render */
  }

  .tool-icon.frozen {
    color: rgb(96, 165, 250); /* Blue for frozen */
  }

  .tool-icon.visible {
    color: rgb(34, 197, 94); /* Green for visible timeline */
  }
</style>
