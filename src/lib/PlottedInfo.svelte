<script lang="ts">
  import Progress from "./Progress.svelte";

  import { appState, combiningAudio } from "./state/state.svelte";
  import { formatBytes, formatMilliseconds } from "./utils/format";
</script>

<div class="d-flex flex-column text-success">
  <Progress value={$appState.combineAudioFileProgress}></Progress>
  <div class="d-flex gap-2">
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
  </div>
</div>

<style>
  div {
    font-size: 12px;
  }
</style>
