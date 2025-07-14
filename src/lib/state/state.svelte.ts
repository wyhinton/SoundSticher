export const files = $state<string[]>([]);
import { persisted } from "svelte-persisted-store";
import { get } from "svelte/store";
import { listen } from "@tauri-apps/api/event";
import { getNextAvailableColor, type AbletonColor } from "$lib/utils/colors";
import { invokeWithPerf } from "./performance";

export type ErrorKind = {
  kind: "io" | "utf8";
  message: string;
};
// First param `preferences` is the local storage key.
// Second param is the initial value.
export const preferences = persisted("preferences", {
  theme: "dark",
  pane: "50%",
});

interface Song {
  title: String;
}

interface VisualSample {
  path: string;
  svgPath: string;
}
interface AppState {
  sections: Section[];
  playingSong?: string;
  playingSection?: number;
  playProgress?: number;
  combinedFile?: VisualSample;
  combineFileMeta?: FileMetadata
  isCombiningFile: boolean;
}

interface AudioFileItem {
  path: string;
  color: AbletonColor;
  size?: number;
  bitRate?: number;
  channels?: number;
  bitDepth?: number;
  duration?: number;
}

export interface Section {
  folderPath: string;
  files: AudioFileItem[];
  errors: ErrorKind[];
  metaData?: FileMetadata[];
  color: AbletonColor;
}

interface FileMetadata {
  path: string;
  size?: number;
  bitRate?: number;
  channels?: number;
  bitDepth?: number;
  duration: number;
}

export const appState = persisted<AppState>("appState", {
  sections: [],
  isCombiningFile: false,
});

const DEFAULT_FOLDER =
  "C:\\Users\\Primary User\\Desktop\\AUDIO\\FREESOUNDS\\_time-leeuwarden";

export function addSection() {
  appState.update((state) => {
    const color = getNextAvailableColor(state.sections);
    console.log(color);
    state.sections = [
      {
        folderPath: DEFAULT_FOLDER,
        files: [],
        errors: [],
        metaData: [],
        color: color,
      },
      ...state.sections,
    ];
    return state;
  });
  get_file_paths_in_folder(0);
}

export function deleteSection(index: number) {
  appState.update((state) => {
    // Remove the section at the specified index
    state.sections.splice(index, 1);
    return state;
  });
}

export function updatePath(sectionIndex: number, value: string) {
  appState.update((state) => {
    state.sections[sectionIndex].folderPath = value;
    return state;
  });
  get_file_paths_in_folder(sectionIndex);
}

export async function play_song(song: string) {
  await invokeWithPerf<Song[]>("play_song", { title: song }).then((f) => {
    appState.update((state) => {
      state.playingSong = song;
      return state;
    });
    console.log(f);
  });
}

export async function pause_song() {
  await invokeWithPerf<Song[]>("pause_song", {}).then((f) => {
    appState.update((state) => {
      state.playingSong = undefined;
      return state;
    });
  });
}

interface CombineAudioResult {
  output: string;
  svgPath: string;
}

export async function combine_audio_files(
  input_files: string[],
  output_path: string
) {
  await invokeWithPerf<CombineAudioResult>("combine_audio_files", {
    inputFiles: input_files,
    outputPath: output_path,
  }).then((f) => {
    const metadata = invokeWithPerf<FileMetadata>("get_metadata", {
            title: "C:\\Users\\Primary User\\Desktop\\TAURI_APPS\\SKV2\\tauri-v2-sveltekit-template\\assets\\test_output\\test.wav",
    }).then((m)=>{
      console.log(m)
      appState.update((state) => {
        state.combinedFile = { path: f.output, svgPath: f.svgPath };
        state.combineFileMeta = m;
        return state;
      });
      console.log(f);
    });
  });
}



export async function get_file_paths_in_folder(sectionIndex: number) {
  const { sections } = get(appState);
  const folder = sections[sectionIndex]?.folderPath;

  if (!folder) return;

  try {
    const files = await invokeWithPerf<string[]>("get_file_paths_in_folder", {
      folderPath: folder,
    });

    // Set file paths first
    appState.update((state) => {
      const section = state.sections[sectionIndex];
      section.files = files.map((f) => ({ path: f, color: section.color }));
      section.errors = section.errors.filter((e) => e.kind === "io");
      return state;
    });

    console.log(`Fetched files for section ${sectionIndex}:`, files);

    // Now fetch metadata for each file in parallel
    const metadataList = await Promise.all<FileMetadata | null>(
      files.map(async (file) => {
        try {
          const metadata = await invokeWithPerf<FileMetadata>("get_metadata", {
            title: file,
          });
          return metadata;
        } catch (err) {
          console.error(`Failed to get metadata for ${file}:`, err);
          return null;
        }
      })
    );

    // Store metadata in the section (you can customize this structure)
    appState.update((state) => {
      const section = state.sections[sectionIndex];
      state.sections.forEach((s, i) => {
        s.files.forEach((f, j) => {
          const meta = metadataList.filter((m) => m.path === f.path)[0];
          state.sections[i].files[j] = {
            ...f,
            bitRate: meta.bitRate,
            size: meta.size,
            channels: meta.channels,
            duration: meta.duration,
            bitDepth: meta.duration,
          };
        });
      });
      console.log(state.sections);
      section.metaData = metadataList;
      return state;
    });
  } catch (e: any) {
    console.error("Failed to fetch files:", e);

    appState.update((state) => {
      const section = state.sections[sectionIndex];
      section.errors.push({
        kind: "io",
        message: e.message || "Unknown error",
      });
      return state;
    });
  }
}

listen<number>("song-progress", (event) => {
  appState.update((state) => {
    state.playProgress = event.payload;
    return state;
  });
});

appState.subscribe((s) => {
  console.log(s);
});

export function resetAppState(){
  appState.update((state) => {
  state.combinedFile = undefined;
  state.sections = [];
  state.playingSong = undefined;
  state.playingSection = undefined;
  state.playProgress = undefined;
  state.combineFileMeta = undefined;
  state.isCombiningFile = false;
  return state;
});
}

export function getAllFiles(sections: Section[]) {
  return sections.map((s) => s.files).flat();
}


