<script lang="ts">
  import { formatBytes } from '../utils/format';
  import { invoke } from '@tauri-apps/api/core';

  // Props
  export let operationId: string | undefined = undefined;

  // Interface for artifact records
  interface ArtifactRecordForFrontend {
    id: string;
    creator_op_id: string;
    frontend_op_id: string | null; // Frontend operation ID for filtering/display
    created_at: number;
    artifact_type: string;
    size_bytes: number;
    exists: boolean;
    tags: Record<string, string>;
    file_paths: string[];
  }

  // Store for artifacts
  let artifacts: ArtifactRecordForFrontend[] = [];
  let artifactsLoading = false;

  // Function to fetch artifacts for the operation
  async function fetchArtifacts() {
    if (!operationId) {
      artifacts = [];
      return;
    }

    artifactsLoading = true;
    try {
      // Use get_filtered_artifacts from artifacts_service.rs with operation_id filter
      const result: ArtifactRecordForFrontend[] = await invoke('get_filtered_artifacts', {
        filter: {
          operation_id: operationId,
        },
      });
      artifacts = result;
      console.log(
        `[OperationArtifactsTable] Fetched ${result.length} artifacts for operation: ${operationId}`
      );
    } catch (error) {
      console.error('Failed to fetch artifacts:', error);
      artifacts = [];
    } finally {
      artifactsLoading = false;
    }
  }

  // Helper function to format timestamp
  function formatTimestamp(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleString();
  }

  // Reactive statement to fetch artifacts when operationId changes
  $: if (operationId) {
    fetchArtifacts();
  }
</script>

{#if operationId}
  <div class="artifacts-section">
    <h6 class="artifacts-title">Operation Artifacts</h6>
    {#if artifactsLoading}
      <div class="artifacts-loading">
        <i class="fas fa-spinner fa-spin"></i> Loading artifacts...
      </div>
    {:else if artifacts.length === 0}
      <div class="no-artifacts-message">No artifacts for this operation</div>
    {:else}
      <div class="table-responsive section-table dot-grid-background">
        <table class="table table-xs border-0 m-0">
          <thead>
            <tr>
              <th class="artifact-column">Artifact ID</th>
              <th class="text-center">Type</th>
              <th class="text-center">Size</th>
              <th class="text-center">Exists</th>
              <th class="text-center">Created</th>
            </tr>
          </thead>
          <tbody>
            {#each artifacts as artifact}
              <tr class="artifact-row">
                <td class="artifact-id-cell">
                  <span class="artifact-id" title={artifact.id}>
                    {artifact.id.substring(0, 12)}...
                  </span>
                </td>
                <td class="text-center">
                  <span
                    class="artifact-type-badge"
                    class:audio={artifact.artifact_type === 'audio'}
                  >
                    {artifact.artifact_type}
                  </span>
                </td>
                <td class="text-center artifact-number">
                  {formatBytes(artifact.size_bytes)}
                </td>
                <td class="text-center">
                  <span
                    class="exists-badge"
                    class:exists={artifact.exists}
                    class:missing={!artifact.exists}
                  >
                    {artifact.exists ? '✓' : '✗'}
                  </span>
                </td>
                <td class="text-center artifact-timestamp">
                  {formatTimestamp(artifact.created_at)}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </div>
{/if}

<style>
  .dot-grid-background {
    background-image: radial-gradient(circle, #141313 1px, transparent 1px);
    background-size: 5px 5px;
  }

  .artifacts-section {
    margin-top: 20px;
    padding: 10px 0;
    border-top: 1px solid rgba(255, 255, 255, 0.1);
  }

  .artifacts-title {
    color: #9d9d9d;
    font-size: 13px;
    font-weight: 600;
    margin: 10px 0 10px 0;
    padding: 0 5px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .artifacts-loading {
    padding: 15px;
    text-align: center;
    color: #7d7d7d;
    font-size: 12px;
  }

  .no-artifacts-message {
    padding: 15px;
    text-align: center;
    color: #5d5d5d;
    font-size: 12px;
    font-style: italic;
  }

  :global(.table) {
    margin-bottom: 0;
  }

  :global(th) {
    text-align: left;
    padding-top: 0px !important;
    padding-bottom: 0px !important;
    position: sticky !important;
    top: 0;
    font-size: 11px;
    color: #9d9d9d !important;
    border-bottom: 0px !important;
  }

  :global(td) {
    background-color: rgb(6, 5, 8) !important;
    border: 1px solid rgb(10, 9, 13) !important;
    color: #e8e8e8 !important;
    padding-top: 4px !important;
    padding-bottom: 3px !important;
    font-size: 11px;
    white-space: nowrap;
  }

  .artifact-row {
    border-bottom: 1px solid rgba(255, 255, 255, 0.05) !important;
  }

  .artifact-row:hover {
    background: linear-gradient(
      90deg,
      rgba(62, 60, 74, 0.5) 0%,
      rgba(73, 73, 105, 0.5) 46%,
      rgba(0, 22, 120, 0.5) 100%
    ) !important;
    border: 1px solid rgba(255, 255, 255, 0.1) !important;
  }

  .artifact-id-cell {
    max-width: 200px;
  }

  .artifact-id {
    font-family: 'Fira Code', monospace;
    color: #b0b0b0;
    cursor: pointer;
    transition: color 0.2s;
  }

  .artifact-id:hover {
    color: #e8e8e8;
  }

  .artifact-type-badge {
    display: inline-block;
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    background-color: rgba(100, 100, 100, 0.3);
    color: #a0a0a0;
    border: 1px solid rgba(100, 100, 100, 0.5);
  }

  .artifact-type-badge.audio {
    background-color: rgba(76, 120, 168, 0.2);
    color: #7ca3d4;
    border-color: rgba(76, 120, 168, 0.4);
  }

  .artifact-number {
    font-family: 'Fira Code', monospace;
    color: #b0b0b0;
  }

  .exists-badge {
    display: inline-block;
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 11px;
    font-weight: 600;
  }

  .exists-badge.exists {
    background-color: rgba(76, 175, 80, 0.2);
    color: #81c784;
    border: 1px solid rgba(76, 175, 80, 0.4);
  }

  .exists-badge.missing {
    background-color: rgba(244, 67, 54, 0.2);
    color: #ef5350;
    border: 1px solid rgba(244, 67, 54, 0.4);
  }

  .artifact-timestamp {
    font-family: 'Fira Code', monospace;
    color: #8d8d8d;
    font-size: 10px;
  }
</style>
