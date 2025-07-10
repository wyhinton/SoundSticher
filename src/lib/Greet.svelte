<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { draggable, droppable, type DragDropState } from "@thisux/sveltednd";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { listen } from "@tauri-apps/api/event";
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { onMount, unmount } from "svelte";
  import FileDrop from "svelte-tauri-filedrop";
  import {TreeView} from 'svelte-tree-view'
  import DiffValue from "svelte-tree-view"
  import mapDocDeltaChildren from "svelte-tree-view"
  import {Fa} from 'svelte-fa'
// import { faCaretDown, faCaretUp } from '@fortawesome/free-solid-svg-icons'

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
  import Section from "./Section.svelte";

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


  // let folders = $state<string[]>([]);
  let stateAsString = $derived(
    JSON.stringify(
      {
        ...$appState,
        sections: $appState.sections.map((s) => ({
          folderPath: s.folderPath,
          files: s.files.length,
        })),
      
      }
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

  const theme = {
  scheme: 'google',
  author: 'seth wright (http://sethawright.com)',
  base00: '#1d1f21',
  base01: '#282a2e',
  base02: '#373b41',
  base03: '#969896',
  base04: '#b4b7b4',
  base05: '#c5c8c6',
  base06: '#e0e0e0',
  base07: '#ffffff',
  base08: '#CC342B',
  base09: '#F96A38',
  base0A: '#FBA922',
  base0B: '#198844',
  base0C: '#3971ED',
  base0D: '#3971ED',
  base0E: '#A36AC7',
  base0F: '#3971ED'
}
</script>


<button class="btn btn-sm" onclick={addSection}>Add section</button>
<button class="btn btn-sm" onclick={()=>console.log($appState)}>Log App State</button>

<div class="">
  {#each $appState.sections as section, sectionIndex}
    <Section section={$appState.sections[sectionIndex]} sectionIndex={sectionIndex}></Section>

  {/each}
</div>
<!-- <TreeView
  data={$appState}
  theme={theme}
  recursionOpts={{
    maxDepth: 16,
    shouldExpandNode: (n) =>{
      // console.log(n)
      if (["files", "metaData"].includes(n.key) ){
        // console.log(n)
        return false;
      }
      return true
    }
  }}
/> -->

{stateAsString}

<style>


  .dropzone {
    margin: 20px;
    padding: 20px;
    background: #eee;
  }
  .droppable {
    background: #d6dff0;
  }




  

</style>
