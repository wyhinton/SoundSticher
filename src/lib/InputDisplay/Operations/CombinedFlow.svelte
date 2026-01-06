<script lang="ts">
  import { SvelteFlow, Controls, Background } from '@xyflow/svelte';
  import type { Node, Edge, NodeTypes } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';

  import type { CombineOperation } from '$lib/state/operation';
  import { OperationInfoDictionary } from '$lib/state/operation';
  import { type Section, type AudioFileItem } from '$lib/state/state.svelte';

  import SourceNode from './SourceNode.svelte';
  import OperationNode from './OperationNode.svelte';
  import { createEventDispatcher } from 'svelte';

  export let operation: CombineOperation;
  export let operationName: string;
  export let isSelected: boolean = false;

  const dispatch = createEventDispatcher<{
    operationSelect: { operationName: string };
  }>();

  $: opInfo = OperationInfoDictionary[operation.kind];
  operation.source;
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
    const VERTICAL_SPACING = 80;
    const GRID_COLUMNS = 3; // Number of columns in the grid
    const START_X = 50;
    const START_Y = 50;

    // Create source nodes for each file in each section
    const sections = operation.sections || [];
    const sourceNodeIds: string[] = [];
    let nodeIndex = 0;

    sections.forEach((section, sectionIndex) => {
      section.files.forEach((file, fileIndex) => {
        const sourceNodeId = `combine-source-${operationName}-${sectionIndex}-${fileIndex}`;
        sourceNodeIds.push(sourceNodeId);

        // Calculate grid position
        const column = nodeIndex % GRID_COLUMNS;
        const row = Math.floor(nodeIndex / GRID_COLUMNS);
        const xPosition = START_X + (column * 150); // Reduced spacing for grid
        const yPosition = START_Y + (row * VERTICAL_SPACING);

        nodes.push({
          id: sourceNodeId,
          type: 'source',
          position: { x: xPosition, y: yPosition },
          data: {
            source: { type: 'files', fileIds: [file.id] },
            label: getFileLabel(file, section),
            file: file,
            section: section,
          },
          draggable: false,
          selectable: false,
        });

        nodeIndex++;
      });
    });

    // Create operation node - position it to the right of the grid
    const totalFiles = sections.reduce((total, section) => total + section.files.length, 0);
    const gridRows = Math.ceil(totalFiles / GRID_COLUMNS);
    const gridCenterY = START_Y + ((gridRows - 1) * VERTICAL_SPACING) / 2;
    const gridWidth = (GRID_COLUMNS - 1) * 150;
    const opNodeId = `combine-op-${operationName}`;
    
    nodes.push({
      id: opNodeId,
      type: 'operation',
      position: { x: START_X + gridWidth + HORIZONTAL_SPACING, y: gridCenterY },
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

    // Create edges from each source node to the operation node
    sourceNodeIds.forEach((sourceNodeId, index) => {
      edges.push({
        id: `combine-edge-${index}-${operationName}`,
        source: sourceNodeId,
        target: opNodeId,
        type: 'bezier',
        animated: false,
        style: 'stroke: #64748b; stroke-width: 2px;',
      });
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

  function getSectionLabel(section: Section): string {
    const fileCount = section.files.length;
    const folderName = section.folderPath.split(/[/\\]/).pop() || 'Unknown Folder';
    return `${folderName} (${fileCount} files)`;
  }

  function getFileLabel(file: AudioFileItem, section: Section): string {
    const fileName = file.path.split(/[/\\]/).pop() || 'Unknown File';
    const sectionName = section.folderPath.split(/[/\\]/).pop() || 'Unknown Folder';
    return `${fileName} (${sectionName})`;
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

  // Handle flow header click for operation selection
  function handleFlowHeaderClick() {
    dispatch('operationSelect', { operationName });
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
      class:selected={isSelected}
      tabindex="0"
      on:click={handleFlowHeaderClick}
      on:keydown={handleKeydown}
      role="button"
      aria-label="Operation flow header - Click to select, Press Ctrl+Shift+Space to toggle debug info"
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
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .flow-header:hover {
    background: rgba(0, 0, 0, 0.8);
    transform: translateY(-1px);
  }

  .flow-header:focus {
    box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.5);
  }

  .flow-header.selected {
    background: rgba(59, 130, 246, 0.3);
    border: 1px solid rgba(59, 130, 246, 0.6);
  }

  .flow-header.selected:hover {
    background: rgba(59, 130, 246, 0.4);
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
