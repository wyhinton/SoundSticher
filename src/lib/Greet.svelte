<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { draggable, droppable, type DragDropState } from "@thisux/sveltednd";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { listen } from "@tauri-apps/api/event";
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { onMount, unmount } from "svelte";
  import FileDrop from "svelte-tauri-filedrop";
  import {
    addSection,
    appState,
    deleteSection,
    get_file_paths_in_folder,
    play_song,
    preferences,
    updatePath,
  } from "./state/state.svelte";
  import { get } from "svelte/store";

  WebviewWindow.getCurrent()
    .once<null>("initialized", (event) => {})
    .then((v) => {
      console.log(v);
    });

  // listen('tauri://drag-drop', event => {
  // console.log(event)
  // })
  // listen('tauri://drag-enter', event => {
  // console.log(event)
  // })
  // listen('tauri://drag-over', event => {
  // console.log(event)
  // })
  // listen('tauri://file-drop', event => {
  // console.log(event)
  // })

  listen<number>('song-progress', (event) => {
    console.log(
      event
    );
  });
  // let folders = $state<string[]>([]);
  let stateAsString = $derived(
    JSON.stringify(
      $appState.sections.map((s) => ({
        folderPath: s.folderPath,
        files: s.files.length,
      }))
    )
  );

  onMount(async () => {
    console.log(get(preferences));

    await getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "over") {
        console.log("User hovering", event.payload.position);
      } else if (event.payload.type === "drop") {
        console.log("User dropped", event.payload.paths);
      } else {
        console.log("File drop cancelled");
      }
    });
  });
</script>


<button class="btn btn-sm" onclick={addSection}>Add section</button>
<button class="btn btn-sm btn-secondary" onclick={() => play_song("test")}
  >PLAY SONG</button
>
{stateAsString}
<div class="">
  {#each $appState.sections as section, sectionIndex}
    <div
      class="card d-flex flex-column"
      class:error={$appState.sections[sectionIndex].errors.length > 0}
    >
      <div class="d-flex ">
        <input
          class="folder-input"
          onchange={(e) => {
            updatePath(sectionIndex, (e.target as HTMLInputElement).value);
          }}
          bind:value={$appState.sections[sectionIndex].folderPath}
          type="text"
          id="name"
          placeholder="Enter your name"
        />
		      <button
        class="btn btn-sm"
        >Get Files Test</button
      >
      <button
        class="btn btn-sm btn-danger"
        onclick={() => deleteSection(sectionIndex)}>Delete Section</button
      >
      </div>


      <div class="d-flex flex-column">
        <div class="d-flex flex-column">
          {#each $appState.sections[sectionIndex].errors as sectionError, errorIndex}
            {sectionError.message}
          {/each}
        </div>
        <div class="table-responsive section-table">
          <table class="table table-sm table-striped table-hover">
            <thead>
              <tr class="">
                <th class="file-column">File</th>
              </tr>
            </thead>
            <tbody>
              {#each $appState.sections[sectionIndex].files as file, fileIndex}
                <tr onclick={()=>play_song(file)}><td><div class="file-name">{file.split(/[/\\]/).pop()}</div></td></tr>
              {/each}
            </tbody>
          </table>
        </div>
      </div>

      <!-- ERRORS -->
    </div>
  {/each}
</div>

<style>
	.section-table{
		max-height: 400px;
	}
.file-column {
    max-width: 300px;
  }
  .dropzone {
    margin: 20px;
    padding: 20px;
    background: #eee;
  }
  .droppable {
    background: #d6dff0;
  }

  .error {
    border: 1px solid red;
    color: red;
  }

  .file-name {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    max-width: 400px;
  }

  th {
    text-align: left;
  }

  .folder-input {
    width: 500px;
  }

  tr {
  }
</style>
