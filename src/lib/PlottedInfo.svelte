<script lang="ts">

  import {
    appState,
  } from "./state/state.svelte";
  import { formatBytes, formatMilliseconds } from "./utils/format";

</script>
<div class="d-flex gap-4 text-success px-1" 
style:border-top="3px solid var(--bs-success)"
style:background-color="rgb(6, 5, 9) !important;">
    <div>
        Length: {formatMilliseconds($appState.combineFileMeta.duration)}
    </div>
    <div>
        Size: {formatBytes($appState.combineFileMeta.size)}
    </div>
</div>

<style>

    div{
        font-size: 12px;
    }
</style>

