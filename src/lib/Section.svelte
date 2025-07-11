<script lang="ts">
  import { formatBytes, formatMilliseconds } from "./utils/format";
  import {
    appState,
    deleteSection,
    getAllFiles,
    play_song,
    updatePath,
    type Section,
  } from "./state/state.svelte";
  import { toCssRgb } from "./utils/colors";
  import { getAbortSignal } from "svelte";

  export let sections: Section[];
</script>

<div class="card d-flex flex-column">
  <div class="d-flex flex-column">
    <div class="d-flex flex-column">
      <!-- {#each section.errors as sectionError, errorIndex}
          {sectionError.message}
        {/each} -->
    </div>
    <div class="table-responsive section-table">
      <table class="table table-xs border-0">
        <thead>
          <tr class="">
            <th class="file-column">File</th>
            <th class="file-column text-center">Size</th>
            <th class="file-column text-center">bitRate</th>
            <th class="file-column text-center">Channels</th>
            <th class="file-column text-center">bitDepth</th>
            <th class="file-column text-center">Duration</th>
          </tr>
        </thead>
        <tbody>
          {#each getAllFiles(sections) as file, fileIndex}
            <tr
              class:playing={file.path === $appState.playingSong &&
                $appState.playProgress < 1}
              onclick={() => play_song(file.path)}
              ><td
                ><div
                  style:background-color={toCssRgb(file.color.rgb, 0.1)}
                  class="file-name"
                >
                  {file.path.split(/[/\\]/).pop()}
                </div></td
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
  .section-table {
    max-height: 400px;
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
  }

  .audio-number {
    text-align: center;
  }

  .folder-input {
    width: 500px;
  }

  td {
    background-color: #181c20 !important;
    padding: 0px !important;
    font-size: 12px;
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
