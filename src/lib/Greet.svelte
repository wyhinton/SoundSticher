<script lang="ts">
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

  import { appState } from './state/state.svelte';
  import Section from './InputDisplay/FileTable.svelte';
  import Plotted from './PlaybackDisplay/Timeline.svelte';
  import Sources from './InputDisplay/Sources.svelte';
  import PlottedInfo from './PlaybackDisplay/PlottedInfo.svelte';
  import type { Event, UnlistenFn } from '@tauri-apps/api/event';
  import Toolbar from './Toolbar.svelte';
  import { onDestroy, onMount } from 'svelte';
  import { invokeWithPerf, updateInputs } from './state/performance';
  import Export from './Export.svelte';
  import Footer from './Footer.svelte';
  import { exportState } from './state/export';
  import { get } from 'svelte/store';

  WebviewWindow.getCurrent()
    .once<null>('initialized', event => {})
    .then(v => {
      console.log(v);
    });

  let filedropEvent: Event<any>;
  let unlisten: UnlistenFn;
  async function onDrop(event) {
    filedropEvent = event;
    if (!filedropEvent) return;
    console.log('ondrop', filedropEvent);
    unlisten();
  }

  const handleSpaceBar = (ev: KeyboardEvent) => {
    if (ev.code === 'Space') {
      ev.preventDefault(); // optional, if you want to prevent default scrolling
      console.log('Spacebar pressed');

      appState.update(s => {
        s.playingCombined = !s.playingCombined;
        if (s.playingCombined) {
          invokeWithPerf('play_timeline_audio', { startSeconds: 0 });
        } else {
          invokeWithPerf('pause_timeline_audio');
        }
        return s;
      });
    }
  };

  onMount(() => {
    window.addEventListener('keyup', handleSpaceBar);
    exportState.update(s => {
      s.message = undefined;
      s.progress = undefined;
      s.error = undefined;
      return s;
    });
    updateInputs(get(appState).sections);
  });

  onDestroy(() => {
    window.removeEventListener('keyup', handleSpaceBar);
  });
</script>

<!-- <Toolbar></Toolbar> -->

<div class="main-content d-flex flex-column">
  <div class="content-area flex-grow-1">
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
  <Footer></Footer>
</div>

<style>
  .blender-icon > svg {
    height: 12px;
    width: 12px;
  }

  .main-content {
    height: 100vh;
    overflow: hidden;
  }

  .content-area {
    overflow-y: auto;
    overflow-x: hidden;
  }
</style>
