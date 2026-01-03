<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  export let target: HTMLElement | string = document.body;

  let mountEl: HTMLDivElement;
  let targetEl: HTMLElement;

  onMount(() => {
    targetEl =
      typeof target === 'string' ? (document.querySelector(target) as HTMLElement) : target;

    (targetEl ?? document.body).appendChild(mountEl);
  });

  onDestroy(() => {
    mountEl?.remove();
  });
</script>

<div bind:this={mountEl}>
  <slot />
</div>
