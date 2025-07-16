<script lang="ts">
  import { onMount, tick } from 'svelte';

  export let fullPath = "/some/long/path/filename.txt";
  let isEditing = false;
  let inputEl: HTMLInputElement;

  function getFileName(path: string): string {
    return path.split(/[/\\]/).pop() || '';
  }

  function startEditing() {
    isEditing = true;
    tick().then(() => inputEl?.focus());
  }

  function stopEditing() {
    isEditing = false;
  }

  function handleInputChange(e: Event) {
    const newName = (e.target as HTMLInputElement).value;
    const parts = fullPath.split(/[/\\]/);
    parts[parts.length - 1] = newName;
    fullPath = parts.join('/');
    stopEditing();
  }
</script>

<style>
  .wrapper {
    position: relative;
    width: fit-content;
  }

  .input {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    width: 200px;
  }

  .display-input {
    pointer-events: auto;
  }

  .edit-input {
    pointer-events: auto;
  }
</style>

<div class="wrapper">
  {#if !isEditing}
    <input
      class="input display-input"
      readonly
      value={getFileName(fullPath)}
      on:click={startEditing}
    />
  {/if}

  {#if isEditing}
    <input
      bind:this={inputEl}
      class="input edit-input"
      value={fullPath }
      on:blur={handleInputChange}
      on:keydown={(e) => e.key === 'Enter' && handleInputChange(e)}
    />
  {/if}
</div>

