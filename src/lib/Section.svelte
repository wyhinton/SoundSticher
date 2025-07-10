<script lang="ts">
  import { formatBytes } from "./format";
  import {
    appState,
    deleteSection,
    play_song,
    updatePath,
    type Section,
  } from "./state/state.svelte";

  export let section: Section;
  export let sectionIndex: number;
</script>

  <div class="card d-flex flex-column" class:error={section.errors.length > 0}>
    <div class="d-flex">
      <div class="d-flex flex-column">
          <input
            class="folder-input input-group-sm"
            onchange={(e) => {
              updatePath(sectionIndex, (e.target as HTMLInputElement).value);
            }}
            bind:value={section.folderPath}
            type="text"
            id="name"
            placeholder="Enter your name"
          />
        
      </div>
      <button class="btn btn-sm">Get Files Test</button>
      <button
        class="btn btn-sm btn-danger"
        onclick={() => deleteSection(sectionIndex)}>Delete Section</button
      >
        <div class="stat">Samples: {section.files.length}</div>
    </div>

    <div class="d-flex flex-column">
      <div class="d-flex flex-column">
        {#each section.errors as sectionError, errorIndex}
          {sectionError.message}
        {/each}
      </div>
      <div class="table-responsive section-table">
        <table class="table table-xs border-0">
          <thead>
            <tr class="">
              <th class="file-column">File</th>
               <th class="file-column">Size</th>
            <th class="file-column">bitRate</th>
            </tr>
          </thead>
          <tbody>
            {#each section.files as file, fileIndex}
              <tr
                class:playing={file.path === $appState.playingSong && sectionIndex === $appState.playingSection  &&
                  $appState.playProgress < 1}
                onclick={() => play_song(file.path, sectionIndex)}
                ><td
                  ><div class="file-name">{file.path.split(/[/\\]/).pop()}</div></td
                >
                <td>{formatBytes(file.size)}</td>
                <td>{file.bitRate}</td>
                </tr
              >
            {/each}
          </tbody>
        </table>
      </div>
    </div>

    <!-- ERRORS -->
  </div>

<style>

 .btn{
    height: min-content;
    padding: 0px;
 }
  th {
    text-align: left;
  }

  .folder-input {
    width: 500px;
  }

  td{
    background-color: #181c20 !important;
  }

  tr:hover > td{
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
    /* color: red !important;  */
  }

  tr {
    font-family: sans-serif;
  }

  .playing > td{
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


  .section-table {
    max-height: 400px;
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

  input{
    height: 20px;
  }
</style>
