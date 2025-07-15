<script lang="ts">
  import { appState } from "./state/state.svelte";
  import TimeRuler from "./TimeRuler.svelte";


</script>  
  
  
  {#if $appState.combinedFile}
    <div class="p-2">
        <svg class="waveform-container" width="'100%'" height="100" viewBox="0 0 1000 100">
          <path d={$appState.combinedFile.svgPath} stroke="#62c462" fill="none" stroke-width="1" />
        </svg>
    </div>
    <TimeRuler></TimeRuler>   
  {/if}


<style>
      .waveform-container{
    /* position: fixed;
    bottom: 0; */
    background-color: black;
  }
</style>