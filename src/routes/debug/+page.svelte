<script lang="ts">
  import {
    exportAudio,
    invokeWithPerf,
    performanceStore,
    resetPerformance,
    type PerformanceMetric,
  } from '$lib/state/performance';
  import { addNewFolderOnDrop, positionStore } from '$lib/state/position';
  import { addSource, appState, hoveredSourceItem, resetAppState } from '$lib/state/state.svelte';
  import Prism from 'prismjs';
  import 'prismjs/components/prism-json';
  import clipboard from 'tauri-plugin-clipboard-api';

  import 'prismjs/themes/prism-okaidia.css';
  import { derived, get } from 'svelte/store';
  import { toSource } from '$lib/utils/format';
  import { examples } from '$lib/utils/examples';
  import { onDestroy, onMount } from 'svelte';
  import { Channel, invoke } from '@tauri-apps/api/core';
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import type { CombineAudioEvent, ExportAudioEvent } from '$lib/state/events';
  import { exportState } from '$lib/state/export';
  let highlighted = '';
  let appStateContainer: HTMLElement;
  let appBackendState: HTMLElement;
  let appExportState: HTMLElement;

  // Reactive derived state for simplified display
  $: forPrint = {
    ...$appState,
    sections: $appState.sections.map(s => ({
      folderPath: s.folderPath,
      files: s.files.length,
      // files: s.files.length,
    })),
  };

  // FRONTEND JSON VISUALIZER
  $: {
    const frontendStateJSON = JSON.stringify(forPrint, null, 2);
    highlighted = Prism.highlight(frontendStateJSON, Prism.languages.json, 'json');
    if (appStateContainer) {
      appStateContainer.innerHTML = highlighted;
    }
  }

  $: t = {
    x: JSON.stringify($positionStore),
  };

  let seconds = 0;
  let interval;

  onMount(() => {
    interval = setInterval(() => {
      seconds += 50;
    }, 50);

    // Cleanup when component is destroyed
    onDestroy(() => {
      clearInterval(interval);
    });
  });

  async function copyStateToClipboard() {
    return await clipboard.writeText(toSource(get(appState)));
  }

  const applyExampleState = (k: string) => {
    appState.set(examples[k]);
  };

  function test_async() {
    invokeWithPerf('test_async');
  }

  const openAudioFolder = () => {
    invokeWithPerf('open_in_explorer', {
      path: 'C:\\Users\\Primary User\\Desktop\\AUDIO',
    });
  };
  const testExport = () => {
    const s = get(exportState);
    console.log(s);
    exportAudio(
      s.settings,
      `C:\\Users\\Primary User\\Desktop\\AUDIO\\test_audio2.${s.settings.format.toLowerCase()}`
    );
  };
  const sortedPerformance = derived(performanceStore, $store => {
    return Object.entries($store).sort((a, b) => {
      const lastA = a[1][a[1].length - 1] ?? 0;
      const lastB = b[1][b[1].length - 1] ?? 0;
      return lastB.timeStamp - lastA.timeStamp;
    });
  });

  interface AppStateDebug {
    audio_files: { [key: string]: any };
    combined_audio: string;
    buffering_samples: boolean;
    svg_path: string;
    cancel_token: number;
    combine_process: number;
  }

  let appStateDebug: undefined | AppStateDebug = undefined;
  let backendHighlighted = '';

  // BACKEND JSON VISUALIZER
  $: {
    if (appStateDebug) {
      const backendStateJSON = JSON.stringify(appStateDebug, null, 2);
      backendHighlighted = Prism.highlight(backendStateJSON, Prism.languages.json, 'json');
      if (appBackendState) {
        appBackendState.innerHTML = backendHighlighted;
      }
    }
  }

  $: {
    const exportStateJSON = JSON.stringify(get(exportState), null, 2);
    console.log();
    highlighted = Prism.highlight(exportStateJSON, Prism.languages.json, 'json');
    if (appExportState) {
      appExportState.innerHTML = highlighted;
    }
  }

  const addTwoSections = () => {
    addSource('C:\\Users\\Primary User\\Desktop\\AUDIO\\FREESOUNDS\\37427__dbs_sounds__foley');
    setTimeout(() => {
      addSource('C:\\Users\\Primary User\\Desktop\\AUDIO\\FREESOUNDS\\WOMB_VOX');
    }, 100);
  };

  const combineTest = () => {
    const onCombineAudioEvent = new Channel<CombineAudioEvent>();

    onCombineAudioEvent.onmessage = message => {
      if (message.event === 'started') {
        appState.update(state => {
          state.isCombiningFile = true;
          state.combinedFileLength = message.data.duration;
          return state;
        });
      }
      if (message.event === 'progress') {
        appState.update(s => {
          s.combinedFile = { svgPath: message.data.svgPath };
          return s;
        });
      }
      if (message.event === 'finished') {
        console.log(message);
        appState.update(s => {
          s.isCombiningFile = false;
          s.combinedFile = { svgPath: message.data.svgPath };
          return s;
        });
        console.log(message.event);
      }
    };

    invokeWithPerf('combine_all_cached_samples', {
      onEvent: onCombineAudioEvent,
    });
  };

  let intervalId: number;
  let refreshBackendState = false; // Toggle for auto-refresh
  let isFetching = false;

  // Function to fetch backend state
  async function fetchBackendState() {
    if (isFetching) return;
    isFetching = true;

    try {
      let zDebug = await invokeWithPerf<AppStateDebug>('get_app_state');

      if (zDebug.ok) {
        appStateDebug = zDebug.value;
      }
    } catch (err) {
      console.error('Failed to fetch app state', err);
    } finally {
      isFetching = false;
    }
  }

  // Reactive statement to handle auto-refresh toggle
  $: {
    if (refreshBackendState) {
      // Start interval when toggle is enabled
      intervalId = setInterval(fetchBackendState, 1000);
    } else {
      // Clear interval when toggle is disabled
      if (intervalId) {
        clearInterval(intervalId);
      }
    }
  }

  onMount(() => {
    // Initial fetch
    fetchBackendState();
  });

  onDestroy(() => {
    clearInterval(intervalId);
  });

  let selectedKey = Object.keys(examples)[0]; // default selection

  const resetMainWindow = async () => {
    try {
      // Get reference to the main window by its label
      const mainWindow = await WebviewWindow.getByLabel('main');
      if (!mainWindow) {
        console.error('Main window not found');
        return;
      }
      // Close the main window
      await mainWindow.close();

      // Create a new main window with the same properties
      const newWindow = new WebviewWindow('main', {
        url: '/',
        title: 'Sound Stitch',
        width: 1200,
        height: 800,
        center: true,
        resizable: true,
        decorations: true,
      });

      // Wait for the new window to be ready
      await newWindow.once('tauri://created', () => {
        console.log('New main window created');
      });
    } catch (error) {
      console.error('Failed to reset main window:', error);
    }
  };
</script>

<div>
  <button
    on:click={() => {
      resetAppState();
    }}
    class="btn btn-sm"><i class="fa fa-arrows-spin"></i>Reset AppState</button
  >
  <button
    on:click={() => {
      resetMainWindow();
    }}
    class="btn btn-sm"><i class="fa fa-window-restore"></i>Reset Main Window</button
  >
  <button
    on:click={() => {
      console.log($appState);
    }}
    class="btn btn-sm"><i class="fa fa-arrows-spin"></i>Log AppState</button
  >
  <button
    on:click={() => {
      console.log($appState.sections);
    }}
    class="btn btn-sm"><i class="fa fa-arrows-spin"></i>Log Sections</button
  >
  <button
    on:click={() => {
      copyStateToClipboard();
    }}
    class="btn btn-sm"><i class="fa fa-clipboard"></i>Copy state to clipboard</button
  >

  <!-- Backend State Controls -->
  <div class="backend-controls">
    <label class="toggle-label">
      <input type="checkbox" bind:checked={refreshBackendState} />
      <i class="fa fa-sync-alt"></i> Refresh Backend State
    </label>
    {#if !refreshBackendState}
      <button on:click={fetchBackendState} class="btn btn-sm" disabled={isFetching}>
        <i class="fa {isFetching ? 'fa-spinner fa-spin' : 'fa-download'}"></i>
        {isFetching ? 'Fetching...' : 'Get Backend State'}
      </button>
    {/if}
  </div>
  <button
    on:click={() => {
      test_async();
    }}
    class="btn btn-sm">Test async</button
  >
  <button
    on:click={() => {
      testExport();
    }}
    class="btn btn-sm">Test Export</button
  >
  <button
    on:click={() => {
      openAudioFolder();
    }}
    class="btn btn-sm">Open Audio Folder</button
  >
  <select bind:value={selectedKey}>
    {#each Object.keys(examples) as key}
      <option value={key}>{key}</option>
    {/each}
  </select>
  <button
    on:click={() => {
      applyExampleState(selectedKey);
    }}
    class="btn btn-sm">Apply example state</button
  >
  <button
    on:click={() => {
      addTwoSections();
    }}
    class="btn btn-sm">Add two sections</button
  >
  <button
    on:click={() => {
      combineTest();
    }}
    class="btn btn-sm">Combine Test</button
  >
  <pre class="language-json">
      <code class="language-json" bind:this={appStateContainer}></code>
    </pre>
  <pre class="language-json">
      <code class="language-json" bind:this={appBackendState}></code>
    </pre>
  <pre class="language-json">
      <code class="language-json" bind:this={appExportState}></code>
    </pre>
  <div>{$hoveredSourceItem}</div>
  <div>
    HoveredItem: {$hoveredSourceItem === null ? 'None' : $hoveredSourceItem}
  </div>
  <div>{seconds}</div>
  <div>
    <div class="d-flex bg-black">
      <button
        on:click={() => {
          resetPerformance();
        }}
        class="btn btn-sm">Reset Performance</button
      >
    </div>
    <table>
      <thead>
        <tr>
          <th style:min-width="150px"> Metric </th>
          <th> Time </th>
          <th> Count </th>
        </tr>
      </thead>
      <tbody>
        {#each $sortedPerformance as [key, value]}
          <tr>
            <td><b>{key}</b></td>
            {#if value.length > 0}
              <td class="text-center">{value[value.length - 1].time.toFixed(2)}</td>
            {/if}
            <td class="text-center">{value.length}</td>
          </tr>
        {/each}
      </tbody>
    </table>

    <b>Is Over Table Container: </b>{$positionStore.isOverTableContainer}
  </div>
  <div><b>Inputs under mouse: </b>{$positionStore.inputsUnderMouse}</div>
  <div><b>Add new folder on drop: </b>{$addNewFolderOnDrop}</div>
  <div>{JSON.stringify($positionStore)}</div>
  <div>{t}</div>
  <div>{JSON.stringify(appStateDebug)}</div>
</div>

<style>
  pre.language-json {
    font-size: 0.7rem;
    line-height: 1.4;
  }

  td,
  th {
    font-size: 10px;
  }
  .btn {
    border: 1px solid white !important;
  }

  .backend-controls {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 10px 0;
    padding: 8px;
    background-color: rgba(255, 255, 255, 0.1);
    border-radius: 4px;
  }

  .toggle-label {
    display: flex;
    align-items: center;
    gap: 6px;
    color: white;
    font-size: 14px;
    cursor: pointer;
    user-select: none;
  }

  .toggle-label input[type='checkbox'] {
    margin: 0;
  }
</style>
