<script lang="ts">
  import { type OperationDef } from '$lib/state/operation';
  import { dispatch, type AddOperationCommand } from '$lib/state/undo/undo';

  function addMergeOpRender() {
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    const operationName = `merge_${timestamp}`;
    const command: AddOperationCommand = {
      type: 'add-operation',
      operation: {
        renderPolicy: 'auto',
        name: operationName,
        kind: 'merge',
        sources: [],
        outputPath: `output/combined_${timestamp}.wav`,
        gapSeconds: 0,
        format: 'wav',
      } as Omit<OperationDef, 'id'>,
    };
    dispatch(command, `Add Merge Operation: ${operationName}`);
  }
</script>

<div class="operation-creation-panel">
  <div class="creation-header" style="--header-bg: {undefined}">
    <h4>Add Operations</h4>
  </div>
  <div class="operation-buttons">
    <button class="operation-add-btn" on:click={addMergeOpRender} title="Add merge operation">
      <span class="operation-icon">🔗</span>
      <span class="operation-label">Merge</span>
      <i class="fa fa-plus"></i>
    </button>

    <button
      class="operation-add-btn"
      on:click={() => console.log('Split operation - coming soon')}
      title="Add split operation"
    >
      <span class="operation-icon">✂️</span>
      <span class="operation-label">Split</span>
      <i class="fa fa-plus"></i>
    </button>

    <button
      class="operation-add-btn"
      on:click={() => console.log('FX Rack operation - coming soon')}
      title="Add FX rack operation"
    >
      <span class="operation-icon">🎛️</span>
      <span class="operation-label">FX Rack</span>
      <i class="fa fa-plus"></i>
    </button>

    <!-- <button
      class="operation-add-btn"
      on:click={() => console.log('Stems operation - coming soon')}
      title="Add stems operation"
    >
      <span class="operation-icon">🎵</span>
      <span class="operation-label">Stems</span>
      <i class="fa fa-plus"></i>
    </button>

    <button
      class="operation-add-btn"
      on:click={() => console.log('Audio Wrangle operation - coming soon')}
      title="Add audio wrangle operation"
    >
      <span class="operation-icon">🔧</span>
      <span class="operation-label">Audio Wrangle</span>
      <i class="fa fa-plus"></i>
    </button>

    <button
      class="operation-add-btn"
      on:click={() => console.log('Layer operation - coming soon')}
      title="Add layer operation"
    >
      <span class="operation-icon">📚</span>
      <span class="operation-label">Layer</span>
      <i class="fa fa-plus"></i>
    </button> -->
  </div>
</div>

<style>
  .operation-creation-panel {
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
</style>
