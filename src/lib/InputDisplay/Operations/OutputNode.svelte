<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import type { AudioFormat } from '$lib/state/operation';

  interface OutputNodeData {
    operationName: string;
    format: AudioFormat;
  }

  export let data: OutputNodeData;

  function getFormatIcon(format: AudioFormat): string {
    switch (format) {
      case 'wav':
        return '🎵';
      case 'mp3':
        return '🎧';
      case 'flac':
        return '💿';
      case 'ogg':
        return '🔊';
      case 'aiff':
        return '🎼';
      default:
        return '📁';
    }
  }

  $: icon = getFormatIcon(data.format);
</script>

<div class="output-node">
  <Handle type="target" position={Position.Left} />

  <div class="node-content">
    <span class="node-icon">{icon}</span>
    <div class="node-info">
      <span class="output-label">Output</span>
      <span class="format-badge">.{data.format}</span>
    </div>
  </div>
</div>

<style>
  .output-node {
    background: linear-gradient(135deg, #14532d 0%, #1e1e2e 100%);
    border: 2px solid #22c55e;
    border-radius: 8px;
    padding: 10px 14px;
    min-width: 100px;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.3);
  }

  .node-content {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .node-icon {
    font-size: 1.2rem;
  }

  .node-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .output-label {
    font-weight: 500;
    color: #86efac;
    font-size: 0.8rem;
  }

  .format-badge {
    font-size: 0.7rem;
    color: #4ade80;
    font-family: 'Fira Code', monospace;
    background: rgba(34, 197, 94, 0.2);
    padding: 1px 4px;
    border-radius: 3px;
  }

  :global(.output-node .svelte-flow__handle) {
    width: 10px;
    height: 10px;
    background: #22c55e;
    border: 2px solid #4ade80;
  }
</style>
