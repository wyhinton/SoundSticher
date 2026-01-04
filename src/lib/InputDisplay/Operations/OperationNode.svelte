<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import type { OperationDef } from '$lib/state/operation';

  interface OperationNodeData {
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

  $: categoryColor = getCategoryColor(data.category);

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
  style="--category-color: {categoryColor}; {customStyleString}"
>
  <Handle type="target" position={Position.Left} />

  {#if data.customStyle}
    <!-- Simple circular node for custom styled nodes -->
    <div class="custom-content">
      <span class="custom-icon">{data.icon}</span>
    </div>
  {:else}
    <!-- Default node layout -->
    <div class="node-header">
      <span class="node-icon">{data.icon}</span>
      <span class="node-label">{data.label}</span>
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
  }

  .custom-icon {
    font-size: inherit;
    color: inherit;
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

  :global(.operation-node .svelte-flow__handle) {
    width: 10px;
    height: 10px;
    background: #45475a;
    border: 2px solid var(--category-color, #64748b);
  }
</style>
