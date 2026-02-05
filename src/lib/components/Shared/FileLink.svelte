<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  // Props
  export let filePath: string | null = null;
  export let showIcon: boolean = true;

  // Function to open file in explorer
  async function openInExplorer() {
    if (!filePath) return;
    try {
      await invoke('open_in_explorer', { fileToOpen: filePath });
    } catch (error) {
      console.error('Failed to open file in explorer:', error);
    }
  }

  // Helper function to get the filename from a path
  function getFileName(path: string): string {
    const parts = path.split(/[/\\]/);
    return parts[parts.length - 1] || path;
  }
</script>

{#if filePath}
  <button class="file-link" title={filePath} on:click={openInExplorer}>
    {#if showIcon}
      <i class="fas fa-folder-open"></i>
    {/if}
    {getFileName(filePath)}
  </button>
{:else}
  <span class="no-file">—</span>
{/if}

<style>
  .file-link {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    background: none;
    border: none;
    padding: 2px 6px;
    border-radius: 3px;
    font-family: 'Fira Code', monospace;
    font-size: 10px;
    color: #7ca3d4;
    cursor: pointer;
    transition: all 0.2s;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-link:hover {
    background-color: rgba(76, 120, 168, 0.2);
    color: #a8c8e8;
  }

  .file-link i {
    flex-shrink: 0;
  }

  .no-file {
    color: #5d5d5d;
    font-style: italic;
  }
</style>
