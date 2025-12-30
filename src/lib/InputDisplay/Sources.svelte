<script lang="ts">
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
  // import { faCaretDown, faCaretUp } from '@fortawesome/free-solid-svg-icons'
  import { stat } from '@tauri-apps/plugin-fs';
  import {
    addSource,
    appState,
    combine_audio_files,
    deleteSection,
    getAllFiles,
    updatePath,
    applySyncIndexes,
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
  import SineWaveShader from '../Examples/SineWaveShader.svelte';
  import EditableInput from './EditableInput.svelte';
  import SourceRow from './SourceRow.svelte';
  import SourceToolbar from './SourceToolbar.svelte';
  import Favorites from './Favorites.svelte';
  import { get } from 'svelte/store';
  import { generateProgressChannel, type SortAudioEvent } from '../state/events';
  import { Channel, invoke } from '@tauri-apps/api/core';
  import { invokeWithPerf, updateInputs, type Result } from '../state/performance';

  // Local sorting function - moved from store
  function getSortedFiles(state: typeof $appState) {
    let files = getAllFiles(state.sections);

    // If no sort key is set, return files sorted by index
    if (!state.sortKey) {
      return files.sort((a, b) => a.index - b.index);
    }

    // Sort by the specified key and direction
    let sorted = [...files].sort((a, b) => {
      let valA = a[state.sortKey!];
      let valB = b[state.sortKey!];

      if (typeof valA === 'string' && typeof valB === 'string') {
        return state.sortDirection === 'asc' ? valA.localeCompare(valB) : valB.localeCompare(valA);
      } else {
        return state.sortDirection === 'asc'
          ? (valA as number) - (valB as number)
          : (valB as number) - (valA as number);
      }
    });

    return sorted;
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
  let x;
  let y;
  let scaleFactor = 1;

  let lottieContainer: HTMLDivElement;
  let lottieSize = 150;

  // Local selection state
  let selectedRows: Set<number> = new Set();
  let lastSelectedIndex: number | null = null;

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
    for (let i = 0; i < $appState.sections.length; i++) {
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

    // Delete sections
    indicesToDelete.forEach(index => {
      deleteSection(index);
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
            console.log(event.payload.paths);
            const paths = event.payload.paths;
            console.log(atDrop);
            if (atDrop.length > 0) {
              Promise.all(event.payload.paths.map(p => stat(p))).then(v => {
                v.forEach(v => {
                  if (v.isDirectory) {
                    updatePath(atDrop[0], paths[0]);
                  }
                });
                positionStore.reset();
                clearUnderMouse();
              });
              inputsUnderMouse = [];
            }
            if (addNewFolderOnDrop && atDrop.length === 0) {
              addSource(paths);
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

  let prevSortKey: string | null = null;
  let prevSortDirection: 'asc' | 'desc' | null = null;
  let debounceTimeout: number | undefined;

  appState.subscribe($appState => {
    // Clear the previous timeout if it exists
    if (debounceTimeout) clearTimeout(debounceTimeout);

    debounceTimeout = window.setTimeout(() => {
      if (!$appState.sortKey || !$appState.sortDirection) return;

      // Only proceed if sortKey or sortDirection changed
      if ($appState.sortKey === prevSortKey && $appState.sortDirection === prevSortDirection) {
        return;
      }

      prevSortKey = $appState.sortKey;
      prevSortDirection = $appState.sortDirection;

      // Compute new sorted order
      const files = getSortedFiles($appState);

      // Build array for Rust: { id, index }
      const updates = files.map((file, index) => ({
        id: file.id, // UUID string
        index,
      }));

      console.log(updates);

      const onEvent = generateProgressChannel<SortAudioEvent>(Channel, {
        started: data => {
          console.log('STARTED SORT');
        },
        progress: data => {},
        finished: data => {
          console.log('FINISHED SORT');
        },
      });

      invokeWithPerf<[string, number][]>('update_sorting', { updates, onEvent })
        .then(newOrder => {
          updateInputs($appState.sections);
          console.log(newOrder);
          console.log(newOrder);
          // Use the reusable index syncing function if newOrder has value
          if (newOrder.ok && newOrder.value) {
            applySyncIndexes(newOrder.value);
            console.log(`%cHERE LINE :227 %c`, 'color: yellow; font-weight: bold', '');
          }
        })
        .catch(err => console.error('Tauri invoke failed', err));
    }, 100); // 100ms debounce
  });

  // Add tab state at the end of script section
  let activeTab: string = 'Global';
  let tabContentHeight: number = 120; // Default height in pixels
  let isResizing: boolean = false;

  function setActiveTab(tab: string) {
    activeTab = tab;
  }

  function handleResizeStart(event: MouseEvent) {
    event.preventDefault();
    isResizing = true;

    const startY = event.clientY;
    const startHeight = tabContentHeight;

    function handleMouseMove(e: MouseEvent) {
      if (!isResizing) return;

      const deltaY = e.clientY - startY;
      const newHeight = Math.max(80, Math.min(400, startHeight + deltaY)); // Min 80px, Max 400px
      tabContentHeight = newHeight;
    }

    function handleMouseUp() {
      isResizing = false;
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    }

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="position-relative"
  onclick={handleSourcePanelClick}
  onkeydown={handleKeyDown}
  tabindex="0"
  role="region"
  aria-label="Source sections"
>
  <div
    bind:this={tableContainer}
    class:drop-add={$addNewFolderOnDrop}
    class="table-responsive h-100 d-flex flex-column justify-content-between"
    style:background-color="rgb(15 21 27)"
    style:width="400px"
    id="sources-panel"
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

    <div>
      <!-- Tab navigation -->
      <div class="tab-navigation">
        <div
          class="tab"
          class:active={activeTab === 'Global'}
          onclick={() => setActiveTab('Global')}
        >
          Global
        </div>
        <div class="tab" class:active={activeTab === 'Group'} onclick={() => setActiveTab('Group')}>
          Group
        </div>
        <div
          class="tab"
          class:active={activeTab === 'Favorites'}
          onclick={() => setActiveTab('Favorites')}
        >
          Favorites
        </div>
      </div>
      <!-- Tab content - add your content here based on the active tab -->
      <div class="tab-content" style="height: {tabContentHeight}px;">
        {#if activeTab === 'Global'}
          <div class="tab-panel">
            <p>Global content goes here</p>
            <!-- Add your global content here -->
          </div>
        {/if}
        {#if activeTab === 'Group'}
          <div class="tab-panel">
            <p>Group content goes here</p>
            <!-- Add your group content here -->
          </div>
        {/if}
        {#if activeTab === 'Favorites'}
          <div class="tab-panel">
            <Favorites />
          </div>
        {/if}
      </div>
      <!-- Resize handle -->
      <div
        class="resize-handle"
        class:resizing={isResizing}
        onmousedown={handleResizeStart}
        role="separator"
        aria-label="Resize tab content"
      >
        <div class="resize-indicator"></div>
      </div>
    </div>
  </div>
</div>

<style>
  #lottie-container {
    opacity: 0.8;
  }
  .drop-add {
    border: 2px solid green;
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
    top: 80%;
    left: 50%;
    transform: translate(-50%, -150%);
    font-size: 12px;
    display: flex;
    flex-direction: column;
  }

  /* Tab styles */
  .tab-navigation {
    display: flex;
    background-color: rgb(15 21 27);
    padding: 0 8px;
    border-bottom: 1px solid #555;
    gap: 2px;
    height: 30px;
  }

  .tab {
    padding: 8px 16px;
    cursor: pointer;
    position: relative;
    background: #2a2a2a;
    border: 1px solid #555;
    border-bottom: none;
    color: #9d9d9d;
    font-size: 11px;
    transition: all 0.2s ease;
    border-radius: 6px 6px 0 0;
    margin-top: 4px;
    min-width: 70px;
  }

  .tab:hover {
    background-color: #3a3a3a;
    color: #fff;
    border-color: #666;
  }

  .tab.active {
    color: #fff;
    font-weight: bold;
    background-color: rgb(15 21 27);
    border-color: #777;
    margin-top: 0;
    padding-top: 8px;
    z-index: 1;
    position: relative;
  }

  .tab.active::after {
    content: '';
    position: absolute;
    bottom: -1px;
    left: 0;
    right: 0;
    height: 1px;
    background: rgb(15 21 27);
  }

  .tab-content {
    background-color: rgb(15 21 27);
    border-top: none;
    border-radius: 0 0 4px 4px;
    overflow-y: auto;
    resize: vertical;
    min-height: 80px;
    max-height: 400px;
  }

  .tab-panel {
    color: #9d9d9d;
    font-size: 12px;
    line-height: 1.4;
    height: 100%;
  }

  .tab-panel p {
    margin: 0 0 12px 0;
    color: #ccc;
  }

  .resize-handle {
    height: 8px;
    background-color: rgb(15 21 27);
    cursor: ns-resize;
    border-top: 1px solid #555;
    border-bottom: 1px solid #555;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background-color 0.2s ease;
  }

  .resize-handle:hover {
    background-color: #2a2a2a;
  }

  .resize-handle.resizing {
    background-color: #3a3a3a;
  }

  .resize-indicator {
    width: 40px;
    height: 2px;
    background-color: #666;
    border-radius: 1px;
    position: relative;
  }

  .resize-indicator::before {
    content: '';
    position: absolute;
    top: -2px;
    left: 0;
    right: 0;
    height: 2px;
    background-color: #666;
    border-radius: 1px;
  }

  .resize-indicator::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 0;
    right: 0;
    height: 2px;
    background-color: #666;
    border-radius: 1px;
  }
</style>
