<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { derived } from 'svelte/store';
  import { TAURI_COMMANDS, type CommandInfo } from '$lib/generated/tauri_commands';
  import { performanceStore, resetPerformance } from '$lib/state/performance';

  const sortedPerformance = derived(performanceStore, $store => {
    return Object.entries($store).sort((a, b) => {
      const lastA = a[1][a[1].length - 1] ?? 0;
      const lastB = b[1][b[1].length - 1] ?? 0;
      return lastB.timeStamp - lastA.timeStamp;
    });
  });

  function handleReset() {
    resetPerformance();
  }

  function getCommandInfo(commandName: string): CommandInfo | undefined {
    return TAURI_COMMANDS[commandName];
  }

  async function openCommandSource(commandName: string) {
    const info = getCommandInfo(commandName);
    if (!info) {
      console.warn(`Command info not found for: ${commandName}`);
      return;
    }

    try {
      await invoke('open_file_in_editor', {
        filePath: info.file_path,
        lineNumber: info.line_number,
      });
    } catch (error) {
      console.error('Failed to open file:', error);
    }
  }
</script>

{#snippet actionButton(
  onClick: () => void,
  icon: string,
  text: string,
  disabled: boolean = false,
  variant:
    | 'default'
    | 'primary'
    | 'secondary'
    | 'danger'
    | 'warning'
    | 'success'
    | 'info' = 'default'
)}
  <button on:click={onClick} class="btn btn-sm btn-{variant}" {disabled}>
    <i class="me-1 fa {icon}"></i>{text}
  </button>
{/snippet}

<div>
  <div class="d-flex justify-content-between">
    <h3>Performance Metrics</h3>
    <div class="performance-controls">
      {@render actionButton(() => handleReset(), 'fa-trash', 'Reset Performance')}
    </div>
  </div>
  <table class="performance-table">
    <thead>
      <tr>
        <th style:min-width="150px">Metric</th>
        <th>Time (ms)</th>
        <th>Count</th>
      </tr>
    </thead>
    <tbody>
      {#each $sortedPerformance as [key, value]}
        <tr>
          <td>
            {#if getCommandInfo(key)}
              <button
                class="metric-link"
                on:click={() => openCommandSource(key)}
                title="Click to open {key} source in VS Code"
              >
                <b>{key}</b>
              </button>
            {:else}
              <b>{key}</b>
            {/if}
          </td>
          {#if value.length > 0}
            <td class="text-center">{value[value.length - 1].time.toFixed(2)}</td>
          {/if}
          <td class="text-center">{value.length}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  /* Performance Section */
  .performance-controls {
    margin-bottom: 16px;
  }

  .performance-table {
    width: 100%;
    background-color: rgba(255, 255, 255, 0.05);
    border-radius: 6px;
    overflow: hidden;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .performance-table th {
    background-color: rgba(245, 158, 11, 0.2);
    color: #f59e0b;
    padding: 12px;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .performance-table td {
    padding: 8px 12px;
    font-size: 11px;
    color: rgba(255, 255, 255, 0.8);
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }

  .performance-table tr:last-child td {
    border-bottom: none;
  }

  /* Metric Link Styles */
  .metric-link {
    background: none;
    border: none;
    color: #60a5fa;
    cursor: pointer;
    padding: 0;
    margin: 0;
    font-weight: 700;
    text-decoration: underline;
    transition: all 0.2s ease;
    font-size: inherit;
    font-family: inherit;
  }

  .metric-link:hover {
    color: #93c5fd;
    text-decoration-color: #93c5fd;
  }

  .metric-link:active {
    color: #3b82f6;
  }

  /* Button Styles */
  .btn {
    border: 1px solid rgba(255, 255, 255, 0.3) !important;
    background-color: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.9);
    transition: all 0.2s ease;
    font-size: 12px;
    padding: 6px 12px;
    border-radius: 4px;
    width: min-content;
    white-space: nowrap;
  }

  /* Button Color Variants */
  .btn-primary {
    background-color: rgba(59, 130, 246, 0.2);
    border-color: rgba(59, 130, 246, 0.5) !important;
    color: #60a5fa;
  }

  .btn-primary:hover {
    background-color: rgba(59, 130, 246, 0.3);
    border-color: rgba(59, 130, 246, 0.7) !important;
  }

  .btn-success {
    background-color: rgba(34, 197, 94, 0.2);
    border-color: rgba(34, 197, 94, 0.5) !important;
    color: #4ade80;
  }

  .btn-success:hover {
    background-color: rgba(34, 197, 94, 0.3);
    border-color: rgba(34, 197, 94, 0.7) !important;
  }

  .btn-warning {
    background-color: rgba(245, 158, 11, 0.2);
    border-color: rgba(245, 158, 11, 0.5) !important;
    color: #fbbf24;
  }

  .btn-warning:hover {
    background-color: rgba(245, 158, 11, 0.3);
    border-color: rgba(245, 158, 11, 0.7) !important;
  }

  .btn-danger {
    background-color: rgba(239, 68, 68, 0.2);
    border-color: rgba(239, 68, 68, 0.5) !important;
    color: #f87171;
  }

  .btn-danger:hover {
    background-color: rgba(239, 68, 68, 0.3);
    border-color: rgba(239, 68, 68, 0.7) !important;
  }

  .btn-info {
    background-color: rgba(14, 165, 233, 0.2);
    border-color: rgba(14, 165, 233, 0.5) !important;
    color: #38bdf8;
  }

  .btn-info:hover {
    background-color: rgba(14, 165, 233, 0.3);
    border-color: rgba(14, 165, 233, 0.7) !important;
  }

  .btn-secondary {
    background-color: rgba(107, 114, 128, 0.2);
    border-color: rgba(107, 114, 128, 0.5) !important;
    color: #9ca3af;
  }

  .btn-secondary:hover {
    background-color: rgba(107, 114, 128, 0.3);
    border-color: rgba(107, 114, 128, 0.7) !important;
  }

  .btn:hover {
    background-color: rgba(255, 255, 255, 0.2);
    border-color: rgba(255, 255, 255, 0.5) !important;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
