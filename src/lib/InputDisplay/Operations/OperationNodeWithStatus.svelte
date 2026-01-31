<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import type { OperationDef } from '$lib/state/operation';
  import { createOperationRenderStore, type OperationRenderState } from '$lib/state/autoRender';

  interface OperationNodeData {
    id: string; // Operation ID for render state tracking
    name: string;
    kind: OperationDef['kind'];
    icon: string;
    label: string;
    category: 'render' | 'edit' | 'meta';
    def: OperationDef;
    customStyle?: {
      background?: string;
      borderRadius?: string;
      width?: string;
      height?: string;
      border?: string;
      color?: string;
      fontSize?: string;
    };
  }

  export let data: OperationNodeData;
  export let selected: boolean = false;

  // Subscribe to this operation's render state
  const operationRenderState = createOperationRenderStore(data.id);

  function getCategoryColor(category: string): string {
    switch (category) {
      case 'render':
        return '#f59e0b';
      case 'edit':
        return '#8b5cf6';
      case 'meta':
        return '#22c55e';
      default:
        return '#64748b';
    }
  }

  function getStatusColor(status: OperationRenderState['status']): string {
    switch (status) {
      case 'pending':
        return '#6b7280'; // Gray
      case 'rendering':
        return '#3b82f6'; // Blue
      case 'success':
        return '#10b981'; // Green
      case 'error':
        return '#ef4444'; // Red
      case 'skipped':
        return '#f59e0b'; // Orange
      default:
        return '#6b7280'; // Default gray
    }
  }

  //   function getStatusIcon(status: OperationRenderState['status']): string {
  //     switch (status) {
  //       case 'pending':
  //         return '⏸️';
  //       case 'rendering':
  //         return '⏳';
  //       case 'success':
  //         return '✅';
  //       case 'error':
  //         return '❌';
  //       case 'skipped':
  //         return '⏭️';
  //       default:
  //         return '⚪';
  //     }
  //   }

  $: categoryColor = getCategoryColor(data.category);
  $: statusColor = $operationRenderState ? getStatusColor($operationRenderState.status) : '#6b7280';
  //   $: statusIcon = $operationRenderState ? getStatusIcon($operationRenderState.status) : '⚪';

  // Create style object for custom styling
  $: customStyleString = data.customStyle
    ? Object.entries(data.customStyle)
        .map(([key, value]) => `${key.replace(/([A-Z])/g, '-$1').toLowerCase()}: ${value}`)
        .join('; ')
    : '';
</script>

<div
  class="operation-node"
  class:selected
  class:custom-styled={!!data.customStyle}
  style="--category-color: {categoryColor}; --status-color: {statusColor}; {customStyleString}"
>
  <Handle type="target" position={Position.Left} />

  {#if data.customStyle}
    <!-- Custom styled circular node with status indicator -->
    <div class="custom-content">
      <span class="custom-icon">{data.icon}</span>
      <!-- Status indicator positioned at top-right of the circle -->
      <div class="status-indicator" title={$operationRenderState?.status || 'No status'}>
        <div
          class="status-circle"
          class:pulsing={$operationRenderState?.status === 'rendering'}
        ></div>
      </div>
    </div>
  {:else}
    <!-- Default node layout with status indicator -->
    <div class="node-header">
      <span class="node-icon">{data.icon}</span>
      <span class="node-label">{data.label}</span>
      <div class="status-indicator-inline" title={$operationRenderState?.status || 'No status'}>
        <div
          class="status-circle-small"
          class:pulsing={$operationRenderState?.status === 'rendering'}
        ></div>
      </div>
    </div>

    <div class="node-name">{data.name}</div>

    <div class="node-category">
      <span class="category-badge" style="background: {categoryColor}20; color: {categoryColor}">
        {data.category}
      </span>
    </div>
  {/if}

  <Handle type="source" position={Position.Right} />
</div>

<style>
  .operation-node {
    background: #1e1e2e;
    border: 2px solid #313244;
    border-radius: 8px;
    padding: 12px;
    min-width: 160px;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.3);
    transition: all 0.2s ease;
    position: relative;
  }

  .operation-node.custom-styled {
    padding: 0;
    min-width: auto;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .custom-content {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
  }

  .custom-icon {
    font-size: inherit;
    color: inherit;
  }

  /* Status indicator for custom styled nodes (positioned at top-right) */
  .status-indicator {
    position: absolute;
    top: -6px;
    right: -6px;
    z-index: 10;
  }

  .status-circle {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--status-color);
    border: 2px solid #1e1e2e;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.3);
    transition: all 0.2s ease;
  }

  .status-icon {
    font-size: 8px;
    line-height: 1;
  }

  /* Status indicator for default nodes (inline in header) */
  .status-indicator-inline {
    margin-left: auto;
  }

  .status-circle-small {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--status-color);
    border: 1px solid #1e1e2e;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
    transition: all 0.2s ease;
  }

  .status-icon-small {
    font-size: 7px;
    line-height: 1;
  }

  /* Pulsing animation for rendering status */
  .pulsing {
    animation: pulse 1.5s ease-in-out infinite;
  }

  @keyframes pulse {
    0%,
    100% {
      transform: scale(1);
      opacity: 1;
    }
    50% {
      transform: scale(1.1);
      opacity: 0.8;
    }
  }

  .operation-node:hover {
    border-color: var(--category-color);
    box-shadow: 0 0 12px color-mix(in srgb, var(--category-color) 30%, transparent);
  }

  .operation-node.selected {
    border-color: var(--category-color);
    box-shadow: 0 0 16px color-mix(in srgb, var(--category-color) 50%, transparent);
  }

  .node-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 6px;
  }

  .node-icon {
    font-size: 1.2rem;
  }

  .node-label {
    font-weight: 600;
    color: #cdd6f4;
    font-size: 0.9rem;
  }

  .node-name {
    color: #a6adc8;
    font-size: 0.75rem;
    font-family: 'Fira Code', monospace;
    background: #11111b;
    padding: 4px 8px;
    border-radius: 4px;
    margin-bottom: 8px;
    word-break: break-all;
  }

  .node-category {
    display: flex;
    justify-content: flex-end;
  }

  .category-badge {
    font-size: 0.65rem;
    padding: 2px 6px;
    border-radius: 4px;
    font-weight: 500;
    text-transform: uppercase;
  }

  :global(.svelte-flow__handle) {
    width: 0px;
    height: 0px;
    border: 0px !important;
    background: transparent !important;
  }
</style>
