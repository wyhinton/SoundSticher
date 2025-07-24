<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import Progress from "./Progress.svelte";

  import { appState, combiningAudio } from "./state/state.svelte";
  import { formatBytes, formatMilliseconds } from "./utils/format";

  let bufferingProgress = 0;

  listen<number>("buffering-progress", (e)=>{
    bufferingProgress = e.payload;
  })
</script>

<div class="d-flex flex-column text-success">

  <Progress value={$appState.combineAudioFileProgress}></Progress>
  <div class="d-flex justify-content-between">
    <div class="d-flex gap-2">
      <div>
        {#if $appState.playingCombined}
             <i class="fa-solid fa-play"></i>
        {:else}
         <i class="fa-solid fa-pause"></i>
        {/if}
      </div>
      <div class="d-flex gap-1">
        Length: <div
          class:skeleton={$combiningAudio}
        >
          {formatMilliseconds($appState.combineFileMeta.duration)}
        </div>
      </div>

      <div class="d-flex gap-1">
        Size: <div class:skeleton={$combiningAudio}>{formatBytes($appState.combineFileMeta.size)}</div>
      </div>
            <div class="d-flex">
        Buffering Status: {bufferingProgress.toFixed(2)}
      </div>
    </div>
        <div>
        <i class="fa-solid fa-repeat"></i>
      </div>
  </div>
</div>

<style>
  div {
    font-size: 12px;
  }
</style>
