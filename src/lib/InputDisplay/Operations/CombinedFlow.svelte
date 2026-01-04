<script lang="ts">
  import { SvelteFlow, Controls, Background } from '@xyflow/svelte';
  import type { Node, Edge, NodeTypes } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';

  import type { CombineOperation } from '$lib/state/operation';
  import { OperationInfoDictionary } from '$lib/state/operation';
  import { appState } from '$lib/state/state.svelte';

  import SourceNode from './SourceNode.svelte';
  import OperationNode from './OperationNode.svelte';
  import OutputNode from './OutputNode.svelte';

  export let operation: CombineOperation;
  export let operationName: string;

  $: opInfo = OperationInfoDictionary[operation.kind];

  // Custom node types
  const nodeTypes: NodeTypes = {
    operation: OperationNode,
    source: SourceNode,
    output: OutputNode,
  };

  // Generate flow for this specific combine operation
  function generateCombineFlow(): { nodes: Node[]; edges: Edge[] } {
    const nodes: Node[] = [];
    const edges: Edge[] = [];

    // Layout configuration
    const HORIZONTAL_SPACING = 200;
    const START_X = 20;
    const START_Y = 60;

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
        icon: opInfo.icon,
        label: opInfo.label,
        category: opInfo.category,
        def: operation,
      },
      draggable: false,
      selectable: true,
    });

    // Create output node
    const outputNodeId = `combine-output-${operationName}`;
    nodes.push({
      id: outputNodeId,
      type: 'output',
      position: { x: START_X + HORIZONTAL_SPACING * 2, y: START_Y },
      data: {
        operationName: operationName,
        format: operation.format || 'wav',
      },
      draggable: false,
      selectable: false,
    });

    // Create edges
    edges.push({
      id: `combine-edge-1-${operationName}`,
      source: sourceNodeId,
      target: opNodeId,
      type: 'smoothstep',
      animated: false,
      style: 'stroke: #64748b; stroke-width: 2px;',
    });

    edges.push({
      id: `combine-edge-2-${operationName}`,
      source: opNodeId,
      target: outputNodeId,
      type: 'smoothstep',
      animated: false,
      style: 'stroke: #22c55e; stroke-width: 2px;',
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

  $: flowData = generateCombineFlow();
</script>

<div class="combined-flow">
  <div class="flow-content">
    <SvelteFlow
      nodes={flowData.nodes}
      edges={flowData.edges}
      {nodeTypes}
      fitView
      minZoom={0.5}
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
    <div class="flow-header">
      <span class="operation-icon">{opInfo.icon}</span>
      <span class="operation-name fira font-size-12px">{operationName}</span>
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
