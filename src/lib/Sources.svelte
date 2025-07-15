<script lang="ts">
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
  // import { faCaretDown, faCaretUp } from '@fortawesome/free-solid-svg-icons'
  import { stat } from '@tauri-apps/plugin-fs'
  import {
    appState,
    deleteSection,
    get_file_paths_in_folder,
    updatePath,
  } from "./state/state.svelte";
  import { toCssRgb } from "./utils/colors";
  import { onMount } from "svelte";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
    WebviewWindow.getCurrent()
    .once<null>("initialized", (event) => {})
    .then((v) => {
      console.log(v);
    });
 let container: HTMLElement;

  function getInputRects(): DOMRect[] {
    if (!container) return [];
    const inputs = container.querySelectorAll('input');
    console.log(inputs)
    return Array.from(inputs).map(input => input.getBoundingClientRect());
  }

  function isPointInRect(x: number, y: number, rect: DOMRect): boolean {
  return (
    x >= rect.left &&
    x <= rect.right &&
    y >= rect.top &&
    y <= rect.bottom
  );
}

  let test;
  let rects;
  let inputsUnderMouse: number[] = [];
  let isOver;
  let x;
  let y;
  let scaleFactor = 1;


  onMount(async () => {
    const view = getCurrentWebview();
    await view.onDragDropEvent((event) => {
      console.log(event)
      rects = getInputRects();
      inputsUnderMouse = [];
      const factor = view.window.scaleFactor();
      factor.then(f=>{
        console.log(f)
        scaleFactor = f;
      })
      switch (event.payload.type) {
        case 'enter':
          isOver = true;
        case 'over':
            x = (event.payload.position.x/scaleFactor).toString();
            y = (event.payload.position.y/scaleFactor).toString();
            rects.forEach((r,i)=>{
              if (isPointInRect(parseInt(x), parseInt(y), r)){
                console.log(`%cHERE LINE :67 %c`,'color: brown; font-weight: bold', '');
                
                inputsUnderMouse.push(i)
              }
            })
            test = event.payload.position.toJSON();
            console.log(event.payload.position);

            // logPositionInfo(view);
        case 'drop':
            let atDrop: number[] = []
            x = (event.payload.position.x/scaleFactor).toString();
            y = (event.payload.position.y/scaleFactor).toString();
            rects.forEach((r,i)=>{
              if (isPointInRect(parseInt(x), parseInt(y), r)){
                console.log(`%cHERE LINE :67 %c`,'color: brown; font-weight: bold', '');
                atDrop.push(i)
                inputsUnderMouse.push(i)
              }
            })
            test = event.payload.position.toJSON();
          if (event.payload.type === "drop"){
              console.log(event.payload.paths)
              const paths =  event.payload.paths;
              console.log(atDrop)
              if (atDrop.length>0){
              Promise.all(event.payload.paths.map(p=>stat(p))).then((v)=>{
                    console.log(v)
                    console.log(inputsUnderMouse)
                    v.forEach((v)=>{
                      if (v.isDirectory){
                          updatePath(atDrop[0], paths[0])
                      }
                    })
                })
                inputsUnderMouse = [];
              }
    
          }
          break;
        case 'leave':
           isOver = false;
          console.log("No position data");
          break;
      }
    });
  });


</script>


<div class="position-relative">
    <div class="table-responsive" style:min-height="100px" style:background-color="rgb(15 21 27)">
        {#if $appState.sections.length === 0}
          <div class="position-absolute no-inputs-warning">No inputs!</div>
        {/if}
        <table class="w-100 table m-0" >
          <thead>
            <tr>
              <th class="file-column">Source</th>
              <th class="file-column text-center">Samples</th>
              <th class="file-column text-center">Actions</th>
            </tr>
          </thead>
          <tbody bind:this={container}>
            {#each $appState.sections as item, sectionIndex}
              <tr class="source-row">
                <!-- <tr style:background-color={toCssRgb(item.color.rgb, 0.5)}> -->
                <td>
                  <div class:under-drag={inputsUnderMouse.includes(sectionIndex)} class="d-flex justify-content-start align-items-center">
                    <i class="fas fa-folder my-0 mx-2"></i>
    
                    <input
                      style:color={toCssRgb(item.color.rgb, 1)}
                      class="folder-input input-group-sm my-auto "
                      onchange={(e) => {
                        updatePath(sectionIndex, (e.target as HTMLInputElement).value);
                      }}
                      bind:value={item.folderPath}
                      type="text"
                      id="name"
                      placeholder="Enter your name"
                    />
                  </div>
                  <div class="d-flex"></div>
                </td>
                <td>
                  <!-- <div style:color={"red"} class="stat">Samples: {item.files.length}</div> -->
                  <div class="stat text-center">
                    {item.files.length}
                  </div>
                </td>
                <td>
                  <button
                    class="text-danger"
                    onclick={() => deleteSection(sectionIndex)}
                  >
                    X
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
    </div>
</div>
<!-- <div style:font-size="9px">
  x: {x}
  y: {y}
  {JSON.stringify(isOver)}
  {JSON.stringify(test)}
  {JSON.stringify(rects)}
  {JSON.stringify(inputsUnderMouse)}
</div> -->
<style>
 
.source-row{
  border-bottom: 1px solid white;
}
 .under-drag{
    border: 2px solid green;
  }
  .folder-input {
    width: 500px;
    border-radius: 2px;
    border: 0px;
    /* background: var(--bs-primary-bg-subtle) !important; */
    /* background-color: var(--bs-primary-bg-subtle) !important; */
  }

  .folder-input,
  td {
    font-size: 13px;
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

  td {
    background-color: var(--bs-primary-bg-subtle) !important;
    /* background-color: #181c20 !important; */
    padding: 0px !important;
    font-size: 12px;
  }

    .no-inputs-warning{
    position: absolute;
    top: 65%;
    left: 50%;
    transform: translate(-50%, -50%);
    font-size: 12px;

  }
</style>
