<script lang="ts">
  import Prism from 'prismjs';
  import 'prismjs/components/prism-json';
  import 'prismjs/themes/prism-okaidia.css';
  import { onMount, afterUpdate } from 'svelte';

  export let data: any = {};
  export let language: string = 'json';
  export let maxHeight: string = '600px';
  export let fontSize: string = '0.7rem';
  export let lineHeight: string = '1.4';
  export let className: string = '';

  let codeContainer: HTMLElement;
  let highlighted = '';

  // Convert data to formatted string
  $: dataString = typeof data === 'string' ? data : JSON.stringify(data, null, 2);

  // Highlight code when data changes
  $: {
    if (dataString && Prism.languages[language]) {
      highlighted = Prism.highlight(dataString, Prism.languages[language], language);
      updateContainer();
    }
  }

  function updateContainer() {
    if (codeContainer && highlighted) {
      codeContainer.innerHTML = highlighted;
    }
  }

  // Ensure highlighting is applied after component updates
  afterUpdate(() => {
    updateContainer();
  });

  onMount(() => {
    updateContainer();
  });
</script>

<div class="prism-wrapper {className}">
  <pre
    class="language-{language}"
    style="max-height: {maxHeight}; font-size: {fontSize}; line-height: {lineHeight};">
    <code class="language-{language}" bind:this={codeContainer}></code>
  </pre>
</div>

<style>
  .prism-wrapper {
    width: 100%;
  }

  pre {
    background-color: rgba(0, 0, 0, 0.3) !important;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 6px;
    margin: 0;
    overflow-y: auto;
    overflow-x: auto;
  }

  code {
    font-family: 'Fira Code', 'Courier New', monospace;
    display: block;
    padding: 16px;
    white-space: pre-wrap;
    word-break: break-word;
  }

  /* Custom scrollbar */
  pre::-webkit-scrollbar {
    width: 8px;
    height: 8px;
  }

  pre::-webkit-scrollbar-track {
    background: rgba(255, 255, 255, 0.1);
    border-radius: 4px;
  }

  pre::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.3);
    border-radius: 4px;
  }

  pre::-webkit-scrollbar-thumb:hover {
    background: rgba(255, 255, 255, 0.5);
  }

  /* Responsive adjustments */
  @media (max-width: 768px) {
    pre {
      font-size: 0.6rem !important;
      line-height: 1.3 !important;
    }

    code {
      padding: 12px;
    }
  }
</style>
