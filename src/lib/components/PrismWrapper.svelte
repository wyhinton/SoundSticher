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

  // Convert filtered data to formatted string
  $: dataString =
    typeof filteredData === 'string' ? filteredData : JSON.stringify(filteredData, null, 2);

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
