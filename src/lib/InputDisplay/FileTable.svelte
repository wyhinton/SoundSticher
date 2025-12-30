<script lang="ts">
  import { formatBytes, formatMilliseconds } from '../utils/format';
  import {
    animatedIds,
    appState,
    getAllFiles,
    hoveredSourceItem,
    hoveredTimelineItem,
    pause_sample_preview,
    play_sample_preview,
    setHoveredItem,
    applySyncIndexes,
    type Section,
  } from '../state/state.svelte';
  import { generateProgressChannel, type SortAudioEvent } from '../state/events';
  import { Channel } from '@tauri-apps/api/core';
  import { invokeWithPerf, updateInputs } from '../state/performance';

  export let sections: Section[];

  // Local sorting function - moved from store
  function getSortedFiles(state: typeof $appState) {
    let files = getAllFiles(state.sections);

    // Always sort by index since index syncing updates the actual indices
    return files.sort((a, b) => a.index - b.index);
  }
  function toggleSort(key: keyof ReturnType<typeof getAllFiles>[0]) {
    if ($appState.sortKey === key) {
      appState.update(s => ({
        ...s,
        sortDirection: s.sortDirection === 'asc' ? 'desc' : 'asc',
      }));
    } else {
      appState.update(s => ({
        ...s,
        sortKey: key,
        sortDirection: 'asc',
      }));
    }

    // After updating sort, sync the indexes with the new sorted order
    setTimeout(() => {
      // Compute the sorted order based on the current sort key and direction
      let files = getAllFiles($appState.sections);

      // If no sort key is set, sort by index
      if (!$appState.sortKey) {
        files = files.sort((a, b) => a.index - b.index);
      } else {
        // Sort by the specified key and direction
        files = [...files].sort((a, b) => {
          let valA = a[$appState.sortKey!];
          let valB = b[$appState.sortKey!];

          if (typeof valA === 'string' && typeof valB === 'string') {
            return $appState.sortDirection === 'asc'
              ? valA.localeCompare(valB)
              : valB.localeCompare(valA);
          } else {
            return $appState.sortDirection === 'asc'
              ? (valA as number) - (valB as number)
              : (valB as number) - (valA as number);
          }
        });
      }

      console.log('FileTable sort - new order:', files);

      // Build array for Rust backend: { id, index }
      const updates = files.map((file, index) => ({
        id: file.id, // UUID string
        index,
      }));

      console.log('FileTable sort updates:', updates);

      const onEvent = generateProgressChannel<SortAudioEvent>(Channel, {
        started: () => {
          console.log('FileTable sort started');
        },
        progress: data => {
          console.log('FileTable sort progress:', data);
        },
        finished: () => {
          console.log('FileTable sort finished');
        },
      });

      invokeWithPerf<[string, number][]>('update_sorting', { updates, onEvent })
        .then(newOrder => {
          updateInputs($appState.sections);
          console.log('FileTable sort - received new order from backend:', newOrder);

          // Use the reusable index syncing function if newOrder has value
          if (newOrder.ok && newOrder.value) {
            applySyncIndexes(newOrder.value);
          }
        })
        .catch(error => {
          console.error('Failed to update sorting from FileTable:', error);
        });
    }, 10); // Small delay to ensure state update has propagated
  }
</script>

<div style:width="-webkit-fill-available" class="card d-flex flex-column position-relative">
  <div class="d-flex flex-column h-fill-available" style:background-color="#080808">
    <div class="d-flex flex-column"></div>
    {#if sections.length === 0}
      <div class="position-absolute no-inputs-warning">No inputs</div>
    {/if}

    <div class="table-responsive section-table dot-grid-background">
      <table class="table table-xs border-0 m-0">
        <thead>
          <tr class="">
            <th class="number-column"> # </th>
            {#snippet sortableHeader(key, label, classes = 'text-center')}
              <th class={classes} onclick={() => toggleSort(key)}>
                {label}
                {#if $appState.sortKey === key}
                  {#if $appState.sortDirection === 'asc'}
                    <span class="sort-arrow-active">▲</span>
                  {:else}
                    <span class="sort-arrow-active">▼</span>
                  {/if}
                {/if}
              </th>
            {/snippet}

            {@render sortableHeader('path', 'File', 'file-column')}
            {@render sortableHeader('size', 'Size')}
            {@render sortableHeader('bitRate', 'bitRate')}
            {@render sortableHeader('channels', 'Channels')}
            {@render sortableHeader('bitDepth', 'bitDepth')}
            {@render sortableHeader('duration', 'Duration')}
            {#if import.meta.env.DEV}
              <!-- <DevPanel /> -->
            {/if}
          </tr>
        </thead>
        <tbody>
          {#each getSortedFiles($appState) as file, fileIndex}
            <tr
              onmouseenter={() => {
                hoveredSourceItem.set(fileIndex);
                setHoveredItem(fileIndex);
              }}
              onmouseleave={() => {
                setHoveredItem(null);
              }}
              class:timeline-hovered={$hoveredTimelineItem === fileIndex}
              class:playing={file.path === $appState.playingSong && $appState.playProgress < 1}
              class:animated={$animatedIds.has(file.id)}
              class:inactive={file.active === false}
              data-file-id={file.id}
              data-file-active={file.active}
              data-file-path={file.path}
              onclick={() => {
                if (file.path === $appState.playingSong && $appState.playProgress < 1) {
                  pause_sample_preview();
                } else {
                  console.log(`%cHERE LINE :47 %c`, 'color: yellow; font-weight: bold', '');

                  play_sample_preview(file.path);
                }
              }}
            >
              <td>
                <div class="align-items-center text-center">{file.index}</div>
              </td>
              <td>
                <div class="d-flex align-items-center">
                  <div class="file-name ms-1">
                    {file.path.split(/[/\\]/).pop()}
                  </div>
                  {#if file.path === $appState.playingSong && $appState.playProgress < 1}
                    <i class="ms-1 fas fa-play text-success"></i>
                    <!-- content here -->
                  {/if}
                  <!-- <div class="color-indicator ms-1" style:background-color={toCssRgb(file.color.rgb, 1)}>
                </div> -->
                </div>
              </td>
              <td class="audio-number">{formatBytes(file.size)}</td>
              <td class="audio-number">{file.bitRate}</td>
              <td class="audio-number">{file.channels}</td>
              <td class="audio-number">{file.bitDepth}</td>
              <td class="audio-number">{formatMilliseconds(file.duration)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>

  <!-- ERRORS -->
</div>

<style>
  .dot-grid-background {
    background-image: radial-gradient(circle, #141313 1px, transparent 1px);
    background-size: 5px 5px;
  }
  .color-indicator {
    height: 5px;
    width: 5px;
  }

  .no-inputs-warning {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
  }
  .section-table {
    min-height: 200px;
  }

  th {
    text-align: left;
    padding-top: 0px !important;
    padding-bottom: 0px !important;
    position: sticky !important;
    top: 0;
    font-size: 11px;
    color: #9d9d9d !important;
    border-bottom: 0px !important;
  }

  .audio-number {
    text-align: center;
  }

  td {
    background-color: var(--bs-primary-bg-subtle) !important;
    /* background-color: #181c20 !important; */
    padding: 0px !important;
    font-size: 12px;
  }

  td > div > div {
    font-family: 'Fira Code';
  }

  tbody tr:hover > td {
    background-color: transparent !important;
  }

  tbody tr:hover {
    /* background: red !important; */
    /* background-color: red !important; */
    border: 1px dotted white;
    background: #3e3c4a;
    background: linear-gradient(
      90deg,
      rgba(62, 60, 74, 1) 0%,
      rgba(73, 73, 105, 1) 46%,
      rgba(0, 22, 120, 1) 100%
    );
  }

  td {
    padding-top: 2px;
    padding-bottom: 1px;
    border: 0px;
    color: #e8e8e8 !important;
    background-color: rgb(6, 5, 8) !important;
    border: 1px solid rgb(6, 5, 9) !important;
    white-space: nowrap;
    /* color: red !important;  */
  }

  tr {
    font-family: sans-serif;
  }

  .playing > td {
    background-color: transparent !important;
  }

  .playing {
    background: linear-gradient(
      90deg,
      rgba(62, 60, 74, 1) 0%,
      rgba(73, 73, 105, 1) 46%,
      rgba(0, 22, 120, 1) 100%
    );
    background-size: 200% 100%; /* makes it big enough to animate */
    background-position: 0% 0%;
    animation: gradientShift 1s linear infinite;
    border: 1px dotted white;
  }

  .timeline-hovered {
    color: red;
    border: 1px dotted white !important;
  }

  .animated {
    animation: positionChanged 2s ease-in-out;
  }

  @keyframes positionChanged {
    0% {
      background: #00ff00 !important;
      outline: 1px solid #00bfff;
    }
    50% {
      background: #66ff66 !important;
      outline: 1px solid #8edffa;
    }
    100% {
      background: transparent !important;
      outline: 1px solid rgb(6, 5, 9);
    }
  }

  @keyframes gradientShift {
    0% {
      background-position: 0% 0%;
    }
    100% {
      background-position: 100% 0%;
    }
  }

  .file-column {
    max-width: 300px;
    /* border-radius: 5px 0px 0px 0px; */
  }

  .file-name {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    max-width: 400px;
  }

  .error {
    border: 1px solid red;
    color: red;
  }

  input {
    height: 20px;
  }

  .sort-arrow-active {
    color: rgb(48, 145, 241) !important; /* Bootstrap blue color */
    font-weight: bold;
  }

  /* Inactive file styles */
  .inactive {
    opacity: 0.9;
    background-color: #2a2a2a !important;
  }

  .inactive > td {
    background-color: #2a2a2a !important;
    color: #666 !important;
    text-decoration: line-through;
  }

  .inactive:hover {
    opacity: 0.6;
    background: #4a4a4a !important;
  }

  .inactive:hover > td {
    background-color: #4a4a4a !important;
    color: #888 !important;
  }

  /* Ensure inactive state overrides other states when needed */
  .inactive.timeline-hovered {
    border: 1px dotted #666 !important;
  }

  .inactive.animated {
    animation: none; /* Disable animation for inactive files */
  }
</style>
