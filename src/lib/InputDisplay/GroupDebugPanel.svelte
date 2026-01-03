<script lang="ts">
  import { appState } from '../state/state.svelte';
  import { testGroups } from '../state/groups';

  export let onClose: () => void;

  function deleteAllGroups() {
    const confirmed = confirm(
      'Are you sure you want to delete all groups? This action cannot be undone.'
    );
    if (!confirmed) return;

    appState.update(state => {
      if (!state.groups) {
        state.groups = { defs: {}, _version: 1 };
      }

      state.groups.defs = {};
      state.groups.folders = {};
      state.groups._version = (state.groups._version || 0) + 1;
      state._rev = (state._rev || 0) + 1;

      return state;
    });

    console.log('🗑️ All groups deleted');
  }

  function addTestGroups() {
    appState.update(state => {
      if (!state.groups) {
        state.groups = { defs: {}, folders: {}, _version: 1 };
      }

      // Add test group definitions
      testGroups.forEach(group => {
        state.groups!.defs[group.name] = group.def;
      });

      // Add test folders
      state.groups.folders = {
        ...state.groups.folders,
        'Basic Queries': ['sec0_half', 'global_last', 'active_only'],
        Combined: ['half_or_last', 'combo'],
      };

      state.groups._version = (state.groups._version || 0) + 1;
      state._rev = (state._rev || 0) + 1;

      return state;
    });

    console.log('🧪 Test groups added');
  }

  function exportGroups() {
    const groupsData = $appState.groups;
    if (!groupsData) {
      alert('No groups to export');
      return;
    }

    const dataStr = JSON.stringify(groupsData, null, 2);
    const dataBlob = new Blob([dataStr], { type: 'application/json' });
    const url = URL.createObjectURL(dataBlob);

    const link = document.createElement('a');
    link.href = url;
    link.download = 'groups-export.json';
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);

    console.log('📥 Groups exported');
  }

  function importGroups() {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json';

    input.onchange = event => {
      const file = (event.target as HTMLInputElement).files?.[0];
      if (!file) return;

      const reader = new FileReader();
      reader.onload = e => {
        try {
          const importedData = JSON.parse(e.target?.result as string);

          // Validate the structure
          if (!importedData.defs || typeof importedData.defs !== 'object') {
            throw new Error('Invalid groups file format');
          }

          appState.update(state => {
            state.groups = {
              defs: importedData.defs,
              folders: importedData.folders || {},
              _version: (importedData._version || 0) + 1,
            };
            state._rev = (state._rev || 0) + 1;
            return state;
          });

          console.log('📤 Groups imported successfully');
        } catch (error) {
          console.error('Failed to import groups:', error);
          alert('Failed to import groups: Invalid file format');
        }
      };
      reader.readAsText(file);
    };

    input.click();
  }

  function getGroupsStats() {
    const groups = $appState.groups;
    if (!groups) return { totalGroups: 0, totalFolders: 0 };

    return {
      totalGroups: Object.keys(groups.defs || {}).length,
      totalFolders: Object.keys(groups.folders || {}).length,
    };
  }

  $: stats = getGroupsStats();
</script>

<div class="debug-panel">
  <div class="debug-header">
    <span class="debug-title">
      <i class="fa fa-bug"></i>
      Groups Debug
    </span>
    <button
      class="btn-close"
      onclick={onClose}
      title="Close debug panel"
      aria-label="Close debug panel"
    >
      <i class="fa fa-times"></i>
    </button>
  </div>

  <div class="debug-buttons">
    <div class="button-group">
      <span class="group-title">Stats</span>
      <div class="stats-row">
        <span class="stat-badge">G:{stats.totalGroups}</span>
        <span class="stat-badge">F:{stats.totalFolders}</span>
      </div>
    </div>

    <div class="button-group">
      <span class="group-title">Actions</span>
      <button
        class="btn btn-xs btn-outline-danger"
        onclick={deleteAllGroups}
        title="Delete all groups"
      >
        <i class="fa fa-trash"></i>
        Delete All
      </button>
      <button
        class="btn btn-xs btn-outline-primary"
        onclick={addTestGroups}
        title="Add test groups"
      >
        <i class="fa fa-flask"></i>
        Add Tests
      </button>
    </div>

    <div class="button-group">
      <span class="group-title">File I/O</span>
      <button
        class="btn btn-xs btn-outline-secondary"
        onclick={exportGroups}
        title="Export groups to JSON"
      >
        <i class="fa fa-download"></i>
        Export
      </button>
      <button
        class="btn btn-xs btn-outline-secondary"
        onclick={importGroups}
        title="Import groups from JSON"
      >
        <i class="fa fa-upload"></i>
        Import
      </button>
    </div>
  </div>

  <div class="debug-info">
    <small>
      <i class="fa fa-info-circle"></i>
      Ctrl+Shift+Space to toggle
    </small>
  </div>
</div>

<style>
  .debug-panel {
    background: var(--bs-dark);
    border: 1px solid var(--bs-info);
    border-radius: 4px;
    padding: 4px 6px;
    margin: 4px 0;
    box-shadow: 0 1px 3px rgba(13, 202, 240, 0.2);
    font-size: 10px;
  }

  .debug-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 4px;
  }

  .debug-title {
    color: var(--bs-info);
    font-weight: 600;
    font-size: 11px;
    margin: 0;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .btn-close {
    background: none;
    border: none;
    color: var(--bs-secondary);
    padding: 0;
    width: 14px;
    height: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 2px;
    transition: all 0.15s ease;
    cursor: pointer;
  }

  .btn-close:hover {
    color: var(--bs-info);
    background: rgba(13, 202, 240, 0.1);
  }

  .btn-close i {
    font-size: 8px;
  }

  .debug-buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 4px 0;
  }

  .button-group {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 70px;
  }

  .group-title {
    color: var(--bs-light);
    font-size: 9px;
    font-weight: 600;
    margin: 0 0 2px 0;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .stats-row {
    display: flex;
    gap: 4px;
  }

  .stat-badge {
    background: var(--bs-secondary);
    color: var(--bs-light);
    padding: 1px 4px;
    border-radius: 2px;
    font-size: 8px;
    font-weight: 500;
    font-family: monospace;
  }

  .btn-xs {
    font-size: 9px;
    padding: 1px 4px;
    border-radius: 2px;
    transition: all 0.15s ease;
    line-height: 1.2;
    min-height: 16px;
    white-space: nowrap;
  }

  .btn-xs i {
    font-size: 8px;
    margin-right: 2px;
  }

  .btn-xs:hover {
    transform: translateY(-0.5px);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.15);
  }

  .debug-info {
    border-top: 1px solid var(--bs-secondary);
    padding-top: 3px;
    margin-top: 4px;
  }

  .debug-info small {
    font-size: 9px;
    color: var(--bs-secondary);
  }

  .debug-info i {
    font-size: 8px;
    margin-right: 2px;
  }

  /* Responsive adjustments */
  @media (max-width: 768px) {
    .debug-buttons {
      flex-direction: column;
      gap: 3px;
    }

    .button-group {
      min-width: auto;
    }
  }
</style>
