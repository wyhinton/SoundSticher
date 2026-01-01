<script lang="ts">
  import { GroupRegistry, type GroupDef, type GroupsState, testGroups } from '../state/groups';
  import { appState } from '../state/state.svelte';
  import { selectionService } from '../state/selection.svelte';
  import GroupItem from './GroupItem.svelte';

  // Initialize groups state with test data for now
  let groupsState: GroupsState = {
    defs: Object.fromEntries(testGroups.map(g => [g.name, g.def])),
    folders: {
      'Basic Queries': ['sec0_half', 'global_last', 'active_only'],
      Combined: ['half_or_last', 'combo'],
    },
    _version: 1,
  };

  // Create registry
  const registry = new GroupRegistry(() => groupsState.defs);

  // Track which groups are expanded/selected
  let expandedFolders = new Set<string>(['Basic Queries', 'Combined']);
  let selectedGroup: string | null = null;
  let groupResults = new Map<string, Set<string>>();

  // Toggle folder expansion
  function toggleFolder(folderName: string) {
    if (expandedFolders.has(folderName)) {
      expandedFolders.delete(folderName);
    } else {
      expandedFolders.add(folderName);
    }
    expandedFolders = new Set(expandedFolders);
  }

  // Select a group and evaluate it
  function selectGroup(groupName: string) {
    selectedGroup = groupName;

    try {
      const result = registry.eval(groupName, $appState);
      groupResults.set(groupName, result);
      groupResults = new Map(groupResults);

      // Convert file IDs to segment indices for selection
      const segmentIndices = convertFileIdsToSegmentIndices(result);

      // Update global selection
      selectionService.apply({
        mode: 'replace',
        ids: segmentIndices,
        source: 'groups',
      });
    } catch (error) {
      console.error(`Error evaluating group "${groupName}":`, error);
    }
  }

  // Convert file IDs to timeline segment indices
  function convertFileIdsToSegmentIndices(fileIds: Set<string>): number[] {
    const indices: number[] = [];

    if ($appState?.timelineItems) {
      $appState.timelineItems.forEach((item, index) => {
        if (item.type === 'audio-file' && fileIds.has(item.id)) {
          indices.push(index);
        }
      });
    }

    return indices;
  }

  // Get result count for a group
  function getResultCount(groupName: string): number {
    return groupResults.get(groupName)?.size ?? 0;
  }

  // Get groups that are not in any folder
  $: ungroupedDefs = Object.keys(groupsState.defs).filter(
    name =>
      !Object.values(groupsState.folders || {}).some(folderGroups => folderGroups.includes(name))
  );
</script>

<div class="groups-container">
  <div class="groups-list">
    <!-- Render folders if they exist -->
    {#if groupsState.folders}
      {#each Object.entries(groupsState.folders) as [folderName, groupNames]}
        <div class="folder">
          <div
            class="folder-header"
            class:expanded={expandedFolders.has(folderName)}
            onclick={() => toggleFolder(folderName)}
          >
            <i class="fa fa-{expandedFolders.has(folderName) ? 'chevron-down' : 'chevron-right'}"
            ></i>
            <span class="folder-name">{folderName}</span>
            <span class="folder-count">({groupNames.length})</span>
          </div>

          {#if expandedFolders.has(folderName)}
            <div class="folder-content">
              {#each groupNames as groupName}
                {@const def = groupsState.defs[groupName]}
                {#if def}
                  <GroupItem
                    {groupName}
                    definition={def}
                    isSelected={selectedGroup === groupName}
                    resultCount={groupResults.has(groupName) ? getResultCount(groupName) : null}
                    onSelect={selectGroup}
                  />
                {/if}
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    {/if}

    <!-- Render ungrouped definitions -->
    {#if ungroupedDefs.length > 0}
      <div class="folder">
        <div class="folder-header ungrouped">
          <span class="folder-name">Other Groups</span>
          <span class="folder-count">({ungroupedDefs.length})</span>
        </div>
        <div class="folder-content">
          {#each ungroupedDefs as groupName}
            {@const def = groupsState.defs[groupName]}
            {#if def}
              <GroupItem
                {groupName}
                definition={def}
                isSelected={selectedGroup === groupName}
                resultCount={groupResults.has(groupName) ? getResultCount(groupName) : null}
                onSelect={selectGroup}
              />
            {/if}
          {/each}
        </div>
      </div>
    {/if}
  </div>

  <!-- Selected group details -->
  {#if selectedGroup && groupsState.defs[selectedGroup]}
    <div class="group-details-panel">
      <h5>Group: {selectedGroup}</h5>
      <div class="definition">
        <strong>Definition:</strong>
        <pre>{JSON.stringify(groupsState.defs[selectedGroup], null, 2)}</pre>
      </div>

      {#if groupResults.has(selectedGroup)}
        {@const result = groupResults.get(selectedGroup)}
        <div class="results">
          <strong>Results ({result?.size} items):</strong>
          <div class="result-list">
            {#each Array.from(result || []) as itemId}
              <span class="result-item">{itemId}</span>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .groups-container {
    padding: 12px;
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 12px;
    overflow: hidden;
  }

  .groups-list {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }

  .folder {
    margin-bottom: 8px;
  }

  .folder-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    background: #2a2a2a;
    border-radius: 4px;
    cursor: pointer;
    transition: background-color 0.2s ease;
    font-size: 12px;
  }

  .folder-header:hover {
    background: #3a3a3a;
  }

  .folder-header.expanded {
    background: #3a3a3a;
  }

  .folder-header.ungrouped {
    cursor: default;
    opacity: 0.8;
  }

  .folder-name {
    font-weight: 500;
    color: #ccc;
  }

  .folder-count {
    color: #888;
    font-size: 11px;
  }

  .folder-content {
    margin-left: 16px;
    margin-top: 4px;
    border-left: 1px solid #444;
    padding-left: 12px;
  }

  .group-details-panel {
    border-top: 1px solid #444;
    padding-top: 12px;
    max-height: 40%;
    overflow-y: auto;
  }

  .group-details-panel h5 {
    margin: 0 0 8px 0;
    font-size: 13px;
    color: #fff;
  }

  .definition {
    margin-bottom: 12px;
  }

  .definition strong {
    font-size: 11px;
    color: #ccc;
  }

  .definition pre {
    background: #1a1a1a;
    border: 1px solid #444;
    border-radius: 4px;
    padding: 8px;
    margin: 4px 0;
    font-size: 10px;
    color: #ccc;
    overflow-x: auto;
  }

  .results strong {
    font-size: 11px;
    color: #ccc;
  }

  .result-list {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 4px;
  }

  .result-item {
    background: #444;
    color: #fff;
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 10px;
    font-family: monospace;
  }

  /* Scrollbar styling */
  .groups-list::-webkit-scrollbar,
  .group-details-panel::-webkit-scrollbar {
    width: 6px;
  }

  .groups-list::-webkit-scrollbar-track,
  .group-details-panel::-webkit-scrollbar-track {
    background: #1a1a1a;
  }

  .groups-list::-webkit-scrollbar-thumb,
  .group-details-panel::-webkit-scrollbar-thumb {
    background: #444;
    border-radius: 3px;
  }

  .groups-list::-webkit-scrollbar-thumb:hover,
  .group-details-panel::-webkit-scrollbar-thumb:hover {
    background: #555;
  }
</style>
