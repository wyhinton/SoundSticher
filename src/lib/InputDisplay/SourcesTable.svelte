<script lang="ts">
  import { tick } from 'svelte';
  import { appState, addSource } from '../state/state.svelte';
  import { addNewFolderOnDrop } from '../state/position';
  import SourceRow from './SourceRow.svelte';
  import lottie from 'lottie-web';

  // Props
  export let container: HTMLElement;
  export let tableContainer: HTMLElement;
  export let inputsUnderMouse: number[] = [];
  export let selectedRows: Set<number>;
  export let selectRow: (sectionIndex: number, isShiftSelect?: boolean) => void;
  export let toggleRowSelection: (sectionIndex: number) => void;

  let lottieContainer: HTMLDivElement;
  let lottieSize: number;

  // Reactive Lottie size based on available panel space
  $: {
    const tabContentHeight = $appState.uiSettings?.tabContentHeight || 120;
    // Calculate available height: assume table header takes ~40px, tab nav ~35px, some padding
    const tableHeaderHeight = 40;
    const tabNavHeight = 35;
    const padding = 40;
    const availableHeight = 400 - tabContentHeight - tableHeaderHeight - tabNavHeight - padding; // 400px is max panel height

    // Scale the lottie between 80px (minimum) and 150px (maximum) based on available space
    const minSize = 80;
    const maxSize = 150;
    const minAvailableHeight = 100;
    const maxAvailableHeight = 250;

    const clampedHeight = Math.max(
      minAvailableHeight,
      Math.min(maxAvailableHeight, availableHeight)
    );
    const sizeRatio =
      (clampedHeight - minAvailableHeight) / (maxAvailableHeight - minAvailableHeight);
    lottieSize = Math.round(minSize + (maxSize - minSize) * sizeRatio);
  }

  let animation;
  async function initLottie() {
    await tick(); // wait for DOM to update
    if (lottieContainer) {
      animation?.destroy(); // destroy previous animation if exists
      animation = lottie.loadAnimation({
        container: lottieContainer,
        renderer: 'svg',
        loop: true,
        autoplay: true,
        path: 'FOLDER_ANIM.json',
      });
    }
  }

  // Reactive statement to reinitialize Lottie when size changes
  $: if (lottieSize && lottieContainer) {
    initLottie();
  }
</script>

<div
  bind:this={tableContainer}
  class="table-responsive h-100 d-flex flex-column justify-content-between position-relative"
>
  {#if $appState.sections.length === 0 && !$addNewFolderOnDrop}
    <!-- <SineWaveShader></SineWaveShader> -->
    <div class="position-absolute no-inputs-warning">
      <div
        id="lottie-container"
        class="m-auto"
        style={`width: ${lottieSize}px; height: ${lottieSize}px;`}
        bind:this={lottieContainer}
      ></div>
      <div class="text-center">No inputs! Drag and Drop a folder of samples or add a section</div>
      <button class="btn btn-sm m-auto mt-2" onclick={() => addSource()}
        ><i class="me-1 fas fa-plus-circle text-success"></i>Add section</button
      >
    </div>
    {@html (() => {
      initLottie();
      return '';
    })()}
  {/if}
  {#if $addNewFolderOnDrop}
    <div class="position-absolute no-inputs warning">
      <i class="fa fas-plus">+</i>
    </div>
  {/if}

  {#if $appState.sections.length > 0}
    <!-- <SourceToolbar
    selectedRowCount={selectedRows.size}
    onSelectAll={handleSelectAll}
    onClearSelection={handleClearSelection}
    onDeleteSelected={handleDeleteSelected}
  /> -->
  {/if}

  <table class="w-100 table m-0">
    <thead>
      <tr>
        <th class="file-column">Source</th>
        <th class="file-column text-center">Samples</th>
        <th class="file-column text-center">Actions</th>
      </tr>
    </thead>
    <tbody bind:this={container}>
      {#each $appState.sections as item, sectionIndex}
        <SourceRow
          {item}
          {sectionIndex}
          {inputsUnderMouse}
          isSelected={selectedRows.has(sectionIndex)}
          onRowSelect={selectRow}
          onRowToggle={toggleRowSelection}
        />
      {/each}
    </tbody>
  </table>
</div>

<style>
  #lottie-container {
    opacity: 0.8;
  }

  th {
    text-align: left;
    padding-top: 0px !important;
    padding-bottom: 0px !important;
    position: sticky !important;
    top: 0;
    font-size: 11px;
    color: #9d9d9d !important;
  }

  .no-inputs-warning {
    position: absolute;
    top: 100%;
    left: 50%;
    transform: translate(-50%, -150%);
    font-size: 12px;
    display: flex;
    flex-direction: column;
  }
</style>
