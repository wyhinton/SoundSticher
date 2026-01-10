<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import Progress from '../Progress.svelte';
  import TransportControls from './TransportControls.svelte';

  import TimeDisplay from './TimeDisplay.svelte';
  import TimelineInfo from './TimelineInfo.svelte';

  // For now, we'll show a static progress or could link to operation buffering
  // This was previously used for file combining progress in the legacy system
  let operationProgress = 1; // Set to 1 (100%) since operations are loaded instantly
</script>

<div class="d-flex flex-column text-success">
  <Progress value={operationProgress}></Progress>
  <div class="d-flex gap-1">
    <TransportControls />
    <TimeDisplay />
  </div>
  <TimelineInfo></TimelineInfo>
</div>

<style>
  div {
    font-size: 12px;
  }
</style>
