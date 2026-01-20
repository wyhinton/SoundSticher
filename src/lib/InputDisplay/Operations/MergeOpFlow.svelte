<script lang="ts">
  import { SvelteFlow, Background, useSvelteFlow } from '@xyflow/svelte';
  import type { Node, Edge, NodeTypes } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';

  import type { MergeOp, OperationSource, OperationId } from '$lib/state/operation';
  import { OperationInfoDictionary } from '$lib/state/operation';

  import SourceNode from './SourceNode.svelte';
  import OperationNode from './OperationNode.svelte';
  import OpFlowHeader from './OpFlowHeader.svelte';
  import { onMount } from 'svelte';

  export let operation: MergeOp;
  export let operationId: OperationId;
  export let operationName: string;
  export let isSelected: boolean = false;
  export let panelHeight: number;
  const { fitView } = useSvelteFlow();

  $: {
    if (panelHeight > 0) {
      fitView({ padding: 1 });
    }
  }
  $: opInfo = OperationInfoDictionary[operation.kind];

  // Create a reactive key that includes sources data to ensure re-rendering
  $: mergeOpSources = JSON.stringify(operation.sources || []);

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

  // Generate flow for this specific merge operation
  function generateCombineFlow(_sourcesRevision?: string): { nodes: Node[]; edges: Edge[] } {
    const nodes: Node[] = [];
    const edges: Edge[] = [];

    // Layout configuration
    const HORIZONTAL_SPACING = 200;
    const VERTICAL_SPACING = 80;
    const GRID_COLUMNS = 3; // Number of columns in the grid
    const START_X = 50;
    const START_Y = 50;

    // Create source nodes for each source in the operation
    const sources = operation.sources || [];
    const sourceNodeIds: string[] = [];
    let nodeIndex = 0;

    sources.forEach((source, sourceIndex) => {
      const sourceNodeId = `merge-source-${operationId}-${sourceIndex}`;
      sourceNodeIds.push(sourceNodeId);

      // Calculate grid position
      const column = nodeIndex % GRID_COLUMNS;
      const row = Math.floor(nodeIndex / GRID_COLUMNS);
      const xPosition = START_X + column * 150; // Reduced spacing for grid
      const yPosition = START_Y + row * VERTICAL_SPACING;

      nodes.push({
        id: sourceNodeId,
        type: 'source',
        position: { x: xPosition, y: yPosition },
        data: {
          source: source,
          label: getSourceLabel(source),
          sourceIndex: sourceIndex,
        },
        draggable: false,
        selectable: false,
      });

      nodeIndex++;
    });

    // Create operation node - position it to the right of the grid
    const totalSources = sources.length;
    const gridRows = Math.ceil(totalSources / GRID_COLUMNS);
    const gridCenterY = START_Y + ((gridRows - 1) * VERTICAL_SPACING) / 2;
    const gridWidth = (GRID_COLUMNS - 1) * 150;
    const opNodeId = `merge-op-${operationId}`;

    nodes.push({
      id: opNodeId,
      type: 'operation',
      position: { x: START_X + gridWidth + HORIZONTAL_SPACING, y: gridCenterY },
      data: {
        id: operationId,
        name: operationName,
        kind: operation.kind,
        icon: opInfo?.icon || '🔗',
        label: opInfo?.label || 'Merge',
        category: opInfo?.category || 'render',
        def: operation,
        // Custom styling for merge operation
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

    // Create edges from each source node to the operation node
    sourceNodeIds.forEach((sourceNodeId, index) => {
      edges.push({
        id: `merge-edge-${index}-${operationId}`,
        source: sourceNodeId,
        target: opNodeId,
        type: 'bezier',
        animated: false,
        style: 'stroke: #64748b; stroke-width: 2px;',
      });
    });

    return { nodes, edges };
  }

  function getSourceLabel(source: OperationSource): string {
    console.log(source);
    switch (source.type) {
      case 'group':
        return `Group: ${source.groupRef}`;
      case 'file':
        return `File: ${source.fileId}`;
      case 'files':
        return `${source.fileIds.length} Files`;
      case 'all':
        return 'All Files';
      case 'active':
        return 'Active Files';
      case 'section':
        return `Section ${source.sectionIndex}`;
      case 'operation':
        return `From: ${source.operationId}`;
      case 'previousOperation':
        return `From: ${source.operationId}`;
      default:
        return 'Unknown Source';
    }
  }

  // Handle keyboard events for debug toggle

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

  onMount(() => {
    setTimeout(() => {
      fitView({ padding: 1 });
    }, 100);
  });

  // Handle viewport changes to update debug info

  $: flowData = generateCombineFlow(mergeOpSources);
</script>

<div class="merge-op-flow">
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

    <!-- Operation flow header component -->
    <OpFlowHeader
      {operationId}
      {operationName}
      {isSelected}
      {opInfo}
      bind:showDebugInfo
      bind:debugInfo
      on:toggleDebug={({ detail }) => {
        showDebugInfo = detail.showDebugInfo;
        if (showDebugInfo) {
          updateDebugInfo();
          debugUpdateInterval = setInterval(updateDebugInfo, 100);
        } else {
          if (debugUpdateInterval) {
            clearInterval(debugUpdateInterval);
            debugUpdateInterval = null;
          }
        }
      }}
    />
  </div>
</div>

<style>
  .merge-op-flow {
    background: var(--panel-bg, #1e1e2e);
    overflow: hidden;
    min-width: 480px;
    height: 100%;
    display: flex;
    flex-direction: column;
    border-right: 1px solid #374151;
  }

  .flow-content {
    flex: 1;
    position: relative;
    background: #000000;
  }

  /* Override SvelteFlow styles for this specific instance */
  :global(.merge-op-flow .svelte-flow) {
    background: #000000 !important;
  }

  :global(.merge-op-flow .svelte-flow__background) {
    background: #000000 !important;
  }

  :global(.merge-op-flow .svelte-flow__viewport) {
    background: #000000 !important;
  }
</style>
