<script lang="ts">
  import { appState, setActiveTab } from '$lib/state/state.svelte';
  import Favorites from './Favorites.svelte';
  import Groups from './Groups/Groups.svelte';
  import Operations from './Operations/Operations.svelte';
</script>

<!-- Tab panel section -->
<section class="tab-panel-section">
  <div class="tab-panel-container">
    <!-- Tab navigation -->
    <!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role -->
    <nav class="tab-navigation" role="tablist" aria-label="Source panel tabs">
      <button
        class="tab"
        class:active={$appState.uiSettings?.activeTab === 'Operations'}
        onclick={() => setActiveTab('Operations')}
        role="tab"
        aria-selected={$appState.uiSettings?.activeTab === 'Operations'}
        aria-controls="operations-tab-panel"
      >
        Operations
      </button>
      <button
        class="tab"
        class:active={$appState.uiSettings?.activeTab === 'Group'}
        onclick={() => setActiveTab('Group')}
        role="tab"
        aria-selected={$appState.uiSettings?.activeTab === 'Group'}
        aria-controls="group-tab-panel"
      >
        Group
      </button>
      <button
        class="tab"
        class:active={$appState.uiSettings?.activeTab === 'Favorites'}
        onclick={() => setActiveTab('Favorites')}
        role="tab"
        aria-selected={$appState.uiSettings?.activeTab === 'Favorites'}
        aria-controls="favorites-tab-panel"
      >
        Favorites
      </button>
    </nav>

    <!-- Tab content -->
    <div class="tab-content">
      {#if $appState.uiSettings?.activeTab === 'Operations'}
        <div
          class="tab-panel"
          id="operations-tab-panel"
          role="tabpanel"
          aria-labelledby="operations-tab"
          style="background-color: {$appState.uiSettings?.theme?.tabPanelBackgroundColor ||
            'rgb(15 21 27)'};"
        >
          <Operations />
        </div>
      {/if}
      {#if $appState.uiSettings?.activeTab === 'Group'}
        <div
          class="tab-panel"
          id="group-tab-panel"
          role="tabpanel"
          aria-labelledby="group-tab"
          style="background-color: {$appState.uiSettings?.theme?.tabPanelBackgroundColor ||
            'rgb(15 21 27)'};"
        >
          <Groups />
        </div>
      {/if}
      {#if $appState.uiSettings?.activeTab === 'Favorites'}
        <div
          class="tab-panel"
          id="favorites-tab-panel"
          role="tabpanel"
          aria-labelledby="favorites-tab"
          style="background-color: {$appState.uiSettings?.theme?.tabPanelBackgroundColor ||
            'rgb(15 21 27)'};"
        >
          <Favorites />
        </div>
      {/if}
    </div>
  </div>
</section>

<style>
  .tab-panel-section {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    background-color: rgb(15 21 27);
    border-right: 1px solid #555;
  }

  .tab-panel-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
  }

  /* Tab styles */
  .tab-navigation {
    display: flex;
    background-color: rgb(15 21 27);
    padding: 0 8px;
    border-bottom: 1px solid #555;
    gap: 2px;
    height: 30px;
  }

  .tab {
    padding: 8px 16px;
    cursor: pointer;
    position: relative;
    background: #2a2a2a;
    border: 1px solid #555;
    border-bottom: none;
    color: #9d9d9d;
    font-size: 11px;
    transition: all 0.2s ease;
    border-radius: 6px 6px 0 0;
    margin-top: 4px;
    min-width: 70px;
  }

  .tab:hover {
    background-color: #3a3a3a;
    color: #fff;
    border-color: #666;
  }

  .tab.active {
    color: #fff;
    font-weight: bold;
    background-color: rgb(15 21 27);
    border-color: #777;
    margin-top: 0;
    padding-top: 8px;
    z-index: 1;
    position: relative;
  }

  .tab.active::after {
    content: '';
    position: absolute;
    bottom: -1px;
    left: 0;
    right: 0;
    height: 1px;
    background: rgb(15 21 27);
  }

  .tab-content {
    flex: 1;
    background-color: rgb(15 21 27);
    border-top: none;
    border-radius: 0 0 4px 4px;
    overflow-y: auto;
    height: 100%;
  }

  .tab-panel {
    color: #9d9d9d;
    font-size: 12px;
    line-height: 1.4;
    height: 100%;
    padding: 12px;
  }

  .tab-panel p {
    margin: 0 0 12px 0;
    color: #ccc;
  }
</style>
