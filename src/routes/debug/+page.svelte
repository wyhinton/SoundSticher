<script lang="ts">
  import { performanceStore, resetPerformance } from "$lib/state/performance";
  import { addNewFolderOnDrop, positionStore } from "$lib/state/position";
  import { appState, resetAppState } from "$lib/state/state.svelte";
  import Prism from 'prismjs';
  import 'prismjs/components/prism-json';
//   import 'prismjs/themes/prism.css';
import 'prismjs/themes/prism-okaidia.css';
  let highlighted = '';
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
    highlighted = Prism.highlight(json, Prism.languages.json, 'json');
    if (appStateContainer) {
      appStateContainer.innerHTML = highlighted;
    }
  }

  $: t = {
    x: JSON.stringify($positionStore)
  }



</script>
<div>
    <button on:click={()=>{resetPerformance()}} class="btn btn-sm">Reset Performance</button>
    <button on:click={()=>{resetAppState()}} class="btn btn-sm">Reset AppState</button>
    <pre class="language-json">
      <code class="language-json" bind:this={appStateContainer}></code>
    </pre>
    <table>
        <thead>
            <tr>
                <th style:min-width="150px" >
                    Metric
                </th>
                <th>
                    Time
                </th>
                <th>
                    Count
                </th>
            </tr>
        </thead>
        <tbody>
     
            {#each Object.entries($performanceStore) as [key, value]}
            <tr>
                <td><b>{key}</b></td>
                {#if value.length > 0}
                <td class="text-center">{value[value.length-1].time.toFixed(2)}</td>
                {/if}
                <td  class="text-center">{value.length}</td>
            </tr>
            {/each}
        </tbody>
    </table>
    <div><b>Is Over Table Container: </b>{$positionStore.isOverTableContainer}</div>
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

  td, th{
    font-size: 10px;

  }
</style>
