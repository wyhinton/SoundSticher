<script lang="ts">
  import {
    appState,
    removeFromFavorites,
    addSource,
    addSourceToCurrentOperation,
  } from '../state/state.svelte';

  function handleRemoveFromFavorites(path: string) {
    removeFromFavorites(path);
  }

  function handleAddFavoriteAsSource(path: string) {
    addSourceToCurrentOperation([path]);
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
</script>

<div class="favorites-container">
  {#if $appState.favorites}
    {#if $appState.favorites && $appState.favorites.length === 0}
      <div class="empty-state">
        <i class="fas fa-heart empty-icon"></i>
        <p>No favorites yet</p>
        <small>Add folders to favorites from the dropdown menu in the sources table</small>
      </div>
    {:else}
      <div class="favorites-list">
        {#each $appState.favorites as favorite, index (favorite.path)}
          <div
            class="favorite-item"
            draggable="true"
            ondragstart={event => handleDragStart(event, favorite.path)}
            ondragend={handleDragEnd}
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
  }

  .favorites-header {
    margin-bottom: 12px;
    padding-bottom: 8px;
    border-bottom: 1px solid #444;
  }

  .favorites-title {
    margin: 0;
    font-size: 12px;
    font-weight: bold;
    color: #fff;
    display: flex;
    align-items: center;
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
</style>
