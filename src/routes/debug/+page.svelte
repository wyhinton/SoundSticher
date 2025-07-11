<script lang="ts">
  import { appState } from "$lib/state/state.svelte";
  import Prism from 'prismjs';
  import 'prismjs/components/prism-json';
//   import 'prismjs/themes/prism.css';
import 'prismjs/themes/prism-okaidia.css';
  let highlighted = '';
  let appStateContainer: HTMLElement;

  // Reactive derived state for simplified display
  $: forPrint = {
    ...$appState,
    sections: $appState.sections.map((s) => ({
      folderPath: s.folderPath,
      files: s.files.length,
    })),
  };

  // Reactive highlight when `forPrint` changes
  $: {
    const json = JSON.stringify(forPrint, null, 2);
    highlighted = Prism.highlight(json, Prism.languages.json, 'json');
    if (appStateContainer) {
      appStateContainer.innerHTML = highlighted;
    }
  }
</script>

<pre class="language-json">
  <code class="language-json" bind:this={appStateContainer}></code>
</pre>

<style>
  pre.language-json {
    font-size: 0.5rem;
    line-height: 1.4;
  }
</style>
