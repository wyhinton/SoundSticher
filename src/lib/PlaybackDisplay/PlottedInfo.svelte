<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import Progress from '../Progress.svelte';
  import TransportControls from './TransportControls.svelte';

  import { appState } from '../state/state.svelte';
  import TimeDisplay from './TimeDisplay.svelte';
  import TimelineInfo from './TimelineInfo.svelte';

  // Props to control which system to use
  export let useOperationSystem = true;
</script>

<div class="d-flex flex-column text-success">
  <Progress value={$appState.combineAudioFileProgress}></Progress>
  <div class="d-flex gap-1">
    <TransportControls {useOperationSystem} />
    <TimeDisplay {useOperationSystem} />
  </div>
  <TimelineInfo></TimelineInfo>
</div>

<style>
  div {
    font-size: 12px;
  }
</style>
