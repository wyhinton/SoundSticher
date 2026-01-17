<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { invoke } from '@tauri-apps/api/core';
  import {
    appState,
    addOperationSourceToCurrent,
    addSampleOpsFromDirectory,
  } from '../state/state.svelte';
  import { removeFromFavorites, addToFavorites } from '$lib/state/favorites';

  let showDebugPanel = false;

  function handleRemoveFromFavorites(path: string) {
    removeFromFavorites(path);
  }

  async function handleAddFolderToFavorites() {
    try {
      const selected = await open({
        multiple: true,
        directory: true,
        title: 'Select folders to add to favorites',
      });

      if (selected && Array.isArray(selected) && selected.length > 0) {
        // Add each selected folder to favorites
        for (const folderPath of selected) {
          await addToFavorites(folderPath);
        }
        console.log(`Added ${selected.length} folder(s) to favorites:`, selected);
      } else if (selected && typeof selected === 'string') {
        // Handle single folder selection
        await addToFavorites(selected);
        console.log('Added folder to favorites:', selected);
      }
    } catch (error) {
      console.error('Error opening folder dialog:', error);
    }
  }

  async function handleAddFavoriteAsSource(path: string) {
    try {
      await addSampleOpsFromDirectory(path);
      console.log(`Added all files from directory as SampleOps: ${path}`);
    } catch (error) {
      console.error('Failed to add favorite as source:', error);
    }
  }

  function getDirectoryName(path: string): string {
    // Extract just the directory name from the full path
    const parts = path.split(/[/\\]/);
    return parts[parts.length - 1] || path;
  }

  // Drag and drop functionality
  function handleDragStart(event: DragEvent, favoritePath: string) {
    if (!event.dataTransfer) return;

    // Set the data that will be transferred
    event.dataTransfer.effectAllowed = 'copy';
    event.dataTransfer.setData('text/plain', favoritePath);
    event.dataTransfer.setData('application/x-favorite-path', favoritePath);

    // Create a drag image
    const dragElement = event.currentTarget as HTMLElement;
    if (dragElement) {
      event.dataTransfer.setDragImage(dragElement, 20, 20);
    }

    console.log('Started dragging favorite:', favoritePath);
  }

  function handleDragEnd(event: DragEvent) {
    console.log('Drag ended');
  }

  // Debug Panel Functions
  function handleKeyDown(event: KeyboardEvent) {
    if (event.ctrlKey && event.shiftKey && event.code === 'Space') {
      event.preventDefault();
      showDebugPanel = !showDebugPanel;
      console.log('Debug panel toggled:', showDebugPanel);
    }
  }

  async function deleteAllFavorites() {
    if (confirm('Are you sure you want to delete ALL favorites? This cannot be undone.')) {
      appState.update(state => {
        state.favorites = [];
        console.log('All favorites deleted');
        return state;
      });
    }
  }

  async function refreshAllFileCount() {
    try {
      if (!$appState.favorites || $appState.favorites.length === 0) {
        console.log('No favorites to refresh');
        return;
      }

      // Get all folder paths
      const folderPaths = $appState.favorites.map(fav => fav.path);

      // Call the count function with all paths at once
      const counts = await invoke<Record<string, number>>('count_audio_files_in_folders', {
        folderPaths,
      });

      // Update all favorites with new counts
      appState.update(state => {
        if (state.favorites && Array.isArray(state.favorites)) {
          state.favorites = state.favorites.map(fav => ({
            ...fav,
            numAudioFiles: counts[fav.path] || 0,
          }));
          console.log('File counts refreshed for all favorites:', counts);
        }
        return state;
      });
    } catch (error) {
      console.error('Error refreshing file counts:', error);
    }
  }
</script>

<div class="favorites-container" onkeydown={handleKeyDown}>
  {#if $appState.favorites}
    {#if $appState.favorites && $appState.favorites.length === 0}
      <div class="empty-state">
        <i class="fas fa-heart empty-icon"></i>
        <p>No favorites yet</p>
        <small
          >Add folders to favorites from the dropdown menu in the sources table or use the button
          below</small
        >
        <button
          class="add-folder-btn"
          onclick={handleAddFolderToFavorites}
          title="Add folders to favorites"
          aria-label="Add folders to favorites"
        >
          <i class="fas fa-folder-plus"></i>
          Add Folders
        </button>
      </div>
    {:else}
      <div class="favorites-header">
        <div class="favorites-title">
          <i class="fas fa-heart" style="margin-right: 6px;"></i>
          Favorites ({$appState.favorites.length})
        </div>
        <button
          class="header-add-btn"
          onclick={handleAddFolderToFavorites}
          title="Add more folders to favorites"
          aria-label="Add more folders to favorites"
        >
          <i class="fas fa-plus"></i>
        </button>
      </div>
      <div class="favorites-list">
        {#each $appState.favorites as favorite, index (favorite.path)}
          <div
            class="favorite-item"
            draggable="true"
            ondragstart={event => handleDragStart(event, favorite.path)}
            ondragend={handleDragEnd}
            role="button"
            tabindex="0"
          >
            <div class="favorite-info">
              <i class="fas fa-folder favorite-icon"></i>
              <div class="favorite-details">
                <div class="favorite-name" title={favorite.path}>
                  {getDirectoryName(favorite.path)}
                </div>
                <div class="favorite-path">
                  {favorite.path}
                </div>
                <div class="favorite-count">
                  {favorite.numAudioFiles || 0} audio file{(favorite.numAudioFiles || 0) === 1
                    ? ''
                    : 's'}
                </div>
              </div>
            </div>

            <div class="favorite-actions">
              <button
                class="action-btn add-btn"
                onclick={() => handleAddFavoriteAsSource(favorite.path)}
                title="Add as source"
                aria-label="Add favorite as source"
              >
                <i class="fas fa-plus"></i>
              </button>

              <button
                class="action-btn remove-btn"
                onclick={() => handleRemoveFromFavorites(favorite.path)}
                title="Remove from favorites"
                aria-label="Remove from favorites"
              >
                <i class="fas fa-trash"></i>
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
    <!-- content here -->
  {/if}
  {#if showDebugPanel}
    <div class="debug-panel" onkeydown={handleKeyDown}>
      <button class="debug-btn refresh-btn" onclick={refreshAllFileCount}>
        <i class="fas fa-sync-alt"></i>
        Refresh All File Counts
      </button>
      <button class="debug-btn delete-btn" onclick={deleteAllFavorites}>
        <i class="fas fa-trash"></i>
        Delete All Favorites
      </button>
    </div>
  {/if}
</div>

<style>
  .favorites-container {
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    text-align: center;
    color: #9d9d9d;
  }

  .empty-icon {
    font-size: 32px;
    margin-bottom: 12px;
    opacity: 0.6;
  }

  .empty-state p {
    margin: 0 0 8px 0;
    font-size: 14px;
    color: #ccc;
  }

  .empty-state small {
    font-size: 11px;
    color: #777;
    max-width: 200px;
    line-height: 1.3;
    margin-bottom: 16px;
  }

  .add-folder-btn {
    background: linear-gradient(to bottom, #28a745, #1e7e34);
    border: 1px solid #155724;
    color: white;
    padding: 4px 16px;
    border-radius: 4px;
    font-size: 12px;
    cursor: pointer;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .add-folder-btn:hover {
    background: linear-gradient(to bottom, #34ce57, #28a745);
    transform: translateY(-1px);
    box-shadow: 0 2px 8px rgba(40, 167, 69, 0.3);
  }

  .add-folder-btn:active {
    transform: translateY(0);
    box-shadow: 0 1px 4px rgba(40, 167, 69, 0.2);
  }

  .favorites-header {
    margin-bottom: 12px;
    padding-bottom: 8px;
    border-bottom: 1px solid #444;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .favorites-title {
    margin: 0;
    font-size: 12px;
    font-weight: bold;
    color: #fff;
    display: flex;
    align-items: center;
  }

  .header-add-btn {
    background: rgba(40, 167, 69, 0.2);
    border: 1px solid rgba(40, 167, 69, 0.5);
    color: #28a745;
    padding: 4px 8px;
    border-radius: 3px;
    font-size: 10px;
    cursor: pointer;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .header-add-btn:hover {
    background: rgba(40, 167, 69, 0.3);
    border-color: #28a745;
    transform: scale(1.05);
  }

  .header-add-btn:active {
    transform: scale(0.95);
  }

  .favorites-list {
    flex: 1;
    overflow-y: auto;
  }

  .favorite-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid #444;
    border-radius: 4px;
    transition: all 0.2s ease;
    cursor: grab;
    user-select: none;
  }

  .favorite-item:hover {
    background: rgba(255, 255, 255, 0.08);
    border-color: #555;
  }

  .favorite-item:focus {
    outline: 2px solid #007acc;
    outline-offset: 2px;
  }

  .favorite-item:active {
    cursor: grabbing;
  }

  .favorite-item[draggable='true']:hover {
    transform: translateY(-1px);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
  }

  .favorite-info {
    display: flex;
    align-items: center;
    flex: 1;
    min-width: 0;
  }

  .favorite-icon {
    color: #ffd700;
    margin-right: 8px;
    font-size: 14px;
    flex-shrink: 0;
  }

  .favorite-details {
    min-width: 0;
    flex: 1;
  }

  .favorite-name {
    font-size: 12px;
    font-weight: 500;
    color: #fff;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-bottom: 2px;
  }

  .favorite-path {
    font-size: 10px;
    color: #888;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .favorite-actions {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .action-btn {
    background: none;
    border: none;
    color: #9d9d9d;
    padding: 6px;
    border-radius: 3px;
    cursor: pointer;
    font-size: 10px;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
  }

  .action-btn:hover {
    background: rgba(255, 255, 255, 0.1);
  }

  .add-btn:hover {
    color: #28a745;
    transform: scale(1.1);
  }

  .remove-btn:hover {
    color: #e74c3c;
    transform: scale(1.1);
  }

  .action-btn:active {
    transform: scale(0.9);
  }

  /* Custom scrollbar for favorites list */
  .favorites-list::-webkit-scrollbar {
    width: 6px;
  }

  .favorites-list::-webkit-scrollbar-track {
    background: rgba(255, 255, 255, 0.05);
    border-radius: 3px;
  }

  .favorites-list::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.2);
    border-radius: 3px;
  }

  .favorites-list::-webkit-scrollbar-thumb:hover {
    background: rgba(255, 255, 255, 0.3);
  }

  /* Debug Panel Styles */
  .debug-panel {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    background: rgba(40, 167, 69, 0.05);
    border: 1px solid rgba(40, 167, 69, 0.3);
    border-radius: 4px;
    margin-top: 12px;
  }

  .debug-btn {
    padding: 8px 12px;
    border: 1px solid rgba(40, 167, 69, 0.5);
    border-radius: 4px;
    background: rgba(40, 167, 69, 0.1);
    color: #28a745;
    cursor: pointer;
    font-size: 11px;
    font-weight: 500;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
  }

  .debug-btn:hover {
    background: rgba(40, 167, 69, 0.2);
    border-color: #28a745;
    transform: translateX(2px);
  }

  .debug-btn:active {
    transform: translateX(0);
  }

  .delete-btn {
    border-color: rgba(231, 76, 60, 0.5);
    background: rgba(231, 76, 60, 0.1);
    color: #e74c3c;
  }

  .delete-btn:hover {
    background: rgba(231, 76, 60, 0.2);
    border-color: #e74c3c;
  }
</style>
