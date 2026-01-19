<script lang="ts">
  import { activeStatus } from './state/status';
  import { appState } from './state/state.svelte';
  import { formatBytes } from './utils/format';
  import { invokeWithPerf } from './state/performance';

  let lastActivity = '';

  // Subscribe to the active status - activeStatus always returns a value, never undefined
  $: status = $activeStatus;

  // Note: File and size calculations removed - operations no longer have sections
  $: totalFiles = 0;
  $: totalSize = 0;

  const openInExplorer = async (filePath: string) => {
    try {
      console.log('📁 Opening file in explorer from footer:', filePath);
      await invokeWithPerf('open_in_explorer', {
        fileToOpen: filePath,
      });
    } catch (error) {
      console.error('❌ Failed to open in explorer from footer:', error);
    }
  };
</script>

<footer class="status-footer d-flex justify-content-between align-items-center px-3 py-1">
  <div class="d-flex gap-4 align-items-center" style="margin-bottom: -3px">
    <!-- Status Message -->
    <div class="status-item">
      <span class="status-icon {status.level}">●</span>
      <span class="status-text">{status.message}</span>

      {#if status.progress !== undefined}
        <div class="export-progress-bar">
          <div class="export-progress-fill" style="width: {status.progress * 100}%"></div>
        </div>
        <span class="status-text">({(status.progress * 100).toFixed(1)}%)</span>
      {/if}

      {#if status.level === 'success' && status.source === 'export'}
        <span class="status-text completed-check">✓</span>
      {/if}
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

  /* Status level colors */
  .status-icon.idle {
    color: #68d391;
  }

  .status-icon.info {
    color: #63b3ed;
  }

  .status-icon.working {
    color: #f6ad55;
    animation: pulse 1.5s infinite;
  }

  .status-icon.success {
    color: #68d391;
    animation: successPulse 1s ease-in-out 3;
  }

  .status-icon.warning {
    color: #f6ad55;
  }

  .status-icon.error {
    color: #fc8181;
    animation: errorPulse 0.5s ease-in-out 5;
  }

  .status-icon.active {
    color: #38a169;
    animation: pulse 2s infinite;
  }

  .status-icon.exporting {
    color: #f6ad55;
    animation: exportPulse 1.5s infinite;
  }

  .status-icon.completed {
    color: #68d391;
    animation: completedPulse 1s ease-in-out 3;
  }

  .completed-check {
    color: #68d391 !important;
    font-weight: bold;
    font-size: 12px;
  }

  .footer-file-path {
    background: linear-gradient(135deg, #2d3748, #1a202c);
    border: 1px solid #4a5568;
    border-radius: 3px;
    color: #68d391;
    padding: 2px 6px;
    font-size: 10px;
    font-family: 'Courier New', monospace;
    cursor: pointer;
    transition: all 0.15s ease;
    margin-left: 8px;
    display: inline-block;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    user-select: text;
  }

  .footer-file-path:hover {
    background: linear-gradient(135deg, #68d391, #4fd1c7);
    color: #1a202c;
    border-color: #9ae6b4;
    transform: translateY(-1px);
  }

  .footer-file-path:active {
    transform: translateY(0);
  }

  .footer-file-path:focus {
    outline: 1px solid #68d391;
    outline-offset: 1px;
  }

  .export-progress-bar {
    width: 60px;
    height: 8px;
    background-color: #2d3748;
    border-radius: 4px;
    overflow: hidden;
    border: 1px solid #4a5568;
  }

  .export-progress-fill {
    height: 100%;
    background: linear-gradient(90deg, #f6ad55, #ed8936);
    transition: width 0.2s ease;
    border-radius: 3px;
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

  @keyframes exportPulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.4;
    }
  }

  @keyframes completedPulse {
    0%,
    100% {
      opacity: 1;
      transform: scale(1);
    }
    50% {
      opacity: 0.8;
      transform: scale(1.1);
    }
  }

  @keyframes successPulse {
    0%,
    100% {
      opacity: 1;
      transform: scale(1);
    }
    50% {
      opacity: 0.8;
      transform: scale(1.1);
    }
  }

  @keyframes errorPulse {
    0%,
    100% {
      opacity: 1;
      transform: scale(1);
    }
    50% {
      opacity: 0.5;
      transform: scale(1.15);
    }
  }
</style>
