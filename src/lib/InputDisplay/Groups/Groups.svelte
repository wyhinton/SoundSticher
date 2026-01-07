<script lang="ts">
  import GroupItem from './GroupItem.svelte';
  import GroupDebugPanel from './GroupDebugPanel.svelte';
  import GroupDetailsPanel from './GroupDetailsPanel.svelte';
  import ButtonPill from '$lib/components/ButtonPill.svelte';
  import {
    testGroups,
    GroupRegistry,
    type ItemQuery,
    patchGroupQuery,
    ItemQueryDetailsDictionary,
  } from '$lib/state/groups';
  import { selectionService, previewService } from '$lib/state/selection.svelte';
  import { appState } from '$lib/state/state.svelte';

  // Initialize groups state - use appState groups if available, otherwise fall back to test data
  $: groupsState = $appState.groups || {
    defs: Object.fromEntries(testGroups.map(g => [g.name, g.def])),
    folders: {
      'Basic Queries': ['sec0_half', 'global_last', 'active_only'],
      Combined: ['half_or_last', 'combo'],
    },
    _version: 1,
  };

  // Create registry
  const registry = new GroupRegistry(() => groupsState.defs);

  // Track selected group
  let selectedGroup: string | null = null;
  let groupResults = new Map<string, Set<string>>();

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

  // Handle group hover for preview
  function handleGroupHover(groupName: string) {
    try {
      const result = registry.eval(groupName, $appState);
      const segmentIndices = convertFileIdsToSegmentIndices(result);
      previewService.setPreview(segmentIndices, 'groups');
    } catch (error) {
      console.error(`Error evaluating group "${groupName}" for preview:`, error);
      previewService.clearPreview('groups');
    }
  }

  // Handle group hover leave
  function handleGroupHoverLeave() {
    previewService.clearPreview('groups');
  }

  // Get groups that are not in any folder
  $: ungroupedDefs = Object.keys(groupsState.defs).filter(
    name =>
      !Object.values(groupsState.folders || {}).some(folderGroups => folderGroups.includes(name))
  );

  // Create a combined structure that includes both folders and ungrouped items
  $: allFolders = {
    ...groupsState.folders,
    ...(ungroupedDefs.length > 0 ? { 'Other Groups': ungroupedDefs } : {}),
  };

  // Handle query parameter updates
  function handleUpdateQuery(groupName: string, patch: Partial<ItemQuery>) {
    // Clear the cache for this group since we're updating it
    registry.invalidateAll();

    // Update the query using the patchGroupQuery function
    patchGroupQuery(groupName, patch);

    // Re-evaluate the group if it's currently selected
    if (selectedGroup === groupName) {
      try {
        const result = registry.eval(groupName, $appState);
        groupResults.set(groupName, result);
        groupResults = new Map(groupResults);

        // Update selection with new results
        const segmentIndices = convertFileIdsToSegmentIndices(result);
        selectionService.apply({
          mode: 'replace',
          ids: segmentIndices,
          source: 'groups',
        });
      } catch (error) {
        console.error('Failed to re-evaluate group after update:', error);
      }
    }
  }

  // Add new query functionality
  let newQueryName = '';

  // Debug panel functionality
  let showDebugPanel = false;

  function handleKeyDown(event: KeyboardEvent) {
    // Handle Ctrl+Shift+Space to toggle debug panel
    if (event.ctrlKey && event.shiftKey && event.code === 'Space') {
      event.preventDefault();
      showDebugPanel = !showDebugPanel;
    }
  }

  const queryTemplates = [
    {
      name: 'Section Percent',
      value: 'sectionPercent',
      kind: 'sectionPercent' as const,
      template: {
        kind: 'sectionPercent',
        sectionIndex: 0,
        percent: 0.5,
        orderBy: 'index',
        take: 'first',
      },
    },
    {
      name: 'Random Section Percent',
      value: 'randomSectionPercent',
      kind: 'randomSectionPercent' as const,
      template: { kind: 'randomSectionPercent', sectionIndex: 0, percent: 0.5, seed: 42 },
    },
    {
      name: 'Last of Each Section',
      value: 'lastOfEachSection',
      kind: 'lastOfEachSection' as const,
      template: { kind: 'lastOfEachSection' },
    },
    {
      name: 'Last of All Sections',
      value: 'lastOfAllSections',
      kind: 'lastOfAllSections' as const,
      template: { kind: 'lastOfAllSections' },
    },
    {
      name: 'Active Files Only',
      value: 'where-active',
      kind: 'where' as const,
      template: { kind: 'where', clause: { field: 'active', eq: true } },
    },
    {
      name: 'By Color',
      value: 'where-color',
      kind: 'where' as const,
      template: {
        kind: 'where',
        clause: { field: 'color', eq: { name: 'Red', rgb: [255, 64, 64] } },
      },
    },
    {
      name: 'By Duration',
      value: 'where-duration',
      kind: 'where' as const,
      template: { kind: 'where', clause: { field: 'duration', gt: 1.0 } },
    },
    {
      name: 'By Path',
      value: 'where-path',
      kind: 'where' as const,
      template: { kind: 'where', clause: { field: 'path', includes: '' } },
    },
  ];

  function addNewQuery(templateValue: string) {
    const template = queryTemplates.find(t => t.value === templateValue);
    if (!template) return;

    // Generate a default name if none provided
    let queryName = newQueryName.trim();
    if (!queryName) {
      // Convert template name to snake_case format
      const baseName = template.name.replace(/\s+/g, '_');
      queryName = baseName;

      // Find the next available name by checking for conflicts
      let counter = 0;
      while (groupsState.defs[queryName]) {
        queryName = `${baseName}_${counter}`;
        counter++;
      }
    } else {
      // Check if provided name already exists
      if (groupsState.defs[queryName]) {
        alert(`A group named "${queryName}" already exists`);
        return;
      }
    }

    // Update appState to add the new group
    appState.update(state => {
      if (!state.groups) {
        state.groups = { defs: {}, _version: 1 };
      }

      state.groups.defs[queryName] = {
        kind: 'query',
        query: template.template as ItemQuery,
      };

      state.groups._version = (state.groups._version || 0) + 1;
      state._rev = (state._rev || 0) + 1;

      return state;
    });

    // Clear the input
    newQueryName = '';

    // Invalidate cache since we added a new group
    registry.invalidateAll();
  }

  // Handle group deletion
  function handleDeleteGroup(groupName: string) {
    // Confirm deletion
    if (!confirm(`Are you sure you want to delete the group "${groupName}"?`)) {
      return;
    }

    // Clear selection if the deleted group is currently selected
    if (selectedGroup === groupName) {
      selectedGroup = null;
      selectionService.apply({
        mode: 'replace',
        ids: [],
        source: 'groups',
      });
    }

    // Clear preview if the deleted group is being previewed
    previewService.clearPreview('groups');

    // Remove from results cache
    groupResults.delete(groupName);
    groupResults = new Map(groupResults);

    // Update appState to remove the group
    appState.update(state => {
      if (!state.groups) return state;

      // Remove the group definition
      delete state.groups.defs[groupName];

      // Remove from folders
      if (state.groups.folders) {
        for (const [folderName, groupNames] of Object.entries(state.groups.folders)) {
          const index = groupNames.indexOf(groupName);
          if (index !== -1) {
            groupNames.splice(index, 1);
            // Remove empty folders
            if (groupNames.length === 0) {
              delete state.groups.folders[folderName];
            }
          }
        }
      }

      state.groups._version = (state.groups._version || 0) + 1;
      state._rev = (state._rev || 0) + 1;

      return state;
    });

    // Invalidate cache since we removed a group
    registry.invalidateAll();
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

<div class="groups-container" role="region" aria-label="Groups panel">
  <!-- Add Query Header -->
  <div class="add-query-section">
    <div class="add-query-dropdown">
      <!-- <div class="query-name-input">
        <input
          type="text"
          placeholder="Enter query name..."
          bind:value={newQueryName}
          onkeydown={(e: KeyboardEvent) => {
            if (e.key === 'Enter' && queryTemplates.length > 0) {
              addNewQuery(queryTemplates[0]!.value);
            }
          }}
        />
      </div> -->
      <div class="query-templates">
        {#each queryTemplates.filter(template => !newQueryName.trim() || template.name
              .toLowerCase()
              .includes(newQueryName.toLowerCase()) || template.value
              .toLowerCase()
              .includes(newQueryName.toLowerCase())) as template}
          <ButtonPill
            name={template.name}
            icon={ItemQueryDetailsDictionary[template.kind].icon}
            disabled={false}
            title={`Create ${template.name} query`}
            onClick={() => addNewQuery(template.value)}
          />
        {/each}
      </div>
    </div>
  </div>

  <div
    class="groups-list"
    role="listbox"
    aria-label="Groups list"
    tabindex="0"
    onclick={(e: Event) => {
      // If the click target is the groups-list itself (not a child element),
      // then clear the selection
      if (e.target === e.currentTarget) {
        selectedGroup = null;
      }
    }}
    onkeydown={(e: KeyboardEvent) => {
      // Handle Escape key to clear selection
      if (e.key === 'Escape') {
        selectedGroup = null;
      }
    }}
  >
    <!-- Debug Panel - positioned at the top of groups list when shown -->
    {#if showDebugPanel}
      <GroupDebugPanel onClose={() => (showDebugPanel = false)} />
    {/if}

    <!-- Render all groups (without folder headers) -->
    {#each Object.entries(allFolders) as [folderName, groupNames]}
      {#each groupNames as groupName}
        {@const def = groupsState.defs[groupName]}
        {#if def}
          <GroupItem
            {groupName}
            definition={def}
            isSelected={selectedGroup === groupName}
            resultCount={groupResults.has(groupName) ? getResultCount(groupName) : null}
            onSelect={selectGroup}
            onHover={handleGroupHover}
            onHoverLeave={handleGroupHoverLeave}
            onDelete={handleDeleteGroup}
          />
        {/if}
      {/each}
    {/each}
  </div>

  <!-- Selected group details -->
  {#if selectedGroup && groupsState.defs[selectedGroup]}
    <GroupDetailsPanel
      groupName={selectedGroup}
      definition={groupsState.defs[selectedGroup]!}
      result={groupResults.get(selectedGroup) || null}
      onClose={() => (selectedGroup = null)}
      onUpdateQuery={handleUpdateQuery}
    />
  {/if}
</div>

<style>
  /* Add Query Dropdown Styles */
  .add-query-section {
    border-bottom: 1px solid #444;
    padding-bottom: 12px;
  }

  .add-query-dropdown {
    background: #1e1e1e;
    border: 1px solid #333;
    border-radius: 6px;
    padding: 12px;
  }

  .query-templates {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .groups-container {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 12px;
    overflow: hidden;
    position: relative;
    outline: none;
  }

  .groups-list {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }

  /* Scrollbar styling */
  .groups-list::-webkit-scrollbar {
    width: 6px;
  }

  .groups-list::-webkit-scrollbar-track {
    background: #1a1a1a;
  }

  .groups-list::-webkit-scrollbar-thumb {
    background: #444;
    border-radius: 3px;
  }

  .groups-list::-webkit-scrollbar-thumb:hover {
    background: #555;
  }
</style>
