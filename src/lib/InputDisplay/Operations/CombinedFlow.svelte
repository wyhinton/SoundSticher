<script lang="ts">
  import { SvelteFlow, Controls, Background } from '@xyflow/svelte';
  import type { Node, Edge, NodeTypes } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';

  import type { CombineOperation } from '$lib/state/operation';
  import { OperationInfoDictionary } from '$lib/state/operation';
  import { appState } from '$lib/state/state.svelte';

  import SourceNode from './SourceNode.svelte';
  import OperationNode from './OperationNode.svelte';

  export let operation: CombineOperation;
  export let operationName: string;

  $: opInfo = OperationInfoDictionary[operation.kind];

  // Debug state
  let showDebugInfo = false;
  let debugInfo = { x: 0, y: 0, zoom: 1 };
  let flowInstance: any = null;
  let debugUpdateInterval: any = null;

  // Custom node types
  const nodeTypes: NodeTypes = {
    operation: OperationNode,
    source: SourceNode,
  };

  // Generate flow for this specific combine operation
  function generateCombineFlow(): { nodes: Node[]; edges: Edge[] } {
    const nodes: Node[] = [];
    const edges: Edge[] = [];

    // Layout configuration
    const HORIZONTAL_SPACING = 200;
    const START_X = 50;
    const START_Y = 50;

    // Create source node
    const sourceNodeId = `combine-source-${operationName}`;
    nodes.push({
      id: sourceNodeId,
      type: 'source',
      position: { x: START_X, y: START_Y },
      data: {
        source: operation.source,
        label: getSourceLabel(operation.source),
      },
      draggable: false,
      selectable: false,
    });

    // Create operation node
    const opNodeId = `combine-op-${operationName}`;
    nodes.push({
      id: opNodeId,
      type: 'operation',
      position: { x: START_X + HORIZONTAL_SPACING, y: START_Y },
      data: {
        name: operationName,
        kind: operation.kind,
        icon: opInfo?.icon || '🔗',
        label: opInfo?.label || 'Combine',
        category: opInfo?.category || 'render',
        def: operation,
        // Custom styling for combine operation
        customStyle: {
          background: '#3b82f6',
          borderRadius: '50%',
          width: '80px',
          height: '80px',
          border: '3px solid #1d4ed8',
          color: 'white',
          fontSize: '24px',
        },
      },
      draggable: false,
      selectable: true,
    });

    // Create edges
    edges.push({
      id: `combine-edge-1-${operationName}`,
      source: sourceNodeId,
      target: opNodeId,
      type: 'bezier',
      animated: false,
      style: 'stroke: #64748b; stroke-width: 2px;',
    });

    return { nodes, edges };
  }

  function getSourceLabel(source: CombineOperation['source']): string {
    switch (source.type) {
      case 'group':
        return `Group: ${source.groupRef}`;
      case 'files':
        return `${source.fileIds.length} Files`;
      case 'all':
        return 'All Files';
      case 'active':
        return 'Active Files';
      case 'section':
        return `Section ${source.sectionIndex}`;
      case 'previousOperation':
        return `From: ${source.operationRef}`;
      default:
        return 'Unknown Source';
    }
  }

  // Handle keyboard events for debug toggle
  function handleKeydown(event: KeyboardEvent) {
    if (event.ctrlKey && event.shiftKey && event.code === 'Space') {
      event.preventDefault();
      showDebugInfo = !showDebugInfo;
      
      if (showDebugInfo) {
        updateDebugInfo();
        // Start periodic updates when debug is shown
        debugUpdateInterval = setInterval(updateDebugInfo, 100);
      } else {
        // Clear interval when debug is hidden
        if (debugUpdateInterval) {
          clearInterval(debugUpdateInterval);
          debugUpdateInterval = null;
        }
      }
    }
  }

  // Update debug information from flow instance
  function updateDebugInfo() {
    if (flowInstance) {
      try {
        const viewport = flowInstance.getViewport();
        debugInfo = {
          x: Math.round(viewport.x * 100) / 100,
          y: Math.round(viewport.y * 100) / 100,
          zoom: Math.round(viewport.zoom * 100) / 100,
        };
      } catch (error) {
        // Fallback if getViewport is not available
        console.warn('Could not get viewport info:', error);
      }
    }
  }

  // Handle viewport changes to update debug info
  function onViewportChange(viewport: { x: number; y: number; zoom: number }) {
    if (showDebugInfo) {
      debugInfo = {
        x: Math.round(viewport.x * 100) / 100,
        y: Math.round(viewport.y * 100) / 100,
        zoom: Math.round(viewport.zoom * 100) / 100,
      };
    }
  }

  $: flowData = generateCombineFlow();
</script>

<div class="combined-flow">
  <div class="flow-content">
    <SvelteFlow
      bind:this={flowInstance}
      nodes={flowData.nodes}
      edges={flowData.edges}
      {nodeTypes}
      fitView
      fitViewOptions={{ padding: 1, includeHiddenNodes: false }}
      minZoom={0.2}
      maxZoom={1.5}
      panOnScroll={false}
      zoomOnScroll={false}
      preventScrolling={true}
      nodesDraggable={false}
      nodesConnectable={false}
      elementsSelectable={false}
      proOptions={{ hideAttribution: true }}
    >
      <Background gap={0} size={0} />
    </SvelteFlow>

    <!-- Overlaid header -->
    <div
      class="flow-header"
      tabindex="0"
      on:keydown={handleKeydown}
      role="button"
      aria-label="Flow header - Press Ctrl+Shift+Space to toggle debug info"
    >
      <span class="operation-icon">{opInfo?.icon || '🔗'}</span>
      <span class="operation-name fira font-size-12px">{operationName}</span>

      {#if showDebugInfo}
        <div class="debug-info">
          <span class="debug-label">Debug:</span>
          <span class="debug-value">x: {debugInfo.x}</span>
          <span class="debug-value">y: {debugInfo.y}</span>
          <span class="debug-value">zoom: {debugInfo.zoom}</span>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .combined-flow {
    background: var(--panel-bg, #1e1e2e);
    overflow: hidden;
    min-width: 480px;
    height: 100%;
    display: flex;
    flex-direction: column;
    border-right: 1px solid #374151;
  }

  .flow-header {
    position: absolute;
    top: 4px;
    left: 4px;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(4px);
    padding: 2px 2px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    gap: 2px;
    z-index: 10;
    outline: none;
  }

  .flow-header:focus {
    box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.5);
  }

  .debug-info {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-left: 8px;
    padding: 2px 4px;
    background: rgba(59, 130, 246, 0.2);
    border-radius: 4px;
    font-family: 'Fira Code', monospace;
    font-size: 10px;
  }

  .debug-label {
    color: #60a5fa;
    font-weight: bold;
  }

  .debug-value {
    color: #e5e7eb;
    font-weight: normal;
  }

  .operation-icon {
    font-size: 1rem;
  }

  .operation-name {
    color: #ffffff;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.7);
    white-space: nowrap;
  }

  .flow-content {
    flex: 1;
    position: relative;
    background: #000000;
  }

  /* Override SvelteFlow styles for this specific instance */
  :global(.combined-flow .svelte-flow) {
    background: #000000 !important;
  }

  :global(.combined-flow .svelte-flow__background) {
    background: #000000 !important;
  }

  :global(.combined-flow .svelte-flow__viewport) {
    background: #000000 !important;
  }
</style>
