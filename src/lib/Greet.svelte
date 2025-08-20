<script lang="ts">
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

  import { appState } from "./state/state.svelte";
  import Section from "./Section.svelte";
  import Plotted from "./Plotted.svelte";
  import Sources from "./Sources.svelte";
  import PlottedInfo from "./PlottedInfo.svelte";
  import type { Event, UnlistenFn } from "@tauri-apps/api/event";
  import Toolbar from "./Toolbar.svelte";
  import { onDestroy, onMount } from "svelte";
  import { invokeWithPerf } from "./state/performance";
  import Export from "./Export.svelte";
  import { exportState } from "./state/export";

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

  const handleSpaceBar = (ev: KeyboardEvent) => {
    if (ev.code === "Space") {
      ev.preventDefault(); // optional, if you want to prevent default scrolling
      console.log("Spacebar pressed");

      appState.update((s) => {
        s.playingCombined = !s.playingCombined;
        if (s.playingCombined) {
          invokeWithPerf("play_combined_audio");
        } else {
          invokeWithPerf("pause_combined_audio")
        }
        return s;
      });
      // Add your logic here
    }
  };

  onMount(() => {
    window.addEventListener("keyup", handleSpaceBar);
    exportState.update((s)=>{
      s.message = undefined;
      s.progress = undefined;
      s.error = undefined;
      return s;
    })
    
  });

  onDestroy(() => {
    window.removeEventListener("keyup", handleSpaceBar);
  });
</script>

<Toolbar></Toolbar>

<div class="">
  <div class="px-0 d-flex">
    <Sources></Sources>
    <!-- <div class="text-center pixel-font py-2"><b>$</b></div> -->
    <Section sections={$appState.sections}></Section>
  </div>
  <!-- <Waveform></Waveform> -->
  <PlottedInfo></PlottedInfo>
  <Plotted></Plotted>
  <Export></Export>
</div>

<style>
  .blender-icon > svg {
    height: 12px;
    width: 12px;
  }
</style>
