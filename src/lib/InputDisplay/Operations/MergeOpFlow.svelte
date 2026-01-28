<script lang="ts">
  import { SvelteFlow, Background, useSvelteFlow } from '@xyflow/svelte';
  import type { Node, Edge, NodeTypes } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';

  import type { MergeOp, OperationSource, OperationId } from '$lib/state/operation';
  import { operationMeta, type OperationKind } from '$lib/types';

  import SourceNode from './SourceNode.svelte';
  import OperationNode from './OperationNode.svelte';
  import OpFlowHeader from './OpFlowHeader.svelte';
  import { onMount } from 'svelte';
  import { createOperationRenderStore } from '$lib/state/autoRender';
  import OpFooter from './OpFooter.svelte';

  export let operation: MergeOp;
  export let operationId: OperationId;
  export let operationName: string;
  export let isSelected: boolean = false;
  export let panelHeight: number;
  export let rev: string;

  const { fitView } = useSvelteFlow();

  // Subscribe to this operation's render state
  const operationRenderState = createOperationRenderStore(operationId);

  $: {
    if (panelHeight > 0) {
      fitView({ padding: 1 });
    }
  }

  $: {
    if (rev) {
      fitView({ padding: 1 });
    }
  }

  $: opInfo = operationMeta[operation.kind as OperationKind];

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

<div class="merge-op-flow position-relative">
  <!-- Render Status Indicator -->
  {#if $operationRenderState}
    <div
      class="render-status-overlay"
      class:rendering={$operationRenderState.status === 'rendering'}
      class:success={$operationRenderState.status === 'success'}
      class:error={$operationRenderState.status === 'error'}
      class:skipped={$operationRenderState.status === 'skipped'}
    >
      <div class="status-content">
        {#if $operationRenderState.status === 'rendering'}
          <span class="status-icon">⏳</span>
          <span class="status-text"
            >Rendering... ({$operationRenderState.index}/{$operationRenderState.totalOperations})</span
          >
        {:else if $operationRenderState.status === 'success'}
          <span class="status-icon">✅</span>
          <span class="status-text">Rendered in {$operationRenderState.duration_ms}ms</span>
        {:else if $operationRenderState.status === 'error'}
          <span class="status-icon">❌</span>
          <span class="status-text">Error: {$operationRenderState.error ?? 'Unknown error'}</span>
        {:else if $operationRenderState.status === 'skipped'}
          <span class="status-icon">⏭️</span>
          <span class="status-text">Skipped</span>
        {:else if $operationRenderState.status === 'pending'}
          <span class="status-icon">⏸️</span>
          <span class="status-text">Pending...</span>
        {/if}
      </div>
    </div>
  {/if}

  <div class="flow-content">
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

    <!-- Operation settings tools overlay -->

    <!-- Render Info Footer Overlay -->
    {#if $operationRenderState}
      <div
        class="render-footer-overlay"
        class:rendering={$operationRenderState.status === 'rendering'}
        class:success={$operationRenderState.status === 'success'}
        class:error={$operationRenderState.status === 'error'}
        class:skipped={$operationRenderState.status === 'skipped'}
      >
        <div class="footer-content">
          <div class="footer-section">
            <span class="footer-label">Status:</span>
            <span
              class="footer-value status-badge-footer"
              class:rendering={$operationRenderState.status === 'rendering'}
              class:success={$operationRenderState.status === 'success'}
              class:error={$operationRenderState.status === 'error'}
              class:skipped={$operationRenderState.status === 'skipped'}
            >
              {$operationRenderState.status}
            </span>
          </div>

          <div class="footer-divider"></div>

          <div class="footer-section">
            <span class="footer-label">Progress:</span>
            <span class="footer-value"
              >{$operationRenderState.index} / {$operationRenderState.totalOperations}</span
            >
          </div>

          {#if $operationRenderState.duration_ms !== undefined}
            <div class="footer-divider"></div>
            <div class="footer-section">
              <span class="footer-label">Duration:</span>
              <span class="footer-value">{$operationRenderState.duration_ms}ms</span>
            </div>
          {/if}

          {#if $operationRenderState.status === 'rendering' && $operationRenderState.startedAt}
            <div class="footer-divider"></div>
            <div class="footer-section">
              <span class="footer-label">Started:</span>
              <span class="footer-value"
                >{new Date($operationRenderState.startedAt).toLocaleTimeString()}</span
              >
            </div>
          {/if}

          {#if $operationRenderState.error}
            <div class="footer-divider"></div>
            <div class="footer-section footer-error">
              <span class="footer-label">Error:</span>
              <span class="footer-value error-text">{$operationRenderState.error}</span>
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <!-- Debug toggle button -->
    <button
      class="debug-toggle-btn"
      class:active={showDebugInfo}
      on:click={() => {
        showDebugInfo = !showDebugInfo;
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
      title="Toggle Debug Info"
      aria-label="Toggle Debug Info"
    >
      <svg
        width="16"
        height="16"
        viewBox="0 0 16 16"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
      >
        <path
          d="M8 2L10 4L14 4L14 8L12 10L14 12L14 14L10 14L8 16L6 14L2 14L2 10L4 8L2 6L2 2L6 2L8 2Z"
          stroke="currentColor"
          stroke-width="1.5"
          fill="none"
        />
        <circle cx="8" cy="8" r="2" fill="currentColor" />
      </svg>
    </button>

    <!-- Enhanced Debug Info Panel -->
    {#if showDebugInfo}
      <div class="debug-panel">
        <div class="debug-section">
          <div class="debug-title">Viewport</div>
          <div class="debug-row">
            <span class="debug-label">X:</span>
            <span class="debug-value">{debugInfo.x}</span>
          </div>
          <div class="debug-row">
            <span class="debug-label">Y:</span>
            <span class="debug-value">{debugInfo.y}</span>
          </div>
          <div class="debug-row">
            <span class="debug-label">Zoom:</span>
            <span class="debug-value">{debugInfo.zoom}</span>
          </div>
        </div>

        {#if $operationRenderState}
          <div class="debug-section">
            <div class="debug-title">Render State</div>
            <div class="debug-row">
              <span class="debug-label">Status:</span>
              <span
                class="debug-value status-badge"
                class:rendering={$operationRenderState.status === 'rendering'}
                class:success={$operationRenderState.status === 'success'}
                class:error={$operationRenderState.status === 'error'}
              >
                {$operationRenderState.status}
              </span>
            </div>
            <div class="debug-row">
              <span class="debug-label">Name:</span>
              <span class="debug-value">{$operationRenderState.name}</span>
            </div>
            <div class="debug-row">
              <span class="debug-label">Index:</span>
              <span class="debug-value"
                >{$operationRenderState.index} / {$operationRenderState.totalOperations}</span
              >
            </div>
            {#if $operationRenderState.duration_ms !== undefined}
              <div class="debug-row">
                <span class="debug-label">Duration:</span>
                <span class="debug-value">{$operationRenderState.duration_ms}ms</span>
              </div>
            {/if}
            {#if $operationRenderState.error}
              <div class="debug-row">
                <span class="debug-label">Error:</span>
                <span class="debug-value error-text">{$operationRenderState.error}</span>
              </div>
            {/if}
            {#if $operationRenderState.startedAt}
              <div class="debug-row">
                <span class="debug-label">Started:</span>
                <span class="debug-value"
                  >{new Date($operationRenderState.startedAt).toLocaleTimeString()}</span
                >
              </div>
            {/if}
            {#if $operationRenderState.completedAt}
              <div class="debug-row">
                <span class="debug-label">Completed:</span>
                <span class="debug-value"
                  >{new Date($operationRenderState.completedAt).toLocaleTimeString()}</span
                >
              </div>
            {/if}
          </div>
        {:else}
          <div class="debug-section">
            <div class="debug-title">Render State</div>
            <div class="debug-row">
              <span class="debug-value muted">Not rendered yet</span>
            </div>
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Footer Render Status Overlay -->
  <OpFooter {operationId} />
</div>

<style>
  .merge-op-flow {
    overflow: hidden;
    /* min-width: 480px; */
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

  .render-status-overlay {
    position: absolute;
    top: 8px;
    right: 8px;
    z-index: 10;
    background: rgba(0, 0, 0, 0.8);
    border: 1px solid #374151;
    border-radius: 6px;
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 500;
    backdrop-filter: blur(4px);
    transition: all 0.2s ease;
  }

  .render-status-overlay.rendering {
    border-color: #3b82f6;
    background: rgba(59, 130, 246, 0.15);
  }

  .render-status-overlay.success {
    border-color: #10b981;
    background: rgba(16, 185, 129, 0.15);
  }

  .render-status-overlay.error {
    border-color: #ef4444;
    background: rgba(239, 68, 68, 0.15);
  }

  .render-status-overlay.skipped {
    border-color: #6b7280;
    background: rgba(107, 114, 128, 0.15);
  }

  .status-content {
    display: flex;
    align-items: center;
    gap: 6px;
    color: #e5e7eb;
  }

  .status-icon {
    font-size: 14px;
  }

  .status-text {
    white-space: nowrap;
  }

  .render-status-overlay.rendering .status-icon {
    animation: pulse 1.5s ease-in-out infinite;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.5;
    }
  }

  /* Footer Render Status Overlay */
  .footer-render-overlay {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    background: rgba(30, 30, 30, 0.95);
    backdrop-filter: blur(10px);
    border-top: 1px solid rgba(255, 255, 255, 0.1);
    padding: 2px 4px;
    z-index: 50;
    box-shadow: 0 -2px 8px rgba(0, 0, 0, 0.3);
  }

  .footer-content {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    font-size: 12px;
  }

  .footer-section {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .footer-section.footer-error {
    flex: 1;
    min-width: 200px;
  }

  .footer-label {
    color: rgba(255, 255, 255, 0.6);
    font-weight: 500;
  }

  .footer-value {
    color: rgba(255, 255, 255, 0.9);
  }

  .footer-value.error-text {
    color: #ff6b6b;
    font-family: monospace;
    font-size: 11px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 400px;
  }

  .footer-badge {
    display: inline-flex;
    align-items: center;
    padding: 0px 4px;
    border-radius: 12px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.3px;
    transition: all 0.2s ease;
  }

  .footer-badge.rendering {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    color: white;
    animation: pulse 2s ease-in-out infinite;
  }

  .footer-badge.success {
    background: linear-gradient(135deg, #56ab2f 0%, #a8e063 100%);
    color: white;
  }

  .footer-badge.error {
    background: linear-gradient(135deg, #ff6b6b 0%, #ee5a6f 100%);
    color: white;
  }

  .footer-badge.skipped {
    background: linear-gradient(135deg, #ffa726 0%, #fb8c00 100%);
    color: white;
  }

  .footer-badge.pending {
    background: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.7);
  }

  .footer-divider {
    width: 1px;
    height: 16px;
    background: rgba(255, 255, 255, 0.15);
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.7;
    }
  }

  .debug-toggle-btn {
    position: absolute;
    bottom: 12px;
    right: 12px;
    z-index: 10;
    width: 36px;
    height: 36px;
    border-radius: 8px;
    background: rgba(0, 0, 0, 0.8);
    border: 1px solid #374151;
    color: #9ca3af;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s ease;
    backdrop-filter: blur(4px);
  }

  .debug-toggle-btn:hover {
    background: rgba(0, 0, 0, 0.9);
    border-color: #3b82f6;
    color: #3b82f6;
    transform: scale(1.05);
  }

  .debug-toggle-btn.active {
    background: rgba(59, 130, 246, 0.2);
    border-color: #3b82f6;
    color: #3b82f6;
  }

  .debug-toggle-btn:active {
    transform: scale(0.95);
  }

  .debug-panel {
    position: absolute;
    bottom: 56px;
    right: 12px;
    z-index: 9;
    background: rgba(0, 0, 0, 0.95);
    border: 1px solid #374151;
    border-radius: 8px;
    padding: 12px;
    font-size: 11px;
    font-family: 'Courier New', monospace;
    max-width: 280px;
    backdrop-filter: blur(8px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
  }

  .debug-section {
    margin-bottom: 12px;
  }

  .debug-section:last-child {
    margin-bottom: 0;
  }

  .debug-title {
    color: #3b82f6;
    font-weight: 600;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 6px;
    padding-bottom: 4px;
    border-bottom: 1px solid #374151;
  }

  .debug-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 4px;
    gap: 8px;
  }

  .debug-row:last-child {
    margin-bottom: 0;
  }

  .debug-label {
    color: #6b7280;
    font-weight: 500;
    min-width: 70px;
  }

  .debug-value {
    color: #e5e7eb;
    font-weight: 400;
    word-break: break-word;
    text-align: right;
  }

  .debug-value.muted {
    color: #6b7280;
    font-style: italic;
  }

  .debug-value.error-text {
    color: #ef4444;
    font-size: 10px;
  }

  .status-badge {
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    background: rgba(107, 114, 128, 0.3);
    border: 1px solid #6b7280;
  }

  .status-badge.rendering {
    background: rgba(59, 130, 246, 0.3);
    border-color: #3b82f6;
    color: #3b82f6;
  }

  .status-badge.success {
    background: rgba(16, 185, 129, 0.3);
    border-color: #10b981;
    color: #10b981;
  }

  .status-badge.error {
    background: rgba(239, 68, 68, 0.3);
    border-color: #ef4444;
    color: #ef4444;
  }

  .rendering {
    background: rgba(59, 130, 246, 0.9);
  }

  .success {
    background: rgba(22, 163, 74, 0.9);
  }

  .error {
    background: rgba(239, 68, 68, 0.9);
  }

  .skipped {
    background: rgba(156, 163, 175, 0.9);
  }
</style>
