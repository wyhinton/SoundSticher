// import { listen } from "@tauri-apps/api/event";
// import { appState } from "./state.svelte";

// listen<number>("song-progress", (event) => {
//   appState.update((state) => {
//     state.playProgress = event.payload;
//     console.log(event);
//     return state;
//   });
// });


// interface CachedCombineResult{
//     svgPath: string,
//     duration: number,
// }

// listen<CachedCombineResult>("combined-cached", (event) => {
//   appState.update((state) => {
//     console.log(event);
//     console.log(`%cHERE LINE :21 %c`,'color: brown; font-weight: bold', '');
    
//     state.combinedFile = {
//       ...state.combinedFile,
//       svgPath: event.payload.svgPath,
//     }
//     state.combinedFileLength = event.payload.duration;
//     return state;
//   });
// });

// listen<number>("combine-audio-progress", (event) => {
//   appState.update((state) => {
//     console.log(event);
//     // state.playProgress = event.payload;
//     state.combineAudioFileProgress = event.payload;

//     //  getCurrentWindow().setProgressBar({
//     //   status: ProgressBarStatus.Normal,
//     //   progress: event.payload*100,
//     // });
//     return state;
//   });
// });
