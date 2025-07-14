<script lang="ts">
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
  // import { faCaretDown, faCaretUp } from '@fortawesome/free-solid-svg-icons'

  import {
    appState,
    deleteSection,
    updatePath,
  } from "./state/state.svelte";
  import { toCssRgb } from "./utils/colors";
  import { onMount } from "svelte";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import type { DragDropEvent } from "@tauri-apps/api/webviewWindow";
  import { json } from "@sveltejs/kit";
  import { logPositionInfo } from "./state/position";
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
  let inputsUnderMouse = [];
  let innerPosition;
  let innerSize;
  let viewPosition;
  let viewSize;
  let isOver;
  let x;
  let y;


  onMount(async () => {
    const view = getCurrentWebview();
    await view.onDragDropEvent((event) => {
      console.log(event)
      rects = getInputRects();
      inputsUnderMouse = [];
      
      switch (event.payload.type) {
        case 'enter':
          isOver = true;
        case 'over':
            x = event.payload.position.x.toString();
            y = event.payload.position.y.toString();
            // innerPosition = view.position();
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
          const position = event.payload.position;
          // logPositionInfo(view);
          // console.log(rects)
          // console.log(view.position())
          // console.log(view.size)
          // console.log(view.window.innerPosition())
          // console.log(view.window.innerSize())
          // console.log({x: rects[0].x, y: rects[0].y, width: rects[0].width, height: rects[0].height})
          // const isInside = rects.filter(r=>isPointInRect((event.payload as DragDropEvent).))
          // rects.forEach((r,i)=>{
          //   if (isPointInRect(position.x, position.y, r)){
          //     inputsUnderMouse.push(i)
          //   }
          // })
          
          console.log(event.payload.position); // ✅ Safe
          break;
        case 'leave':
           isOver = false;
          console.log("No position data");
          break;
      }



      // if (event.payload.type === "over") {
      //   // console.log("User hovering", event.payload.position);
      // } else if (event.payload.type === "drop") {
      //   console.log(event)
      //   console.log("User dropped", event.payload.paths);
      // } else {
      //   console.log("File drop cancelled");
      // }
    });
  });


</script>


<div class="position-relative">
    <div class="table-responsive" style:min-height="50px" style:background-color="#32383e">
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
              <tr>
                <!-- <tr style:background-color={toCssRgb(item.color.rgb, 0.5)}> -->
                <td>
                  <div class="d-flex justify-content-start align-items-center">
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
<div style:font-size="9px">
  x: {x}
  y: {y}
  {JSON.stringify(isOver)}
  {JSON.stringify(test)}
  {JSON.stringify(rects)}
  {JSON.stringify(inputsUnderMouse)}
</div>
<style>
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
