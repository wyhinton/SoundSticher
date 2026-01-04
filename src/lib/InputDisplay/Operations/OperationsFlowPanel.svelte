<script lang="ts">
  import { SvelteFlow, Controls, Background } from '@xyflow/svelte';
  import type { Node, Edge, NodeTypes } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';

  import { appState, setSelectedOperationName } from '$lib/state/state.svelte';
  import {
    type OperationDef,
    type OperationsState,
    OperationInfoDictionary,
    addOperation,
    deleteOperation,
    deleteAllOperations,
    addTestOperations,
  } from '$lib/state/operation';

  import OperationNode from './OperationNode.svelte';
  import SourceNode from './SourceNode.svelte';
  import OutputNode from './OutputNode.svelte';
  import CombinedFlow from './CombinedFlow.svelte';

  // Custom node types
  const nodeTypes: NodeTypes = {
    operation: OperationNode,
    source: SourceNode,
    output: OutputNode,
  };

  // Panel visibility
  export let isExpanded = true;

  // Use selected operation from global state
  $: selectedOperationName = $appState.uiSettings?.selectedOperationName || null;

  // Panel height management
  let panelHeight = 300; // default height in pixels
  let isResizing = false;
  let resizeStartY = 0;
  let resizeStartHeight = 0;

  // Flow state
  let nodes: Node[] = [];
  let edges: Edge[] = [];

  // Combined flows state
  let combineOperations: Array<{
    name: string;
    operation: any;
  }> = [];

  // Generate flow diagram from operations state
  function generateFlowFromOperations(operations: OperationsState | undefined): {
    nodes: Node[];
    edges: Edge[];
  } {
    const generatedNodes: Node[] = [];
    const generatedEdges: Edge[] = [];
    const nodePositions = new Map<string, { x: number; y: number }>();
    const foundCombineOps: Array<{
      name: string;
      operation: any;
    }> = [];

    // Layout configuration
    const HORIZONTAL_SPACING = 280;
    const VERTICAL_SPACING = 60;
    const START_X = 50;
    const START_Y = 50;

    // Create source nodes for timeline items
    const timelineItems = $appState.timelineItems;
    let currentY = START_Y;

    timelineItems.forEach((item, index) => {
      if (item.type === 'audio-file') {
        const sourceNodeId = `timeline-source-${item.id}`;

        generatedNodes.push({
          id: sourceNodeId,
          type: 'source',
          position: { x: START_X, y: currentY },
          data: {
            source: { type: 'timeline-item', itemId: item.id },
            label: item.fileName || `Timeline Item ${index + 1}`,
          },
          draggable: false,
          selectable: false,
        });

        nodePositions.set(sourceNodeId, { x: START_X, y: currentY });
        currentY += VERTICAL_SPACING;
      }
    });

    // If no operations are defined, just return the timeline source nodes
    if (!operations?.defs || Object.keys(operations.defs).length === 0) {
      combineOperations = [];
      return { nodes: generatedNodes, edges: generatedEdges };
    }

    // Group operations by their source type for layout
    const operationEntries = Object.entries(operations.defs);

    // First pass: create source nodes and operation nodes
    const sourceNodes = new Map<string, string>(); // source key -> node id

    operationEntries.forEach(([name, def], index) => {
      const sourceKey = getSourceKey(def.source);
      const opInfo = OperationInfoDictionary[def.kind];

      if (!opInfo) return; // Skip if operation info not found

      // Create source node if not exists
      if (!sourceNodes.has(sourceKey)) {
        const sourceNodeId = `source-${sourceKey}`;
        const sourceY = START_Y + sourceNodes.size * VERTICAL_SPACING;

        generatedNodes.push({
          id: sourceNodeId,
          type: 'source',
          position: { x: START_X, y: sourceY },
          data: {
            source: def.source,
            label: getSourceLabel(def.source),
          },
          draggable: false,
          selectable: false,
        });

        nodePositions.set(sourceNodeId, { x: START_X, y: sourceY });
        sourceNodes.set(sourceKey, sourceNodeId);
      }

      // Create operation node
      const opNodeId = `op-${name}`;
      const sourceNodeId = sourceNodes.get(sourceKey)!;
      const sourcePos = nodePositions.get(sourceNodeId)!;

      // Calculate position based on how many ops share this source
      const opsWithSameSource = operationEntries.filter(
        ([, d]) => getSourceKey(d.source) === sourceKey
      );
      const opIndex = opsWithSameSource.findIndex(([n]) => n === name);

      const opX = START_X + HORIZONTAL_SPACING;
      const opY = sourcePos.y + opIndex * (VERTICAL_SPACING * 0.8);

      generatedNodes.push({
        id: opNodeId,
        type: 'operation',
        position: { x: opX, y: opY },
        data: {
          name,
          kind: def.kind,
          icon: opInfo.icon,
          label: opInfo.label,
          category: opInfo.category,
          def,
        },
        draggable: false,
        selectable: true,
      });

      nodePositions.set(opNodeId, { x: opX, y: opY });

      // Track combine operations for separate rendering
      if (def.kind === 'combine') {
        foundCombineOps.push({
          name,
          operation: def,
        });
      }

      // Create edge from source to operation
      generatedEdges.push({
        id: `edge-${sourceNodeId}-${opNodeId}`,
        source: sourceNodeId,
        target: opNodeId,
        type: 'smoothstep',
        animated: false,
        style: 'stroke: #64748b; stroke-width: 2px;',
      });

      // For render operations, create output node
      if (opInfo.category === 'render') {
        const outputNodeId = `output-${name}`;
        const outputX = opX + HORIZONTAL_SPACING;

        generatedNodes.push({
          id: outputNodeId,
          type: 'output',
          position: { x: outputX, y: opY },
          data: {
            operationName: name,
            format: (def as any).format || 'wav',
          },
          draggable: false,
          selectable: false,
        });

        generatedEdges.push({
          id: `edge-${opNodeId}-${outputNodeId}`,
          source: opNodeId,
          target: outputNodeId,
          type: 'smoothstep',
          animated: false,
          style: 'stroke: #22c55e; stroke-width: 2px;',
        });
      }
    });

    // Handle pipeline operations - connect them
    operationEntries.forEach(([name, def]) => {
      if (def.kind === 'pipeline') {
        const pipelineOps = def.operations;
        for (let i = 0; i < pipelineOps.length - 1; i++) {
          const fromOp = pipelineOps[i];
          const toOp = pipelineOps[i + 1];

          generatedEdges.push({
            id: `pipeline-edge-${fromOp}-${toOp}`,
            source: `op-${fromOp}`,
            target: `op-${toOp}`,
            type: 'smoothstep',
            animated: true,
            style: 'stroke: #8b5cf6; stroke-width: 2px;',
            label: 'pipeline',
          });
        }
      }
    });

    // Update combine operations state
    combineOperations = foundCombineOps;

    return { nodes: generatedNodes, edges: generatedEdges };
  }

  function getSourceKey(
    source: OperationDef['source'] | { type: 'timeline-item'; itemId: string }
  ): string {
    switch (source.type) {
      case 'group':
        return `group:${source.groupRef}`;
      case 'files':
        return `files:${source.fileIds.sort().join(',')}`;
      case 'all':
        return 'all';
      case 'active':
        return 'active';
      case 'section':
        return `section:${source.sectionIndex}`;
      case 'previousOperation':
        return `prev:${source.operationRef}`;
      case 'timeline-item':
        return `timeline:${(source as any).itemId}`;
      default:
        return 'unknown';
    }
  }

  function getSourceLabel(
    source: OperationDef['source'] | { type: 'timeline-item'; itemId: string }
  ): string {
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
      case 'timeline-item':
        return `Timeline: ${(source as any).itemId}`;
      default:
        return 'Unknown Source';
    }
  }

  // Stats
  function getOperationStats() {
    const ops = $appState.operations;
    if (!ops) return { total: 0, render: 0, edit: 0, meta: 0 };

    const defs = Object.values(ops.defs);
    return {
      total: defs.length,
      render: defs.filter(d => OperationInfoDictionary[d.kind]?.category === 'render').length,
      edit: defs.filter(d => OperationInfoDictionary[d.kind]?.category === 'edit').length,
      meta: defs.filter(d => OperationInfoDictionary[d.kind]?.category === 'meta').length,
    };
  }

  // Add combine operation
  function addCombineOperation() {
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    const operationName = `combine_${timestamp}`;

    addOperation(operationName, {
      kind: 'combine',
      source: { type: 'active' },
      outputPath: `output/combined_${timestamp}.wav`,
      gapSeconds: 0,
      format: 'wav',
    });
  }

  // Handle operation selection
  function handleOperationSelect(event: CustomEvent<{ operationName: string }>) {
    setSelectedOperationName(event.detail.operationName);
  }

  // Resize functionality
  function startResize(event: MouseEvent) {
    isResizing = true;
    resizeStartY = event.clientY;
    resizeStartHeight = panelHeight;

    // Prevent text selection during resize
    document.body.style.userSelect = 'none';

    // Add global event listeners
    document.addEventListener('mousemove', handleResize);
    document.addEventListener('mouseup', stopResize);

    event.preventDefault();
  }

  function handleResize(event: MouseEvent) {
    if (!isResizing) return;

    const deltaY = event.clientY - resizeStartY;
    const newHeight = Math.max(150, Math.min(800, resizeStartHeight + deltaY)); // Min 150px, max 800px
    panelHeight = newHeight;
  }

  function stopResize() {
    isResizing = false;
    document.body.style.userSelect = '';

    // Remove global event listeners
    document.removeEventListener('mousemove', handleResize);
    document.removeEventListener('mouseup', stopResize);
  }

  // Reactive updates
  $: {
    const flow = generateFlowFromOperations($appState.operations);
    nodes = flow.nodes;
    edges = flow.edges;
  }

  $: stats = getOperationStats();
</script>

<div class="operations-flow-panel" class:collapsed={!isExpanded}>
  <div
    class="panel-header"
    style="--header-bg: {$appState.uiSettings?.theme?.panelHeaderBackgroundColor}"
  >
    <div class="header-left">
      <button
        class="toggle-btn"
        onclick={() => (isExpanded = !isExpanded)}
        title={isExpanded ? 'Collapse panel' : 'Expand panel'}
        aria-label={isExpanded ? 'Collapse panel' : 'Expand panel'}
      >
        <i class="fa fa-{isExpanded ? 'chevron-down' : 'chevron-right'}"></i>
      </button>
      <span class="panel-title">
        <i class="fa fa-project-diagram"></i>
        Operations
      </span>
      <!-- <div class="stats-badges">
        <span class="badge badge-total" title="Total operations">{stats.total}</span>
        {#if stats.render > 0}
          <span class="badge badge-render" title="Render operations">🔗 {stats.render}</span>
        {/if}
        {#if stats.edit > 0}
          <span class="badge badge-edit" title="Edit operations">✂️ {stats.edit}</span>
        {/if}
        {#if stats.meta > 0}
          <span class="badge badge-meta" title="Meta operations">🔀 {stats.meta}</span>
        {/if}
      </div> -->
    </div>
    <div class="header-actions">
      <button
        class="btn btn-xs btn-outline-primary"
        onclick={addTestOperations}
        title="Add test operations"
        aria-label="Add test operations"
      >
        <i class="fa fa-flask"></i>
      </button>
      <button
        class="btn btn-xs btn-outline-danger"
        onclick={() => {
          if (confirm('Delete all operations?')) deleteAllOperations();
        }}
        title="Delete all operations"
        aria-label="Delete all operations"
      >
        <i class="fa fa-trash"></i>
      </button>
    </div>
  </div>

  {#if isExpanded}
    <div class="panel-content" style="height: {panelHeight}px;">
      <div class="flow-container">
        {#if $appState.operations?.defs && Object.keys($appState.operations.defs).some(name => $appState.operations?.defs[name].kind === 'combine')}
          <!-- Show individual CombinedFlow components for each combine operation -->
          <div class="combined-flows-row h-100 d-flex">
            {#each combineOperations as combineOp (combineOp.name)}
              <CombinedFlow 
                operation={combineOp.operation} 
                operationName={combineOp.name}
                isSelected={selectedOperationName === combineOp.name}
                on:operationSelect={handleOperationSelect}
              />
            {/each}
          </div>
        {:else if nodes.length === 0}
          <div class="empty-state">
            <i class="fa fa-project-diagram fa-3x"></i>
            <p>No operations defined</p>
            <button class="btn btn-sm btn-primary" onclick={addTestOperations}>
              <i class="fa fa-plus"></i> Add Test Operations
            </button>
          </div>
        {:else}
          <!-- Show main flow diagram for non-combine operations -->
          <SvelteFlow
            {nodes}
            {edges}
            {nodeTypes}
            fitView
            minZoom={0.1}
            maxZoom={2}
            panOnScroll
            zoomOnScroll
            preventScrolling={false}
            nodesDraggable={false}
            nodesConnectable={false}
            elementsSelectable={true}
          >
            <Background gap={0} size={0} />
            <Controls />
          </SvelteFlow>
        {/if}
      </div>

      <div class="operation-creation-panel">
        <div
          class="creation-header"
          style="--header-bg: {$appState.uiSettings?.theme?.panelHeaderBackgroundColor}"
        >
          <h4>Add Operations</h4>
        </div>
        <div class="operation-buttons">
          <button
            class="operation-add-btn"
            onclick={addCombineOperation}
            title="Add combine operation"
          >
            <span class="operation-icon">🔗</span>
            <span class="operation-label">Combine</span>
            <i class="fa fa-plus"></i>
          </button>
        </div>
      </div>
    </div>

    <!-- Resize handle -->
    <div
      class="resize-handle"
      class:resizing={isResizing}
      onmousedown={startResize}
      title="Drag to resize panel height"
      role="separator"
      aria-label="Resize panel height"
      tabindex="0"
    >
      <div class="resize-indicator">
        <i class="fa fa-grip-lines"></i>
      </div>
    </div>
  {/if}
</div>

<style>
  .operations-flow-panel {
    background: var(--panel-bg, #1e1e2e);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    width: 100%;
  }

  .operations-flow-panel.collapsed {
    height: auto;
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 4px 8px;
    /* background: var(--header-bg); */
    background-color: #161616;
    border-bottom: 1px solid var(--border-color, #313244);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .toggle-btn {
    background: transparent;
    border: none;
    color: var(--text-muted, #a6adc8);
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 3px;
    transition: background 0.2s;
    font-size: 0.75rem;
  }

  .toggle-btn:hover {
    background: var(--hover-bg, #313244);
  }

  .panel-title {
    font-weight: 600;
    color: var(--text-primary, #cdd6f4);
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 0.8rem;
  }

  .stats-badges {
    display: flex;
    gap: 3px;
    margin-left: 6px;
  }

  .badge {
    font-size: 0.65rem;
    padding: 1px 4px;
    border-radius: 3px;
    font-weight: 500;
    line-height: 1.2;
  }

  .badge-total {
    background: var(--badge-bg, #45475a);
    color: var(--text-primary, #cdd6f4);
  }

  .badge-render {
    background: rgba(245, 158, 11, 0.2);
    color: #f59e0b;
  }

  .badge-edit {
    background: rgba(139, 92, 246, 0.2);
    color: #8b5cf6;
  }

  .badge-meta {
    background: rgba(34, 197, 94, 0.2);
    color: #22c55e;
  }

  .header-actions {
    display: flex;
    gap: 2px;
  }

  .btn-xs {
    font-size: 0.65rem;
    padding: 2px 4px;
    line-height: 1.2;
  }

  .panel-content {
    display: flex;
    width: 100%;
    /* height is now set via inline style */
  }

  .resize-handle {
    height: 8px;
    background: var(--panel-bg, #1e1e2e);
    border-top: 1px solid var(--border-color, #313244);
    cursor: row-resize;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.2s;
    user-select: none;
  }

  .resize-handle:hover {
    background: var(--hover-bg, #313244);
  }

  .resize-handle.resizing {
    background: var(--hover-bg, #313244);
  }

  .resize-indicator {
    color: var(--text-muted, #a6adc8);
    font-size: 0.7rem;
    opacity: 0.6;
    transition: opacity 0.2s;
  }

  .resize-handle:hover .resize-indicator {
    opacity: 1;
  }

  .flow-container {
    flex: 1;
    position: relative;
    border-right: 1px solid var(--border-color, #313244);
  }

  .combined-flows-row {
    height: 100%;
    overflow-x: auto;
    overflow-y: hidden;
  }

  .operation-creation-panel {
    width: 200px;
    background: var(--panel-bg, #1e1e2e);
    border-left: 1px solid var(--border-color, #313244);
    display: flex;
    flex-direction: column;
  }

  .creation-header {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-color, #313244);
    background: var(--header-bg, #181825);
  }

  .creation-header h4 {
    margin: 0;
    font-size: 0.8rem;
    color: var(--text-primary, #cdd6f4);
  }

  .operation-buttons {
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .operation-add-btn {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
    background: var(--panel-bg, #1e1e2e);
    border: 1px solid var(--border-color, #313244);
    border-radius: 6px;
    color: var(--text-primary, #cdd6f4);
    cursor: pointer;
    transition: all 0.2s;
    font-size: 0.75rem;
    min-height: 36px;
  }

  .operation-add-btn:hover {
    background: var(--hover-bg, #313244);
    border-color: #3b82f6;
    transform: translateY(-1px);
  }

  .operation-add-btn:active {
    transform: translateY(0);
  }

  .operation-icon {
    font-size: 1rem;
  }

  .operation-label {
    flex: 1;
    text-align: left;
    margin-left: 8px;
    font-weight: 500;
  }

  .operation-add-btn .fa-plus {
    color: #22c55e;
    font-size: 0.7rem;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted, #a6adc8);
    gap: 12px;
    padding: 24px;
  }

  .empty-state p {
    margin: 0;
    font-size: 0.9rem;
  }

  /* SvelteFlow style overrides */
  :global(.svelte-flow) {
    background: #000000 !important;
  }

  :global(.svelte-flow__background) {
    background: #000000 !important;
  }

  :global(.svelte-flow__controls) {
    background: var(--panel-bg, #1e1e2e) !important;
    border: 1px solid var(--border-color, #313244) !important;
    border-radius: 4px !important;
  }

  :global(.svelte-flow__controls-button) {
    background: var(--panel-bg, #1e1e2e) !important;
    border-color: var(--border-color, #313244) !important;
    color: var(--text-muted, #a6adc8) !important;
  }

  :global(.svelte-flow__controls-button:hover) {
    background: var(--hover-bg, #313244) !important;
  }
</style>
