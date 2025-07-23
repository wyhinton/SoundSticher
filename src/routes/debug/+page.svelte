<script lang="ts">
  import { performanceStore, resetPerformance, type PerformanceMetric } from "$lib/state/performance";
  import { addNewFolderOnDrop, positionStore } from "$lib/state/position";
  import { appState, resetAppState } from "$lib/state/state.svelte";
  import { json } from "@sveltejs/kit";
  import { BaseDirectory, create } from "@tauri-apps/plugin-fs";
  import Prism from "prismjs";
  import "prismjs/components/prism-json";
  import clipboard from "tauri-plugin-clipboard-api";

  //   import 'prismjs/themes/prism.css';
  import "prismjs/themes/prism-okaidia.css";
  import { derived, get } from "svelte/store";
  import { toSource } from "$lib/utils/format";
  import { examples } from "$lib/utils/examples";
  let highlighted = "";
  let appStateContainer: HTMLElement;

  // Reactive derived state for simplified display
  $: forPrint = {
    ...$appState,
    sections: $appState.sections.map((s) => ({
      folderPath: s.folderPath,
      files: s.files.length,
    })),
  };

  // Reactive highlight when `forPrint` changes
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

  // const saveStateToJson = () =>{
  //   console.log(get(appState));
  //   const file = create('TEST_THING.txt', { baseDir: BaseDirectory.Desktop }).then(f=>{
  //     console.log(`%cHERE LINE :39 %c`,'color: yellow; font-weight: bold', '');

  //     f.write(new TextEncoder().encode(J)).then(()=>{
  //       console.log(`%cHERE LINE :40 %c`,'color: brown; font-weight: bold', '');

  //       f.close()
  //     })
  //   })

  // }

  async function copyStateToClipboard() {
    return await clipboard.writeText(toSource(get(appState)));
  }

  const applyExampleState = (k: string) =>{
    appState.set(examples[k])
  }

  const sortedPerformance = derived(performanceStore, $store => {
    return Object.entries($store).sort((a, b) =>{
      const lastA = a[1][a[1].length-1];
      const lastB = b[1][b[1].length-1];
      return lastB.timeStamp - lastA.timeStamp; 
    });
  });

  let selectedKey = Object.keys(examples)[0]; // default selection
</script>

<div>
  <button
    on:click={() => {
      resetPerformance();
    }}
    class="btn btn-sm">Reset Performance</button
  >
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
  <div>
    <b>Is Over Table Container: </b>{$positionStore.isOverTableContainer}
  </div>
  <div><b>Inputs under mouse: </b>{$positionStore.inputsUnderMouse}</div>
  <div><b>Add new folder on drop: </b>{$addNewFolderOnDrop}</div>
  <div>{JSON.stringify($positionStore)}</div>
  <div>{t}</div>
</div>

<style>
  pre.language-json {
    font-size: 0.5rem;
    line-height: 1.4;
  }

  td,
  th {
    font-size: 10px;
  }
</style>
