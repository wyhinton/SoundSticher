<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import type { OperationSource } from '$lib/state/operation';

  interface SourceNodeData {
    source: OperationSource;
    label: string;
  }

  export let data: SourceNodeData;

  function getSourceIcon(type: OperationSource['type']): string {
    switch (type) {
      case 'group':
        return '📁';
      case 'files':
        return '📄';
      case 'all':
        return '🗂️';
      case 'active':
        return '✅';
      case 'section':
        return '📂';
      case 'previousOperation':
        return '⬅️';
    }
  }

  $: icon = getSourceIcon(data.source.type);
</script>

<div class="source-node">
  <Handle type="source" position={Position.Right} />
</div>

<style>
  .source-node {
    width: 40px;
    height: 40px;
    background: #3b82f6;
    border: 2px solid #60a5fa;
    border-radius: 50%;
    position: relative;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.3);
  }

  :global(.source-node .svelte-flow__handle) {
    width: 10px;
    height: 10px;
    background: #3b82f6;
    border: 2px solid #60a5fa;
  }
</style>
