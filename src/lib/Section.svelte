<script lang="ts">
  import { formatBytes, formatMilliseconds } from "./utils/format";
  import {
    appState,
    getAllFiles,
    hoveredSourceItem,
    hoveredTimelineItem,
    pause_song,
    play_song,
    setHoveredItem,
    setUnderMouse,
    type Section,
  } from "./state/state.svelte";

  export let sections: Section[];
</script>

<div style:width="-webkit-fill-available" class="card d-flex flex-column position-relative " >
  <div class="d-flex flex-column" style:background-color="#080808">
    <div class="d-flex flex-column">
      <!-- {#each section.errors as sectionError, errorIndex}
          {sectionError.message}
        {/each} -->
    </div>
    {#if sections.length === 0}
      <div class="position-absolute no-inputs-warning">No inputs</div>
    {/if}

    <div class="table-responsive section-table dot-grid-background">
      <table class="table table-xs border-0">
        <thead>
          <tr class="">
            <th class="file-column">File</th>
            <th class="text-center">Size</th>
            <th class="text-center">bitRate</th>
            <th class="text-center">Channels</th>
            <th class="text-center">bitDepth</th>
            <th class="text-center">Duration</th>
          </tr>
        </thead>
        <tbody>
          {#each getAllFiles(sections) as file, fileIndex}
            <tr
              onmouseenter={()=>{
                hoveredSourceItem.set(fileIndex)
                setHoveredItem(fileIndex)}
                }
              onmouseleave={()=>{
                setHoveredItem(null)}
                }
              class:timeline-hovered={$hoveredTimelineItem===fileIndex}
              class:playing={file.path === $appState.playingSong &&
                $appState.playProgress < 1}
              onclick={() => {
                if (file.path === $appState.playingSong &&
                $appState.playProgress < 1){
                   pause_song()
                } else {
                  console.log(`%cHERE LINE :47 %c`,'color: yellow; font-weight: bold', '');
                  
                   play_song(file.path)
                }
                }}
              ><td
                >
                <div class="d-flex align-items-center">
                  {$hoveredTimelineItem===fileIndex}
                <div
                  class="file-name ms-1"
                >
                  {file.path.split(/[/\\]/).pop()}
                </div>
                {#if file.path === $appState.playingSong &&
                $appState.playProgress < 1}
                  <i class="ms-1 fas fa-play text-success"></i>
                   <!-- content here -->
                {/if}
              <!-- <div class="color-indicator ms-1" style:background-color={toCssRgb(file.color.rgb, 1)}>
                </div> -->
                </div>
                
                </td
              >
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
  .color-indicator{
    height: 5px;
    width: 5px;
  }

  .no-inputs-warning{
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
  }
  .section-table {
    max-height: 400px;
    min-height: 400px;
  }

  .btn {
    height: min-content;
    padding: 0px;
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

  .audio-number {
    text-align: center;
  }

  .folder-input {
    width: 500px;
  }

  td {
    background-color: var(--bs-primary-bg-subtle) !important;
    /* background-color: #181c20 !important; */
    padding: 0px !important;
    font-size: 12px;
  }

  td > div > div{
    font-family: 'Fira Code'
  }

  tr:hover > td {
    background-color: transparent !important;
  }

  tr:hover {
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

  .timeline-hovered{
    color: red;
    border: 1px dotted white !important;
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
</style>
