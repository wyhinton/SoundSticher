export const files = $state<string[]>([]);
import { invoke } from '@tauri-apps/api/core';
import { onDestroy } from 'svelte';
import { persisted } from 'svelte-persisted-store'
import { derived, get } from 'svelte/store'
import { warn, debug, trace, info, error } from '@tauri-apps/plugin-log';
import { listen } from '@tauri-apps/api/event';

// trace('Trace');
// info('Info');
// error('Error');



export type ErrorKind = {
	kind: 'io' | 'utf8';
	message: string;
};
// First param `preferences` is the local storage key.
// Second param is the initial value.
export const preferences = persisted('preferences', {
  theme: 'dark',
  pane: '50%',
})

interface Song{
    title: String;
}

interface AppState{
    sections: Section[];
    playingSong?: string;
    playingSection?: number;
    playProgress?: number;
}

interface AudioFileItem{ 
    path: string,
    size?: number,
    bitRate?: number,
}

export interface Section{
    folderPath: string,
    files: AudioFileItem[],
    errors: ErrorKind[],
    metaData?: FileMetadata[]
}

export const appState = persisted<AppState>('appState', {
    sections: [],
})


const DEFAULT_FOLDER = "C:\\Users\\Primary User\\Desktop\\AUDIO\\FREESOUNDS\\_time-leeuwarden"

export function addSection(){
    appState.update(state=>{
         state.sections = [{folderPath: DEFAULT_FOLDER, files: [], errors: [],  metaData: []}, ...state.sections];
         return state;
    })
    get_file_paths_in_folder(0);
    
}

export function deleteSection(index: number) {
    appState.update(state => {
        // Remove the section at the specified index
        state.sections.splice(index, 1);
        return state;
    });
}

export function updatePath(sectionIndex: number, value: string){
    appState.update(state=>{
         state.sections[sectionIndex].folderPath = value;
         return state;
    })
    get_file_paths_in_folder(sectionIndex)
}

export async function play_song(song: string, sectionIndex: number){
    await invoke<Song[]>("play_song", {title: song}).then(f=>{
        appState.update(state=>{
            state.playingSection = sectionIndex;
            state.playingSong = song;
            return state;
        })
        console.log(f)
    });
}


interface FileMetadata{
    path: string,
    size?: number,
    bitRate?: number,
}

export async function get_file_paths_in_folder(sectionIndex: number) {
    const { sections } = get(appState);
    const folder = sections[sectionIndex]?.folderPath;

    if (!folder) return;

    try {
        const files = await invoke<string[]>("get_file_paths_in_folder", { folderPath: folder });

        // Set file paths first
        appState.update(state => {
            const section = state.sections[sectionIndex];
            section.files = files.map(f=>({path: f}));
            section.errors = section.errors.filter(e => e.kind === "io");
            return state;
        });

        console.log(`Fetched files for section ${sectionIndex}:`, files);

        // Now fetch metadata for each file in parallel
        const metadataList = await Promise.all<FileMetadata|null>(
            files.map(async (file) => {
                try {
                    const metadata = await invoke<FileMetadata>("get_metadata", { title: file });
                    return metadata;
                } catch (err) {
                    console.error(`Failed to get metadata for ${file}:`, err);
                    return null;
                }
            })
        );

        // Store metadata in the section (you can customize this structure)
        appState.update(state => {
            const section = state.sections[sectionIndex];
            state.sections.forEach((s,i)=>{
                s.files.forEach((f,j)=>{
                    const meta = metadataList.filter(m=>m.path===f.path)[0];
                    state.sections[i].files[j] = {...f, bitRate: meta.bitRate, size: meta.size}
                })
            })
            console.log(state.sections)
            section.metaData = metadataList;
            return state;
        });

    } catch (e: any) {
        console.error("Failed to fetch files:", e);

        appState.update(state => {
            const section = state.sections[sectionIndex];
            section.errors.push({
                kind: "io",
                message: e.message || "Unknown error",
            });
            return state;
        });
    }
}



listen<number>('song-progress', (event) => {
    appState.update(state => {
        state.playProgress = event.payload
        return state;
    });
});

appState.subscribe((s)=>{
    console.log(s)
    
})



// export class AppStateClass{
//     sections = $state<Section[]>([]);

//     constructor(){
//         onDestroy(()=>{
//             console.log("DESTROYED")
//         })
//     }

//     addSection(){
//         this.sections = [{folderPath: "myfolder", files: [], errors: []}, ...appState.sections];
//         get_file_paths_in_folder(0, appState.sections[0].folderPath);
        
//     }
    
// }