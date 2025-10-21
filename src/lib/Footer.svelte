<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { appState } from './state/state.svelte';
  import { formatBytes } from './utils/format';

  let statusMessage = 'Ready';
  let lastActivity = '';

  listen<number>('buffering-progress', e => {
    if (e.payload < 100) {
      statusMessage = `Buffering... ${e.payload.toFixed(1)}%`;
    } else {
      statusMessage = 'Ready';
    }
  });

  listen<number>('timeline-progress', e => {
    if ($appState.playingCombined) {
      statusMessage = 'Playing';
      lastActivity = new Date().toLocaleTimeString();
    }
  });

  // Watch for app state changes
  $: {
    if ($appState.playingCombined) {
      statusMessage = 'Playing';
    } else if ($appState.isCombiningFile) {
      statusMessage = 'Processing audio...';
    } else if ($appState.sections.length === 0) {
      statusMessage = 'No files loaded';
    } else if (!$appState.playingCombined && statusMessage === 'Playing') {
      statusMessage = 'Paused';
    }
  }

  // Calculate total files and size
  $: totalFiles = $appState.sections.length;
  $: totalSize = $appState.sections.reduce((sum, section) => sum + (section.size || 0), 0);
</script>

<footer class="status-footer d-flex justify-content-between align-items-center px-3 py-1">
  <div class="d-flex gap-4 align-items-center">
    <!-- Status Message -->
    <div class="status-item">
      <span class="status-icon" class:active={$appState.playingCombined}>●</span>
      <span class="status-text">{statusMessage}</span>
    </div>

    <!-- File Count -->
    {#if totalFiles > 0}
      <div class="status-item">
        <span class="status-label">{totalFiles}</span>
        <span class="status-text">{totalFiles === 1 ? 'file' : 'files'}</span>
      </div>
    {/if}

    <!-- Total Size -->
    {#if totalSize > 0}
      <div class="status-item">
        <span class="status-label">{formatBytes(totalSize)}</span>
        <span class="status-text">total</span>
      </div>
    {/if}
  </div>

  <div class="d-flex gap-4 align-items-center">
    <!-- Last Activity -->
    {#if lastActivity}
      <div class="status-item">
        <span class="status-text">Last: {lastActivity}</span>
      </div>
    {/if}

    <!-- Combined File Length -->
    {#if $appState.combinedFileLength}
      <div class="status-item">
        <span class="status-label">Duration:</span>
        <span class="status-text">{($appState.combinedFileLength / 1000).toFixed(2)}s</span>
      </div>
    {/if}
  </div>
</footer>

<style>
  .status-footer {
    background: linear-gradient(to bottom, #1a202c, #2d3748);
    border-top: 1px solid #4a5568;
    font-size: 11px;
    color: #a0aec0;
    min-height: 24px;
    flex-shrink: 0;
  }

  .status-item {
    display: flex;
    gap: 4px;
    align-items: center;
  }

  .status-icon {
    color: #68d391;
    font-size: 8px;
    transition: color 0.2s ease;
  }

  .status-icon.active {
    color: #38a169;
    animation: pulse 2s infinite;
  }

  .status-text {
    color: #cbd5e0;
  }

  .status-label {
    color: #e2e8f0;
    font-weight: 600;
    font-family: 'Courier New', monospace;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.6;
    }
  }
</style>
