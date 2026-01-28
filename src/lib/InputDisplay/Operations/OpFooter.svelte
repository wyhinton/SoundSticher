<script lang="ts">
  import { createOperationRenderStore } from '$lib/state/autoRender';

  export let operationId: string;
  const operationRenderState = createOperationRenderStore(operationId);
</script>

<div class="footer-render-overlay">
  <div class="footer-content">
    <div class="footer-section">
      <span class="footer-label">Status:</span>
      {#if $operationRenderState}
        <span
          class="footer-badge"
          class:rendering={$operationRenderState.status === 'rendering'}
          class:success={$operationRenderState.status === 'success'}
          class:error={$operationRenderState.status === 'error'}
          class:skipped={$operationRenderState.status === 'skipped'}
          class:pending={$operationRenderState.status === 'pending'}
        >
          {#if $operationRenderState.status === 'rendering'}
            ⏳ Rendering
          {:else if $operationRenderState.status === 'success'}
            ✅ Success
          {:else if $operationRenderState.status === 'error'}
            ❌ Error
          {:else if $operationRenderState.status === 'skipped'}
            ⏭️ Skipped
          {:else}
            ⏸️ Pending
          {/if}
        </span>
      {:else}
        ⏸️ Pending
      {/if}
    </div>

    <div class="footer-divider"></div>
    {#if $operationRenderState}
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

      {#if $operationRenderState.error}
        <div class="footer-divider"></div>
        <div class="footer-section footer-error">
          <span class="footer-label">Error:</span>
          <span class="footer-value error-text">{$operationRenderState.error}</span>
        </div>
      {/if}

      {#if $operationRenderState.startedAt && $operationRenderState.status === 'rendering'}
        <div class="footer-divider"></div>
        <div class="footer-section">
          <span class="footer-label">Started:</span>
          <span class="footer-value"
            >{new Date($operationRenderState.startedAt).toLocaleTimeString()}</span
          >
        </div>
      {/if}
      <!-- content here -->
    {/if}
  </div>
</div>

<style>
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
</style>
