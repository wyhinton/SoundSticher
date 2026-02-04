<script lang="ts">
  import { formatBytes, formatMilliseconds } from '../utils/format';
  import OperationArtifactsTable from './OperationArtifactsTable.svelte';
  import {
    animatedIds,
    appState,
    hoveredSourceItem,
    hoveredTimelineItem,
    pause_sample_preview,
    play_sample_preview,
    setHoveredItem,
    type Section,
  } from '../state/state.svelte';
  import { invoke } from '@tauri-apps/api/core';

  // Props
  export let operationId: string | undefined = undefined;

  // Derived file list from operation ID
  let currentFileList: string[] = [];

  // Define interface for file metadata from Tauri
  interface FileMetadata {
    path: string;
    size: number | null;
    bitRate: number | null;
    channels: number | null;
    bitDepth: number | null;
    duration: number;
  }

  // Store for file metadata
  let fileMetadata: Map<string, FileMetadata> = new Map();
  let metadataLoading = false;

  // Function to fetch metadata for current file list
  async function fetchMetadata() {
    if (metadataLoading || currentFileList.length === 0) return;

    metadataLoading = true;
    try {
      const metadataResults: FileMetadata[] = await invoke('get_metadata', {
        titles: currentFileList,
      });

      // Update our metadata map
      fileMetadata.clear();
      metadataResults.forEach(metadata => {
        fileMetadata.set(metadata.path, metadata);
      });

      // Trigger reactivity
      fileMetadata = new Map(fileMetadata);
    } catch (error) {
      console.error('Failed to fetch file metadata:', error);
    } finally {
      metadataLoading = false;
    }
  }

  // Local sorting function - now uses currentFileList with real metadata
  function getSortedFiles(state: typeof $appState) {
    return currentFileList.map((fileId, index) => {
      const metadata = fileMetadata.get(fileId);
      return {
        id: fileId,
        index: index,
        path: fileId,
        active: true,
        size: metadata?.size || 0,
        bitRate: metadata?.bitRate || 0,
        channels: metadata?.channels || 0,
        bitDepth: metadata?.bitDepth || 0,
        duration: metadata?.duration || 0,
        color: { rgb: { r: 128, g: 128, b: 128 } },
      };
    });
  }

  // Function to derive file list from operation ID
  function getFileListFromOperation(opId: string | undefined): string[] {
    if (!opId) return [];

    const operation = $appState.operations?.defs?.[opId];
    if (!operation || operation.kind !== 'merge') return [];

    const fileIds: string[] = [];

    // For each source in the MergeOp (which should be operation references)
    for (const source of operation.sources) {
      if (source.type === 'operation') {
        // Get the referenced SampleOp by its operationId
        const sampleOp = $appState.operations?.defs?.[source.operationId];
        if (sampleOp && sampleOp.kind === 'sample') {
          // Extract file IDs from the SampleOp's sources (should have one 'file' type source)
          for (const sampleSource of sampleOp.sources) {
            if (sampleSource.type === 'file') {
              fileIds.push(sampleSource.fileId);
            }
          }
        }
      }
    }

    return fileIds;
  }

  // Reactive statements
  $: currentFileList = getFileListFromOperation(operationId);

  $: if (currentFileList.length > 0) {
    fetchMetadata();
  }
</script>

<div class="w-fill-available card d-flex flex-column position-relative h-fill-available">
  <div class="d-flex flex-column h-fill-available" style:background-color="#080808">
    <!-- Inputs/Outputs Section -->
    <div class="d-flex flex-column"></div>
    {#if currentFileList.length === 0}
      <div class="position-absolute no-inputs-warning">No files in current operation</div>
    {:else if metadataLoading}
      <div class="position-absolute no-inputs-warning">
        <i class="fas fa-spinner fa-spin"></i> Loading file metadata...
      </div>
    {/if}

    <div class="table-responsive section-table dot-grid-background">
      <table class="table table-xs border-0 m-0">
        <thead>
          <tr class="">
            <th class="number-column"> # </th>
            {#snippet sortableHeader(key, label, classes = 'text-center')}
              <th class={classes}>
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

    <!-- Artifacts Section -->
    <OperationArtifactsTable {operationId} />
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
