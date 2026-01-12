<script lang="ts">
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
  // import { faCaretDown, faCaretUp } from '@fortawesome/free-solid-svg-icons'
  import { stat } from '@tauri-apps/plugin-fs';
  import {
    appState,
    currentOperationSources,
    addOperationSourceToCurrent,
    removeSourceFromCurrentOperation,
    addToFavorites,
  } from '../state/state.svelte';
  import { onMount, tick } from 'svelte';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import { isPointInRect } from '../utils/dragdrop';
  import lottie from 'lottie-web';

  import {
    addNewFolderOnDrop,
    clearUnderMouse,
    positionStore,
    setInputsUnderMouse,
    setIsOverTableContainer,
  } from '../state/position';
  import SourceRow from './SourceRow.svelte';
  import MainLeftPanel from './MainLeftPanel.svelte';
  import { generateProgressChannel, type SortAudioEvent } from '../state/events';
  import { Channel, invoke } from '@tauri-apps/api/core';
  import { invokeWithPerf, updateInputs, type Result } from '../state/performance';
  import type { OperationSource } from '../state/operation';
  import DropDownActionsButton from '../components/DropDownActionsButton.svelte';

  // Get sample op files for display
  function getSampleOpFiles(operationRef: string) {
    const operations = $appState.operations?.defs;
    if (!operations) return [];

    const sampleOp = operations[operationRef];
    if (!sampleOp || sampleOp.kind !== 'sample') return [];

    // For sample ops, there should be exactly one source with type 'file'
    if (sampleOp.sources.length > 0 && sampleOp.sources[0].type === 'file') {
      return [sampleOp.sources[0].fileId];
    }

    return [];
  }

  WebviewWindow.getCurrent()
    .once<null>('initialized', event => {})
    .then(v => {
      console.log(v);
    });

  function getInputRects(): DOMRect[] {
    if (!container) return [];
    const inputs = container.querySelectorAll('input');
    console.log(inputs);
    return Array.from(inputs).map(input => input.getBoundingClientRect());
  }

  function getSourceTableRect() {
    if (!tableContainer) return undefined;
    return tableContainer.getBoundingClientRect();
  }

  let container: HTMLElement;
  let tableContainer: HTMLElement;
  let isOverTableContainer: boolean = false;
  let rects;
  let inputsUnderMouse: number[] = [];

  let isOver;
  let x: string;
  let y: string;
  let scaleFactor = 1;

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

  // Local selection state
  let selectedRows: Set<number> = new Set();
  let lastSelectedIndex: number | null = null;
  const MAX_PANEL_HEIGHT = 800;
  function handleRowSelection(
    sectionIndex: number,
    isMultiSelect: boolean = false,
    isShiftSelect: boolean = false
  ) {
    if (isShiftSelect && lastSelectedIndex !== null) {
      // Shift-select: select range from lastSelectedIndex to sectionIndex
      const start = Math.min(lastSelectedIndex, sectionIndex);
      const end = Math.max(lastSelectedIndex, sectionIndex);

      // Add all indices in the range to selection
      for (let i = start; i <= end; i++) {
        selectedRows.add(i);
      }
    } else if (isMultiSelect) {
      // Toggle selection for multi-select
      if (selectedRows.has(sectionIndex)) {
        selectedRows.delete(sectionIndex);
      } else {
        selectedRows.add(sectionIndex);
      }
    } else {
      // Single selection - clear others and select this one
      selectedRows.clear();
      selectedRows.add(sectionIndex);
    }

    // Update last selected index for future shift-selects
    lastSelectedIndex = sectionIndex;

    // Trigger reactivity
    selectedRows = new Set(selectedRows);
  }

  function toggleRowSelection(sectionIndex: number) {
    handleRowSelection(sectionIndex, true);
  }

  function selectRow(sectionIndex: number, isShiftSelect: boolean = false) {
    handleRowSelection(sectionIndex, false, isShiftSelect);
  }

  // Toolbar functions
  function handleSelectAll() {
    selectedRows.clear();
    for (let i = 0; i < $currentOperationSources.length; i++) {
      selectedRows.add(i);
    }
    selectedRows = new Set(selectedRows);
  }

  function handleClearSelection() {
    selectedRows.clear();
    selectedRows = new Set(selectedRows);
    lastSelectedIndex = null;
  }

  function handleDeleteSelected() {
    if (selectedRows.size === 0) return;

    // Convert to array and sort in descending order to delete from end to start
    const indicesToDelete = Array.from(selectedRows).sort((a, b) => b - a);

    // Delete sources from current operation
    indicesToDelete.forEach(index => {
      removeSourceFromCurrentOperation(index);
    });

    // Clear selection
    handleClearSelection();
  }

  function handleSourcePanelClick(event: MouseEvent) {
    console.log(`%cHERE LINE :155 %c`, 'color: yellow; font-weight: bold', '');

    // Only deselect if clicking directly on the table element or tbody (empty space)
    if (
      event.target === event.currentTarget ||
      (event.target as HTMLElement).id === 'sources-panel' ||
      (event.target as HTMLElement).tagName === 'TABLE'
    ) {
      console.log(`%cHERE LINE :163 %c`, 'color: yellow; font-weight: bold', '');

      handleClearSelection();
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    // Check if Delete key was pressed and we have selected rows
    if (event.key === 'Delete' && selectedRows.size > 0) {
      // Don't trigger if user is typing in an input field
      if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) {
        return;
      }

      event.preventDefault();
      handleDeleteSelected();
    }
  }

  onMount(async () => {
    positionStore.reset();
    const view = getCurrentWebview();
    await view.onDragDropEvent(event => {
      rects = getInputRects();
      inputsUnderMouse = [];
      const factor = view.window.scaleFactor();
      factor.then(f => {
        console.log(f);
        scaleFactor = f;
      });
      switch (event.payload.type) {
        case 'enter':
          isOver = true;
        case 'over':
          x = (event.payload.position.x / scaleFactor).toString();
          y = (event.payload.position.y / scaleFactor).toString();
          let overEventUnderMouse = [];
          rects.forEach((r, i) => {
            if (isPointInRect(parseInt(x), parseInt(y), r)) {
              overEventUnderMouse.push(i);
              inputsUnderMouse.push(i);
            }
          });
          isOverTableContainer = isPointInRect(x, y, getSourceTableRect());
          setIsOverTableContainer(isOverTableContainer);
          setInputsUnderMouse(overEventUnderMouse);
        case 'drop':
          let atDrop: number[] = [];
          x = (event.payload.position.x / scaleFactor).toString();
          y = (event.payload.position.y / scaleFactor).toString();
          rects.forEach((r, i) => {
            if (isPointInRect(parseInt(x), parseInt(y), r)) {
              console.log(`%cHERE LINE :67 %c`, 'color: brown; font-weight: bold', '');
              atDrop.push(i);
              inputsUnderMouse.push(i);
            }
          });
          if (event.payload.type === 'drop') {
            console.log('Drop event detected:', event.payload.paths);
            const paths = event.payload.paths;
            const dropX = event.payload.position.x / scaleFactor;
            const dropY = event.payload.position.y / scaleFactor;

            // Use elementFromPoint to get the topmost element at drop coordinates
            const elementAtPoint = document.elementFromPoint(dropX, dropY);

            // Check if we're dropping on the favorites area or sources area
            const favoritesElement = elementAtPoint?.closest('.favorites-container');
            const sourcesElement = elementAtPoint?.closest('.sources-container');

            console.log('Drop coordinates:', { dropX, dropY });
            console.log('Element at point:', elementAtPoint);
            console.log('Is favorites area:', !!favoritesElement);
            console.log('Is sources area:', !!sourcesElement);

            if (atDrop.length > 0) {
              Promise.all(event.payload.paths.map(p => stat(p))).then(v => {
                v.forEach((filestat, index) => {
                  const path = paths[index];
                  if (filestat.isDirectory) {
                    // If dropped on favorites area, add to favorites
                    if (favoritesElement) {
                      console.log('Adding folder to favorites:', path);
                      addToFavorites(path);
                    } else {
                      // TODO: Implement drag-drop for operation sources
                      console.log('Drag-drop needs to be reimplemented for operation sources');
                    }
                  }
                });
                positionStore.reset();
                clearUnderMouse();
              });
              inputsUnderMouse = [];
            } else if (addNewFolderOnDrop) {
              // Handle drop when not over specific input areas
              Promise.all(event.payload.paths.map(p => stat(p))).then(v => {
                v.forEach((filestat, index) => {
                  const path = paths[index];
                  if (filestat.isDirectory) {
                    // If dropped on favorites area (even outside specific inputs), add to favorites
                    if (favoritesElement) {
                      console.log('Adding folder to favorites (general area):', path);
                      addToFavorites(path);
                    }
                  }
                });
              });
            }
            positionStore.reset();
            clearUnderMouse();
          }
          break;
        case 'leave':
          isOver = false;
          clearUnderMouse();
          positionStore.reset();
          console.log('No position data');
          break;
      }
    });
  });
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

  let prevSortKey: string | null = null;
  let prevSortDirection: 'asc' | 'desc' | null = null;
  let debounceTimeout: number | undefined;

  // appState.subscribe($appState => {
  //   // Clear the previous timeout if it exists
  //   if (debounceTimeout) clearTimeout(debounceTimeout);

  //   debounceTimeout = window.setTimeout(() => {
  //     if (!$appState.sortKey || !$appState.sortDirection) return;

  //     // Only proceed if sortKey or sortDirection changed
  //     if ($appState.sortKey === prevSortKey && $appState.sortDirection === prevSortDirection) {
  //       return;
  //     }

  //     prevSortKey = $appState.sortKey;
  //     prevSortDirection = $appState.sortDirection;

  //     // Compute new sorted order
  //     const files = getSortedFiles($appState);

  //     // Build array for Rust: { id, index }
  //     const updates = files.map((file, index) => ({
  //       id: file.id, // UUID string
  //       index,
  //     }));

  //     console.log(updates);

  //     const onEvent = generateProgressChannel<SortAudioEvent>(Channel, {
  //       started: data => {
  //         console.log('STARTED SORT');
  //       },
  //       progress: data => {},
  //       finished: data => {
  //         console.log('FINISHED SORT');
  //       },
  //     });

  //     invokeWithPerf<[string, number][]>('update_sorting', { updates, onEvent })
  //       .then(newOrder => {
  //         updateInputs($currentOperationSections);
  //         // Use the reusable index syncing function if newOrder has value
  //         if (newOrder.ok && newOrder.value) {
  //           // applySyncIndexes(newOrder.value);
  //           console.log(`%cHERE LINE :227 %c`, 'color: yellow; font-weight: bold', '');
  //         }
  //       })
  //       .catch(err => console.error('Tauri invoke failed', err));
  //   }, 100); // 100ms debounce
  // });

  // Add tab state at the end of script section

  // Selected operation state (bound from parent)
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="position-relative"
  onclick={handleSourcePanelClick}
  onkeydown={handleKeyDown}
  tabindex="0"
  role="region"
  aria-label="Operation sources"
>
  <div
    class="sources-container"
    class:drop-add={$addNewFolderOnDrop}
    style:background-color="rgb(15 21 27)"
    style:width="400px"
    id="sources-panel"
  >
    <!-- Main table section -->
    <section class="table-section">
      <div
        bind:this={tableContainer}
        class="table-responsive h-100 d-flex flex-column justify-content-between position-relative"
      >
        {#if $currentOperationSources.length === 0 && !$addNewFolderOnDrop}
          <!-- <SineWaveShader></SineWaveShader> -->
          <div class="position-absolute no-inputs-warning d-flex flex-column">
            <div
              id="lottie-container"
              class="m-auto"
              style={`width: ${lottieSize}px; height: ${lottieSize}px;`}
              bind:this={lottieContainer}
            ></div>
            <div class="text-center font-size-12px">
              No sources! Add sample operation sources to the current merge operation
            </div>
            <button
              class="btn btn-sm m-auto mt-2"
              onclick={() => {
                // TODO: Implement adding a new sample operation source
                console.log('Add source clicked - need to implement');
              }}><i class="me-1 fas fa-plus-circle text-success"></i>Add source</button
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

        <table class="w-100 table m-0">
          <thead>
            <tr>
              <th class="file-column">Source Operation</th>
              <th class="file-column text-center">File</th>
              <th class="file-column text-center">Actions</th>
            </tr>
          </thead>
          <tbody bind:this={container}>
            {#each $currentOperationSources as source, sourceIndex}
              {#if source.type === 'operation'}
                <tr class:table-warning={selectedRows.has(sourceIndex)}>
                  <td>
                    <small class="text-muted" title={source.operationRef}
                      >{source.operationRef}</small
                    >
                  </td>
                  <td class="text-center">
                    {#each getSampleOpFiles(source.operationRef) as fileId}
                      <small class="text-info" title={fileId}>{fileId}</small>
                    {/each}
                  </td>
                  <td class="text-center">
                    <DropDownActionsButton
                      dropdownId="source-actions-{sourceIndex}"
                      buttonTitle="Source actions"
                      buttonAriaLabel="Source actions for {source.operationRef}"
                      actions={[
                        {
                          id: 'remove',
                          label: 'Remove source',
                          icon: 'fa-times',
                          variant: 'danger',
                          onClick: () => removeSourceFromCurrentOperation(sourceIndex),
                        },
                      ]}
                    />
                  </td>
                </tr>
              {:else}
                <tr class:table-warning={selectedRows.has(sourceIndex)}>
                  <td colspan="3">
                    <small class="text-muted" title="Unsupported source type: {source.type}"
                      >Unsupported source type: {source.type}</small
                    >
                  </td>
                </tr>
              {/if}
            {/each}
          </tbody>
        </table>
      </div>
    </section>
  </div>
</div>

<style>
  .no-inputs-warning {
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
  }
  #lottie-container {
    opacity: 0.8;
  }
  .drop-add {
    border: 2px solid green;
  }

  /* Main layout containers */
  .sources-container {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .table-section {
    flex: 1;
    overflow: hidden;
  }

  th {
    font-size: 12px;
    max-width: 100px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Table cell styling for compact, non-wrapping text with ellipsis */
  td {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100px; /* Set max width to 100px for all columns */
    padding: 2px 8px !important; /* More compact padding */
    font-size: 11px; /* Slightly smaller font */
    line-height: 1.2; /* Tighter line height */
  }

  td small {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: block; /* Make sure the ellipsis works with small tags */
    max-width: 100%; /* Inherit parent width constraint */
  }
</style>
