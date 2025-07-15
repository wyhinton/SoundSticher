<script lang="ts">
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

  import {
    addSection,
    appState,
    combine_audio_files,
    getAllFiles,
  } from "./state/state.svelte";
  import type { Event, UnlistenFn } from "@tauri-apps/api/event";
  
  WebviewWindow.getCurrent()
    .once<null>("initialized", (event) => {})
    .then((v) => {
      console.log(v);
    });

    let filedropEvent: Event<any>;
    let unlisten: UnlistenFn;
    async function onDrop(event) {
        filedropEvent = event;
        if (!filedropEvent) return;
        console.log("ondrop", filedropEvent);
        unlisten();
    }

  const TEST_OUTPUT_DIR = 'C:\\Users\\Primary User\\Desktop\\TAURI_APPS\\SKV2\\tauri-v2-sveltekit-template\\assets\\test_output\\test.wav'

</script>

<div class="d-flex justify-content-center p-2">
  <!-- <div class="pixel-font">!Test</div> -->
  <button class="btn btn-sm" onclick={addSection}><i class="me-1 fas fa-plus-circle text-success"></i>Add section</button>

  <button class:disabled={$appState.sections.length === 0} class="btn btn-sm" onclick={() => combine_audio_files(getAllFiles($appState.sections).map(f=>f.path),TEST_OUTPUT_DIR)}
    ><i class="me-1 fas fa-layer-group text-success"></i>Combine Files</button
  >
    <button class="btn btn-sm" onclick={() => console.log($appState)}
    >Log App State</button
  >
</div>