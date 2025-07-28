<script lang="ts">
  import {
    invokeWithPerf,
    performanceStore,
    resetPerformance,
    type PerformanceMetric,
  } from "$lib/state/performance";
  import { addNewFolderOnDrop, positionStore } from "$lib/state/position";
  import { appState, resetAppState } from "$lib/state/state.svelte";
  import Prism from "prismjs";
  import "prismjs/components/prism-json";
  import clipboard from "tauri-plugin-clipboard-api";

  import "prismjs/themes/prism-okaidia.css";
  import { derived, get } from "svelte/store";
  import { toSource } from "$lib/utils/format";
  import { examples } from "$lib/utils/examples";
  import { onDestroy, onMount } from "svelte";
  let highlighted = "";
  let appStateContainer: HTMLElement;
  let appBackendState: HTMLElement;

  // Reactive derived state for simplified display
  $: forPrint = {
    ...$appState,
    sections: $appState.sections.map((s) => ({
      folderPath: s.folderPath,
      files: s.files.length,
      // files: s.files.length,
    })),
  };

  // FRONTEND JSON VISUALIZER
  $: {
    const json = JSON.stringify(forPrint, null, 2);
    highlighted = Prism.highlight(json, Prism.languages.json, "json");
    if (appStateContainer) {
      appStateContainer.innerHTML = highlighted;
    }
  }

  $: t = {
    x: JSON.stringify($positionStore),
  };

  async function copyStateToClipboard() {
    return await clipboard.writeText(toSource(get(appState)));
  }

  const applyExampleState = (k: string) => {
    appState.set(examples[k]);
  };

  const sortedPerformance = derived(performanceStore, ($store) => {
    return Object.entries($store).sort((a, b) => {
      const lastA = a[1][a[1].length - 1] ?? 0;
      const lastB = b[1][b[1].length - 1] ?? 0;
      return lastB.timeStamp - lastA.timeStamp;
    });
  });

  interface AppStateDebug {
    audio_files: { [key: string]: number };
    combined_audio: string;
    cancel_flag: boolean;
  }

  let appStateDebug: undefined | AppStateDebug = undefined;

  // BACKEND JSON VISUALIZER
  $: {
    if (appStateDebug) {
      const json = JSON.stringify(appStateDebug, null, 2);
      highlighted = Prism.highlight(json, Prism.languages.json, "json");
      if (appBackendState) {
        appBackendState.innerHTML = highlighted;
      }
    }
  }

  let intervalId: number;
  // Make sure to clear the waveform path on mount if no combined audio is present
  onMount(() => {
    let isFetching = false;

    intervalId = setInterval(async () => {
      if (isFetching) return;
      isFetching = true;

      try {
        appStateDebug = await invokeWithPerf<AppStateDebug>("get_app_state");

        if (appStateDebug.combined_audio === "empty") {
          appState.update((s) => {
            if (s.combinedFile) s.combinedFile.svgPath = undefined;
            s.combinedFileLength = undefined;
            return s;
          });
        }
      } catch (err) {
        console.error("Failed to fetch app state", err);
      } finally {
        isFetching = false;
      }
    }, 100);
  });

  onDestroy(() => {
    clearInterval(intervalId);
  });

  let selectedKey = Object.keys(examples)[0]; // default selection
</script>

<div>
  <button
    on:click={() => {
      resetAppState();
    }}
    class="btn btn-sm">Reset AppState</button
  >
  <button
    on:click={() => {
      copyStateToClipboard();
    }}
    class="btn btn-sm">Copy state to clipboard</button
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
  <pre class="language-json">
      <code class="language-json" bind:this={appStateContainer}></code>
    </pre>
  <pre class="language-json">
      <code class="language-json" bind:this={appBackendState}></code>
    </pre>
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
              <td class="text-center"
                >{value[value.length - 1].time.toFixed(2)}</td
              >
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
</style>
