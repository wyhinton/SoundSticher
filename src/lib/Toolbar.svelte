<script lang="ts">
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

  import {
    addSource,
    appState,
    combine_audio_files,
    getAllFiles,
    hoveredTimelineItem,
    hoveredSourceItem
  } from "./state/state.svelte";
  import type { Event, UnlistenFn } from "@tauri-apps/api/event";
  import { get } from "svelte/store";
  import { BaseDirectory, create } from "@tauri-apps/plugin-fs";
  
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


</div>