<script lang="ts">
  import Prism from 'prismjs';
  import 'prismjs/components/prism-json';
  import 'prismjs/themes/prism-okaidia.css';
  import { onMount, afterUpdate } from 'svelte';
  import { appState } from '$lib/state/state.svelte';

  export let data: any = {};
  export let language: string = 'json';
  export let maxHeight: string = '600px';
  export let fontSize: string = '0.7rem';
  export let lineHeight: string = '1.4';
  export let className: string = '';
  export let panelKey: string = 'default'; // Key to identify which panel this is (frontend/backend)

  let codeContainer: HTMLElement;
  let highlighted = '';
  let topLevelKeys: string[] = [];
  let searchTerm: string = '';
  let searchInput: HTMLInputElement;

  // Get toggle states from appState
  $: toggleStates =
    ($appState.uiSettings?.debugPanelPrismDisplay as Record<string, any>)?.[panelKey] || {};

  // Extract top-level keys from data
  $: {
    if (data && typeof data === 'object' && !Array.isArray(data)) {
      topLevelKeys = Object.keys(data);
    } else {
      topLevelKeys = [];
    }
  }

  // Filter data based on toggle states
  $: filteredData = (() => {
    if (topLevelKeys.length === 0 || typeof data !== 'object' || Array.isArray(data)) {
      return data;
    }

    const filtered: any = {};
    for (const key of topLevelKeys) {
      // Show property if toggle is true or if no toggle state exists (default to show)
      if (toggleStates[key] !== false) {
        filtered[key] = data[key];
      }
    }
    return filtered;
  })();

  // Search filtering function
  function searchFilter(obj: any, term: string): any {
    if (!term.trim()) return obj;

    const lowerTerm = term.toLowerCase();

    // Helper function to check if a value matches the search term
    function matchesSearch(value: any, key?: string): boolean {
      // Check if key name matches
      if (key && key.toLowerCase().includes(lowerTerm)) {
        return true;
      }

      // Check if value matches (convert to string for comparison)
      if (value !== null && value !== undefined) {
        const valueStr = String(value).toLowerCase();
        if (valueStr.includes(lowerTerm)) {
          return true;
        }
      }

      return false;
    }

    // Recursive function to filter nested objects/arrays
    function filterRecursive(item: any, parentKey?: string): any {
      if (item === null || item === undefined) {
        return matchesSearch(item, parentKey) ? item : undefined;
      }

      if (typeof item === 'string' || typeof item === 'number' || typeof item === 'boolean') {
        return matchesSearch(item, parentKey) ? item : undefined;
      }

      if (Array.isArray(item)) {
        const filteredArray = item
          .map((subItem, index) => filterRecursive(subItem, `[${index}]`))
          .filter(subItem => subItem !== undefined);

        // Include array if it has matches or if parent key matches
        return filteredArray.length > 0 || matchesSearch(item, parentKey)
          ? filteredArray
          : undefined;
      }

      if (typeof item === 'object') {
        const filteredObj: any = {};
        let hasMatches = false;

        for (const [key, value] of Object.entries(item)) {
          // Check if this key-value pair should be included
          if (matchesSearch(value, key)) {
            filteredObj[key] = value;
            hasMatches = true;
          } else {
            // Recursively filter nested objects
            const filteredValue = filterRecursive(value, key);
            if (filteredValue !== undefined) {
              filteredObj[key] = filteredValue;
              hasMatches = true;
            }
          }
        }

        // Include object if it has matches or if parent key matches
        return hasMatches || matchesSearch(item, parentKey) ? filteredObj : undefined;
      }

      return matchesSearch(item, parentKey) ? item : undefined;
    }

    return filterRecursive(obj);
  }

  // Apply search filter to filtered data
  $: searchFilteredData = searchFilter(filteredData, searchTerm);

  // Convert search filtered data to formatted string
  $: dataString =
    typeof searchFilteredData === 'string'
      ? searchFilteredData
      : JSON.stringify(searchFilteredData, null, 2);

  // Highlight code when data changes
  $: {
    if (dataString && Prism.languages[language]) {
      highlighted = Prism.highlight(dataString, Prism.languages[language], language);
      updateContainer();
    }
  }

  function toggleProperty(key: string, event: MouseEvent) {
    const isAltClick = event.altKey;

    appState.update(state => {
      if (!state.uiSettings) state.uiSettings = {};
      if (!state.uiSettings.debugPanelPrismDisplay) {
        state.uiSettings.debugPanelPrismDisplay = { frontend: {}, backend: {} };
      }
      const prismDisplay = state.uiSettings.debugPanelPrismDisplay as Record<string, any>;
      if (!prismDisplay[panelKey]) {
        prismDisplay[panelKey] = {};
      }

      const currentStates = prismDisplay[panelKey];

      if (isAltClick) {
        // Solo mode: hide all others, show only this one
        for (const k of topLevelKeys) {
          currentStates[k] = k === key;
        }
      } else {
        // Toggle mode: flip the state of this property
        currentStates[key] = !(currentStates[key] ?? true); // default to true if not set
      }

      return state;
    });
  }

  function isPropertyVisible(key: string): boolean {
    return toggleStates[key] !== false;
  }

  function toggleAll() {
    const allVisible = topLevelKeys.every(key => isPropertyVisible(key));
    const newState = !allVisible; // If all are visible, hide all; otherwise show all

    appState.update(state => {
      if (!state.uiSettings) state.uiSettings = {};
      if (!state.uiSettings.debugPanelPrismDisplay) {
        state.uiSettings.debugPanelPrismDisplay = { frontend: {}, backend: {} };
      }
      const prismDisplay = state.uiSettings.debugPanelPrismDisplay as Record<string, any>;
      if (!prismDisplay[panelKey]) {
        prismDisplay[panelKey] = {};
      }

      const currentStates = prismDisplay[panelKey];

      // Set all properties to the new state
      for (const key of topLevelKeys) {
        currentStates[key] = newState;
      }

      return state;
    });
  }

  function clearSearch() {
    searchTerm = '';
    searchInput?.focus();
  }

  function handleSearchKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      clearSearch();
    }
  }

  function updateContainer() {
    if (codeContainer && highlighted) {
      codeContainer.innerHTML = highlighted;
    }
  }

  // Ensure highlighting is applied after component updates
  afterUpdate(() => {
    updateContainer();
  });

  onMount(() => {
    updateContainer();
  });
</script>

<div class="prism-wrapper {className}">
  <!-- Search bar -->
  <div class="search-container">
    <div class="search-input-wrapper">
      <input
        type="text"
        bind:value={searchTerm}
        bind:this={searchInput}
        placeholder="Search by key name or value..."
        class="search-input"
        on:keydown={handleSearchKeydown}
      />
      {#if searchTerm}
        <button class="clear-search-btn" on:click={clearSearch} title="Clear search (Esc)">
          ✕
        </button>
      {/if}
    </div>
    {#if searchTerm}
      <div class="search-info">
        Searching for: "<span class="search-term">{searchTerm}</span>"
      </div>
    {/if}
  </div>

  <!-- Property toggles -->
  {#if topLevelKeys.length > 0}
    <div class="property-toggles">
      <div class="toggle-header">
        <div class="toggle-left">
          <span class="toggle-label">Properties:</span>
          <button class="toggle-all-btn" on:click={toggleAll}> Toggle All </button>
        </div>
        <span class="toggle-hint">Click to toggle, Alt+click to solo</span>
      </div>
      <div class="toggle-row">
        {#each topLevelKeys as key}
          <label class="toggle-item" class:visible={isPropertyVisible(key)}>
            <input
              type="checkbox"
              checked={isPropertyVisible(key)}
              on:click={e => toggleProperty(key, e)}
            />
            <span class="toggle-text">{key}</span>
          </label>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Code display -->
  <pre
    class="language-{language}"
    style="max-height: {maxHeight}; font-size: {fontSize}; line-height: {lineHeight};">
    <code class="language-{language}" bind:this={codeContainer}></code>
  </pre>
</div>

<style>
  .prism-wrapper {
    width: 100%;
  }

  /* Search bar styles */
  .search-container {
    margin-bottom: 8px;
  }

  .search-input-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-input {
    width: 100%;
    padding: 6px 10px;
    padding-right: 30px;
    font-size: 0.7rem;
    font-family: 'Fira Code', 'Courier New', monospace;
    background-color: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 4px;
    color: rgba(255, 255, 255, 0.9);
    outline: none;
    transition: all 0.2s ease;
  }

  .search-input:focus {
    border-color: rgba(48, 145, 241, 0.6);
    background-color: rgba(0, 0, 0, 0.4);
    box-shadow: 0 0 0 2px rgba(48, 145, 241, 0.2);
  }

  .search-input::placeholder {
    color: rgba(255, 255, 255, 0.4);
    font-style: italic;
  }

  .clear-search-btn {
    position: absolute;
    right: 6px;
    top: 50%;
    transform: translateY(-50%);
    background: none;
    border: none;
    color: rgba(255, 255, 255, 0.6);
    cursor: pointer;
    font-size: 0.7rem;
    padding: 2px;
    border-radius: 2px;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
  }

  .clear-search-btn:hover {
    background-color: rgba(255, 255, 255, 0.1);
    color: rgba(255, 255, 255, 0.9);
  }

  .clear-search-btn:active {
    background-color: rgba(255, 255, 255, 0.2);
  }

  .search-info {
    margin-top: 4px;
    font-size: 0.6rem;
    color: rgba(255, 255, 255, 0.6);
    font-style: italic;
  }

  .search-term {
    color: rgba(48, 145, 241, 0.8);
    font-weight: 600;
    font-family: 'Fira Code', monospace;
  }

  /* Property toggles */
  .property-toggles {
    margin-bottom: 8px;
    padding: 8px;
    background-color: rgba(0, 0, 0, 0.2);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
  }

  .toggle-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 6px;
    font-size: 0.7rem;
  }

  .toggle-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .toggle-label {
    font-weight: 600;
    color: rgba(255, 255, 255, 0.9);
  }

  .toggle-all-btn {
    background: rgba(48, 145, 241, 0.2);
    border: 1px solid rgba(48, 145, 241, 0.4);
    color: rgba(255, 255, 255, 0.9);
    font-size: 0.6rem;
    padding: 2px 6px;
    border-radius: 3px;
    cursor: pointer;
    transition: all 0.2s ease;
    font-family: inherit;
  }

  .toggle-all-btn:hover {
    background: rgba(48, 145, 241, 0.3);
    border-color: rgba(48, 145, 241, 0.6);
  }

  .toggle-all-btn:active {
    background: rgba(48, 145, 241, 0.4);
    transform: translateY(1px);
  }

  .toggle-hint {
    color: rgba(255, 255, 255, 0.5);
    font-style: italic;
  }

  .toggle-row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .toggle-item {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 0.65rem;
    color: rgba(255, 255, 255, 0.6);
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 3px;
    transition: all 0.2s ease;
    background-color: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .toggle-item.visible {
    color: rgba(255, 255, 255, 0.9);
    background-color: rgba(48, 145, 241, 0.2);
    border-color: rgba(48, 145, 241, 0.4);
  }

  .toggle-item:hover {
    background-color: rgba(255, 255, 255, 0.1);
  }

  .toggle-item.visible:hover {
    background-color: rgba(48, 145, 241, 0.3);
  }

  .toggle-item input[type='checkbox'] {
    width: 12px;
    height: 12px;
    margin: 0;
    accent-color: #3091f1;
  }

  .toggle-text {
    font-family: 'Fira Code', monospace;
    font-weight: 500;
  }

  /* Code display */
  pre {
    background-color: rgba(0, 0, 0, 0.3) !important;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    margin: 0;
    overflow-y: auto;
    overflow-x: auto;
  }

  code {
    font-family: 'Fira Code', 'Courier New', monospace;
    display: block;
    padding: 16px;
    white-space: pre-wrap;
    word-break: break-word;
  }

  /* Custom scrollbar */
  pre::-webkit-scrollbar {
    width: 8px;
    height: 8px;
  }

  pre::-webkit-scrollbar-track {
    background: rgba(255, 255, 255, 0.1);
    border-radius: 4px;
  }

  pre::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.3);
    border-radius: 4px;
  }

  pre::-webkit-scrollbar-thumb:hover {
    background: rgba(255, 255, 255, 0.5);
  }

  /* Responsive adjustments */
  @media (max-width: 768px) {
    .search-input {
      font-size: 0.65rem;
      padding: 5px 8px;
      padding-right: 26px;
    }

    .clear-search-btn {
      width: 16px;
      height: 16px;
      font-size: 0.6rem;
      right: 5px;
    }

    .search-info {
      font-size: 0.55rem;
    }

    .toggle-header {
      flex-direction: column;
      align-items: flex-start;
      gap: 4px;
    }

    .toggle-left {
      flex-direction: column;
      align-items: flex-start;
      gap: 4px;
    }

    .toggle-hint {
      font-size: 0.6rem;
    }

    .toggle-all-btn {
      font-size: 0.55rem;
      padding: 1px 4px;
    }

    .toggle-item {
      font-size: 0.6rem;
      padding: 1px 4px;
    }

    .toggle-item input[type='checkbox'] {
      width: 10px;
      height: 10px;
    }

    pre {
      font-size: 0.6rem !important;
      line-height: 1.3 !important;
    }

    code {
      padding: 12px;
    }
  }
</style>
