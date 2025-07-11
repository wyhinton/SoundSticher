<script lang="ts">
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { onMount } from "svelte";
  // import { faCaretDown, faCaretUp } from '@fortawesome/free-solid-svg-icons'

  import {
    addSection,
    appState,
    combine_audio_files,
    deleteSection,
    getAllFiles,
    preferences,
    updatePath,
  } from "./state/state.svelte";
  import { get } from "svelte/store";
  import Section from "./Section.svelte";
  import { toCssRgb } from "./utils/colors";

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
    JSON.stringify({
      ...$appState,
      sections: $appState.sections.map((s) => ({
        folderPath: s.folderPath,
        files: s.files.length,
      })),
    })
  );

  const TEST_OUTPUT_DIR = 'C:\\Users\\Primary User\\Desktop\\TAURI_APPS\\SKV2\\tauri-v2-sveltekit-template\\assets\\test_output\\test.wav'

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
    scheme: "google",
    author: "seth wright (http://sethawright.com)",
    base00: "#1d1f21",
    base01: "#282a2e",
    base02: "#373b41",
    base03: "#969896",
    base04: "#b4b7b4",
    base05: "#c5c8c6",
    base06: "#e0e0e0",
    base07: "#ffffff",
    base08: "#CC342B",
    base09: "#F96A38",
    base0A: "#FBA922",
    base0B: "#198844",
    base0C: "#3971ED",
    base0D: "#3971ED",
    base0E: "#A36AC7",
    base0F: "#3971ED",
  };
</script>

<div class="d-flex justify-content-center p-2">
  <button class="btn btn-sm" onclick={addSection}>Add section</button>
  <button class="btn btn-sm" onclick={() => console.log($appState)}
    >Log App State</button
  >
  <button class="btn btn-sm" onclick={() => combine_audio_files(getAllFiles($appState.sections).map(f=>f.path),TEST_OUTPUT_DIR)}
    >Combine Files</button
  >
</div>

<div class="px-2">
  <table class="w-100">
    <thead>
      <tr>
        <th class="file-column">Source</th>
      </tr>
    </thead>
    <tbody>
      {#each $appState.sections as item, sectionIndex}
        <tr style:background-color={toCssRgb(item.color.rgb, 0.1)}>
          <td>
            <div class="d-flex justify-content-between">
              <div class="d-flex flex-column">
                <input
                  class="folder-input input-group-sm m-auto"
                  onchange={(e) => {
                    updatePath(
                      sectionIndex,
                      (e.target as HTMLInputElement).value
                    );
                  }}
                  bind:value={item.folderPath}
                  type="text"
                  id="name"
                  placeholder="Enter your name"
                />
              </div>
              <div class="d-flex">
                <button
                  class="btn btn-sm btn-danger"
                  onclick={() => deleteSection(sectionIndex)}
                >
                  <i class="blender-icon">
                    <svg
                      height="1000"
                      viewBox="0 0 1000 1000"
                      width="1000"
                      xmlns="http://www.w3.org/2000/svg"
                      xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape"
                      xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd"
                      ><sodipodi:namedview pagecolor="#303030" showgrid="true"
                        ><inkscape:grid
                          id="grid5"
                          units="px"
                          spacingx="100"
                          spacingy="100"
                          color="#4772b3"
                          opacity="0.2"
                          visible="true"
                        /></sodipodi:namedview
                      ><g fill="#fff"
                        ><path
                          d="m306.99023 241.72461a.66673335.66673335 0 0 0 -.65625.67578v5.93359h-5.93359a.66673335.66673335 0 1 0 0 1.33204h5.93359v5.93359a.66673335.66673335 0 1 0 1.33204 0v-5.93359h5.93359a.66673335.66673335 0 1 0 0-1.33204h-5.93359v-5.93359a.66673335.66673335 0 0 0 -.67579-.67578z"
                          transform="matrix(-53.033 -53.033 -53.033 53.033 29986.348 3575.914)"
                        /></g
                      ></svg
                    >
                  </i>
                </button>
                <!-- <div style:color={"red"} class="stat">Samples: {item.files.length}</div> -->
                <div style:color={toCssRgb(item.color.rgb)} class="stat">
                  {item.files.length}
                </div>
              </div>
            </div>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
	<div class="text-center"><b>↓</b></div>
  <Section sections={$appState.sections}></Section>
  {#if $appState.combinedFile}
    <svg class="waveform-container" width="'100%'" height="100" viewBox="0 0 1000 100">
      <path d={$appState.combinedFile.svgPath} stroke="#62c462" fill="none" stroke-width="1" />
    </svg>
  {/if}

</div>

<style>
  .waveform-container{
    /* position: fixed;
    bottom: 0; */
    background-color: black;
  }
  .folder-input {
    width: 500px;
  }

  .dropzone {
    margin: 20px;
    padding: 20px;
    background: #eee;
  }
  .droppable {
    background: #d6dff0;
  }

  .blender-icon > svg {
    height: 12px;
    width: 12px;
  }
</style>
