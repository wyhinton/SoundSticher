<script lang="ts">
  import {
    invokeWithPerf,
    performanceStore,
    resetPerformance,
    type PerformanceMetric,
  } from "$lib/state/performance";
  import { addNewFolderOnDrop, positionStore } from "$lib/state/position";
  import { addSection, appState, hoveredSourceItem, resetAppState } from "$lib/state/state.svelte";
  import Prism from "prismjs";
  import "prismjs/components/prism-json";
  import clipboard from "tauri-plugin-clipboard-api";

  import "prismjs/themes/prism-okaidia.css";
  import { derived, get } from "svelte/store";
  import { toSource } from "$lib/utils/format";
  import { examples } from "$lib/utils/examples";
  import { onDestroy, onMount } from "svelte";
  import { Channel, invoke } from "@tauri-apps/api/core";
  import type { CombineAudioEvent } from "$lib/state/events";
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

  function test_async(){
    invokeWithPerf("test_async")
  }

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

  const addTwoSections = () =>{
    addSection("C:\\Users\\Primary User\\Desktop\\AUDIO\\FREESOUNDS\\37427__dbs_sounds__foley")
    setTimeout(() => {
      addSection("C:\\Users\\Primary User\\Desktop\\AUDIO\\FREESOUNDS\\WOMB_VOX")
    }, 100);

  }

  const combineTest = () =>{
    const onCombineAudioEvent = new Channel<CombineAudioEvent>();
  
    onCombineAudioEvent.onmessage = (message) => {
      if (message.event === "started") {
        appState.update((state) => {
          state.isCombiningFile = true;
          state.combinedFileLength = message.data.duration;
          return state;
        });
      }
      if (message.event === "progress") {
            appState.update((s) => {
          s.combinedFile = { svgPath: message.data.svgPath };
          return s;
        });
      }
      if (message.event === "finished") {
        console.log(message);
        appState.update((s) => {
          s.isCombiningFile = false;
          s.combinedFile = { svgPath: message.data.svgPath };
          return s;
        });
        console.log(message.event);
      }
    };
    
    invokeWithPerf("combine_all_cached_samples", {onEvent: onCombineAudioEvent})
  }
  let intervalId: number;
  // Make sure to clear the waveform path on mount if no combined audio is present
  onMount(() => {
    let isFetching = false;

    // intervalId = setInterval(async () => {
    //   if (isFetching) return;
    //   isFetching = true;

    //   try {
    //     appStateDebug = await invokeWithPerf<AppStateDebug>("get_app_state");

    //     if (appStateDebug.combined_audio === "empty") {
    //       appState.update((s) => {
    //         if (s.combinedFile) s.combinedFile.svgPath = undefined;
    //         s.combinedFileLength = undefined;
    //         s.timelineItems = [];
    //         return s;
    //       });
    //     }
    //   } catch (err) {
    //     console.error("Failed to fetch app state", err);
    //   } finally {
    //     isFetching = false;
    //   }
    // }, 1000);
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
  <button
    on:click={() => {
      test_async();
    }}
    class="btn btn-sm">Test async</button
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
    <div>{$hoveredSourceItem}</div>
  <div>HoveredItem: {$hoveredSourceItem === null ? 'None' : $hoveredSourceItem}</div>
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
