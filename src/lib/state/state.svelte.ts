export const files = $state<string[]>([]);
import { persisted } from 'svelte-persisted-store';
import { derived, get, writable } from 'svelte/store';
import { ABLETON_COLORS, type AbletonColor, getDefaultColor } from '$lib/utils/colors';
import { invokeWithPerf, updateInputs } from './performance';
import { listen } from '@tauri-apps/api/event';
import { Channel, invoke } from '@tauri-apps/api/core';
import { generateProgressChannel, type SortAudioEvent } from './events';
import { GroupsState } from './groups';

export type ErrorKind = {
  kind: 'io' | 'utf8';
  message: string;
};
// First param `preferences` is the local storage key.
// Second param is the initial value.
export const preferences = persisted('preferences', {
  theme: 'dark',
  pane: '50%',
});

interface Song {
  title: String;
}

interface VisualSample {
  path?: string;
  svgPath: string;
}

export interface AppState {
  sections: Section[];
  playingSong?: string;
  playingSection?: number;
  playProgress?: number;
  combinedFile?: VisualSample;
  combinedFileLength?: number;
  isCombiningFile: boolean;
  combineAudioFileProgress?: number;
  playingCombined: boolean;
  timelineItems: TimelineItem[];
  sortKey?: keyof AudioFileItem;
  sortDirection?: 'asc' | 'desc';
  isLoopingTimelineAudio: boolean;
  hasNoActiveSamples: boolean;
  favorites: Favorite[];
  uiSettings?: {
    activeTab?: string;
    tabContentHeight?: number;
    theme?: {
      tabPanelBackgroundColor?: string;
    };
    // Add other UI settings here in the future
  };
  _version?: number; // Internal version tracking for migrations
  groups?: GroupsState;
  _rev?: number; // content revision (groups + geometry)
}

export interface AudioFileItem {
  index: number;
  path: string;
  color: AbletonColor;
  size?: number;
  bitRate?: number;
  channels?: number;
  bitDepth?: number;
  duration?: number;
  id: string;
  active: boolean;
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
  id: string;
}

export type TimelineItemType = 'audio-file' | 'spacer';

export interface BaseTimelineItem {
  id: string; // useful for identifying items
  type: TimelineItemType;
  startOffset: number; // common field
}

export interface AudioFileTimelineItem extends BaseTimelineItem {
  type: 'audio-file';
  svgPath: string;
  fileName: string;
  size: number;
  active: boolean;
}

export interface SpacerTimelineItem extends BaseTimelineItem {
  type: 'spacer'; // discriminator
  length: number; // unique property
}

export type TimelineItem = AudioFileTimelineItem | SpacerTimelineItem;

interface Favorite {
  path: string;
}

const CURRENT_STATE_VERSION = 1; // Increment this when you need to run migrations

// Function to validate and migrate appState from localStorage
function validateAndMigrateAppState(loadedState: any): AppState {
  const defaultState: AppState = {
    sections: [],
    isCombiningFile: false,
    combinedFileLength: 0,
    playingCombined: false,
    combinedFile: undefined,
    timelineItems: [],
    isLoopingTimelineAudio: false,
    hasNoActiveSamples: false,
    sortKey: undefined,
    sortDirection: undefined,
    favorites: [],
    uiSettings: {
      activeTab: 'Global',
      tabContentHeight: 120,
      theme: {
        tabPanelBackgroundColor: 'rgb(15 21 27)',
      },
    },
    _rev: 0, // content revision (groups + geometry)
    _version: CURRENT_STATE_VERSION,
  };

  // If no state exists, return default
  if (!loadedState || typeof loadedState !== 'object') {
    console.log('No valid appState found in localStorage, using defaults');
    return defaultState;
  }

  // Check if migration is needed
  const needsMigration = !loadedState._version || loadedState._version < CURRENT_STATE_VERSION;

  if (!needsMigration) {
    // State is current, just ensure it has all required properties
    return {
      ...defaultState,
      ...loadedState,
      _version: CURRENT_STATE_VERSION,
    };
  }

  console.log(
    `Migrating appState from version ${loadedState._version || 0} to ${CURRENT_STATE_VERSION}`
  );

  // Perform migration - merge loaded state with defaults to ensure all properties exist
  const migratedState: AppState = {
    ...defaultState,
    ...loadedState,
    // Ensure favorites array exists and is valid
    favorites: Array.isArray(loadedState.favorites) ? loadedState.favorites : [],
    // Ensure sections array exists and is valid
    sections: Array.isArray(loadedState.sections) ? loadedState.sections : [],
    // Ensure timelineItems array exists and is valid
    timelineItems: Array.isArray(loadedState.timelineItems) ? loadedState.timelineItems : [],
    // Migrate old activeTab to new uiSettings structure
    uiSettings: {
      activeTab: loadedState.activeTab || loadedState.uiSettings?.activeTab || 'Global',
      tabContentHeight: loadedState.uiSettings?.tabContentHeight || 120,
      theme: {
        tabPanelBackgroundColor:
          loadedState.uiSettings?.theme?.tabPanelBackgroundColor || 'rgb(15 21 27)',
        ...loadedState.uiSettings?.theme,
      },
      ...loadedState.uiSettings,
    },
    // Update version to current
    _version: CURRENT_STATE_VERSION,
  };

  console.log('AppState migration completed:', {
    fromVersion: loadedState._version || 0,
    toVersion: CURRENT_STATE_VERSION,
    hasOriginalFavorites: !!loadedState.favorites,
    migratedFavoritesCount: migratedState.favorites.length,
    sectionsCount: migratedState.sections.length,
  });

  return migratedState;
}

export const appState = persisted<AppState>(
  'appState',
  {
    sections: [],
    isCombiningFile: false,
    combinedFileLength: 0,
    playingCombined: false,
    combinedFile: undefined,
    timelineItems: [],
    isLoopingTimelineAudio: false,
    hasNoActiveSamples: false,
    sortKey: undefined,
    sortDirection: undefined,
    favorites: [],
    uiSettings: {
      activeTab: 'Global',
      tabContentHeight: 120,
      theme: {
        tabPanelBackgroundColor: 'rgb(15 21 27)',
      },
    },
    _version: CURRENT_STATE_VERSION,
  },
  {
    serializer: {
      parse: (text: string) => {
        try {
          const parsed = JSON.parse(text);
          return validateAndMigrateAppState(parsed);
        } catch (error) {
          console.error('Error parsing appState from localStorage:', error);
          return validateAndMigrateAppState(null);
        }
      },
      stringify: JSON.stringify,
    },
  }
);

const defaults: AppState = {
  sections: [],
  isCombiningFile: false,
  combinedFileLength: 0,
  playingCombined: false,
  combinedFile: undefined,
  timelineItems: [],
  isLoopingTimelineAudio: false,
  hasNoActiveSamples: false,
  favorites: [],
  _version: CURRENT_STATE_VERSION,
};

export const hoveredSourceItem = writable<null | number>(null);
export const hoveredTimelineItem = writable<null | number>(null);

// Store for tracking IDs that should be animated in FileTable
export const animatedIds = writable<Set<string>>(new Set());

export const setHoveredItem = (index: number | null) => {
  // hoveredItem.update((state) => {
  //   return index;
  // })
  hoveredSourceItem.set(index);
};

// Function to trigger animation for changed file IDs
export const triggerFileAnimation = (changedFileIds: string[]) => {
  // Add all changed IDs to the animated set
  animatedIds.update(currentSet => {
    const newSet = new Set(currentSet);
    changedFileIds.forEach(id => newSet.add(id));
    return newSet;
  });

  // Remove IDs after animation duration (e.g., 2 seconds)
  setTimeout(() => {
    animatedIds.update(currentSet => {
      const newSet = new Set(currentSet);
      changedFileIds.forEach(id => newSet.delete(id));
      return newSet;
    });
  }, 2000); // 2 second animation duration
};

const DEFAULT_FOLDER = 'C:\\Users\\Primary User\\Desktop\\AUDIO\\FREESOUNDS\\_time-leeuwarden';

let isCurrentlyCombining = false;
let combiningCheckInterval;

export async function addSource(paths?: string | string[]) {
  const defaultSectionColor = getDefaultColor();
  const selectedFolderPaths = Array.isArray(paths) ? paths : [paths ?? DEFAULT_FOLDER];

  const tState = get(appState);
  console.log(tState);

  try {
    // Get file paths for each folder
    const folderFilesResult = await invokeWithPerf<Record<string, string[]>>(
      'get_file_paths_in_folder',
      {
        folderPaths: selectedFolderPaths,
      }
    );

    if (folderFilesResult.ok === true) {
      // Flatten all file paths to request metadata at once
      const allDiscoveredFilePaths: string[] = Object.values(folderFilesResult.value).flat();
      console.log(allDiscoveredFilePaths);

      // Get metadata for all discovered files
      const fileMetadataResult = await invokeWithPerf<FileMetadata[]>('get_metadata', {
        titles: allDiscoveredFilePaths,
      });

      if (fileMetadataResult.ok === true) {
        // Calculate starting index for new files - always use sequential indexing
        let nextIndex = 0;
        const allExistingFiles = getAllFiles(tState.sections);
        if (allExistingFiles.length > 0) {
          nextIndex = Math.max(...allExistingFiles.map(f => f.index)) + 1;
        }
        console.log(folderFilesResult);
        const newSourceSections: Section[] = Object.entries(folderFilesResult.value).map(
          ([folderPath, discoveredFiles]) => {
            const filesWithMetadata: AudioFileItem[] = discoveredFiles
              .map((filePath, fileIndex) => {
                const fileMetadata = fileMetadataResult.value.find(
                  metadata => metadata.path === filePath
                );

                // Use sequential indexing for all new files
                const properIndex = nextIndex++;

                return fileMetadata
                  ? {
                      ...fileMetadata,
                      color: defaultSectionColor,
                      index: properIndex,
                      active: true,
                    }
                  : null;
              })
              .filter(Boolean) as AudioFileItem[];

            return {
              folderPath,
              files: filesWithMetadata,
              errors: [],
              metaData: [],
              color: defaultSectionColor,
            };
          }
        );

        console.log(newSourceSections);

        // Update app state with new sections
        appState.update(currentState => {
          return {
            ...currentState,
            combinedFile: undefined,
            combinedFileLength: undefined,
            sections: [...newSourceSections, ...currentState.sections],
          };
        });
      }
    }

    // Send updated sections to backend/input processor
    const updatedAppState = get(appState);
    updateInputs(updatedAppState.sections);
  } catch (error) {
    console.error('Error in addSection:', error);
  }
}

export function deleteSection(index: number) {
  console.log(`%cHERE LINE :150 %c`, 'color: yellow; font-weight: bold', '');

  appState.update(state => {
    // Remove the section at the specified index
    state.sections.splice(index, 1);
    if (state.sections.length === 0) {
      invokeWithPerf('clear_audio_files');
      state.sections = [];
      state.timelineItems = [];
      state.combinedFile = undefined;
      return state;
    } else {
      updateInputs(state.sections);
    }
    return state;
  });
}

export function updatePath(sectionIndex: number, value: string) {
  appState.update(state => {
    console.log(state.sections);
    if (state.sections[sectionIndex]) {
      state.sections[sectionIndex].folderPath = value;
    }
    return state;
  });
  get_file_paths_in_folder(sectionIndex);
}

export async function play_sample_preview(song: string) {
  await invokeWithPerf<Song[]>('play_sample_preview', { title: song }).then(f => {
    appState.update(state => {
      state.playingSong = song;
      return state;
    });
    console.log(f);
  });
}

export async function pause_sample_preview() {
  await invokeWithPerf<Song[]>('pause_sample_preview', {}).then(f => {
    appState.update(state => {
      state.playingSong = undefined;
      return state;
    });
  });
}

export interface CombineAudioResult {
  output: string;
  svgPath: string;
}

export async function combine_audio_files(input_files: string[], output_path: string) {
  const combineAudioFilesRes = await invokeWithPerf<CombineAudioResult>('combine_audio_files', {
    inputFiles: input_files,
    outputPath: output_path,
  });
  if (combineAudioFilesRes.ok === true) {
    const getMetadataRes = await invokeWithPerf<FileMetadata>('get_metadata', {
      title:
        'C:\\Users\\Primary User\\Desktop\\TAURI_APPS\\SKV2\\tauri-v2-sveltekit-template\\assets\\test_output\\test.wav',
    });
    if (getMetadataRes.ok === true) {
      appState.update(state => {
        state.combinedFile = {
          path: combineAudioFilesRes.value.output,
          svgPath: combineAudioFilesRes.value.svgPath,
        };
        return state;
      });
    }
  }
}

export async function get_file_paths_in_folder(sectionIndex: number) {
  // console.log(`%cHERE LINE :188 %c`,'color: brown; font-weight: bold', '');
  // const { sections } = get(appState);
  // const folder = sections[sectionIndex]?.folderPath;
  // if (!folder) return;
  // try {
  //   const files = await invokeWithPerf<string[]>("get_file_paths_in_folder", {
  //     folderPath: folder,
  //   });
  //   // Set file paths first
  //   appState.update((state) => {
  //     const section = state.sections[sectionIndex];
  //     section.files = files.map((f) => ({ path: f, color: section.color }));
  //     section.errors = section.errors.filter((e) => e.kind === "io");
  //     return state;
  //   });
  //   console.log(`Fetched files for section ${sectionIndex}:`, files);
  //   const metadataList: FileMetadata[] = await invokeWithPerf("get_metadata", {
  //     titles: files,
  //   });
  //   // Now fetch metadata for each file in parallel
  //   // const metadataList = await Promise.all<FileMetadata[] | null>(
  //   //   files.map(async (file) => {
  //   //     // try {
  //   //     //   const metadata = await invokeWithPerf<FileMetadata>("get_metadata", {
  //   //     //     title: file,
  //   //     //   });
  //   //     //   return metadata;
  //   //     // } catch (err) {
  //   //     //   console.error(`Failed to get metadata for ${file}:`, err);
  //   //     //   return null;
  //   //     // }
  //   //   })
  //   // );
  //   // Store metadata in the section (you can customize this structure)
  //   appState.update((state) => {
  //     console.log(
  //       `%cHERE LINE :204 %c`,
  //       "color: yellow; font-weight: bold",
  //       ""
  //     );
  //     const section = state.sections[sectionIndex];
  //     state.sections.forEach((s, i) => {
  //       s.files.forEach((f, j) => {
  //         const meta = metadataList.filter((m) => m.path === f.path)[0];
  //         state.sections[i].files[j] = {
  //           ...f,
  //           bitRate: meta.bitRate,
  //           size: meta.size,
  //           channels: meta.channels,
  //           duration: meta.duration,
  //           bitDepth: meta.duration,
  //         };
  //       });
  //     });
  //     console.log(state.sections);
  //     return state;
  //   });
  // } catch (e: any) {
  //   console.error("Failed to fetch files:", e);
  //   appState.update((state) => {
  //     const section = state.sections[sectionIndex];
  //     section.errors.push({
  //       kind: "io",
  //       message: e.message || "Unknown error",
  //     });
  //     return state;
  //   });
  // }
}

appState.subscribe(s => {
  // console.log(s);
});

export function resetAppState() {
  appState.update(state => {
    state.combinedFile = undefined;
    // state.sections = [];
    state.playingSong = undefined;
    state.playingSection = undefined;
    state.playProgress = undefined;
    state.isCombiningFile = false;
    state.isLoopingTimelineAudio = false;
    return state;
  });
}

export function getAllFiles(sections: Section[]) {
  return sections.map(s => s.files).flat();
}

let prevValue = get(appState);

export interface SectionSend {
  folderPath: string;
  paths: AudioSend[];
}

interface AudioSend {
  path: string;
}

listen<number>('song-progress', event => {
  appState.update(state => {
    state.playProgress = event.payload;
    console.log(event);
    return state;
  });
});

interface CachedCombineResult {
  svgPath: string;
  duration: number;
}

listen<CachedCombineResult>('combined-cached', event => {
  console.log(event);
  appState.update(state => {
    if (state.combinedFile?.svgPath) {
      state.combinedFile.svgPath += event.payload.svgPath;
      console.log(state.combinedFile.svgPath.length);
      state.combinedFileLength = event.payload.duration;
    }
    return state;
  });
});

listen<string>('processed-segment', event => {
  appState.update(state => {
    if (state.combinedFile) {
      state.combinedFile = {
        ...state.combinedFile,
        svgPath: state.combinedFile.svgPath + event.payload,
      };
    }
    return state;
  });
});

listen<number>('total-length', event => {
  appState.update(state => {
    console.log(event);
    state.combinedFileLength = event.payload;
    return state;
  });
});

listen<number>('combine-audio-progress', event => {
  appState.update(state => {
    console.log(event);
    // state.playProgress = event.payload;
    state.combineAudioFileProgress = event.payload;

    //  getCurrentWindow().setProgressBar({
    //   status: ProgressBarStatus.Normal,
    //   progress: event.payload*100,
    // });
    return state;
  });
});

function offsetX(path: string, dx: number): string {
  // Regex matches commands followed by coordinate pairs
  // Example: "M0.0,0.0" => command=M, x=0.0, y=0.0
  return path.replace(/([MLCQTZHV])\s*(-?\d*\.?\d*)(?:,|\s*)(-?\d*\.?\d*)?/gi, (_, cmd, x, y) => {
    if (x !== undefined && y !== undefined) {
      const newX = (parseFloat(x) + dx).toFixed(1); // keep 1 decimal place like your input
      return `${cmd}${newX},${y}`;
    }
    return `${cmd}${x ?? ''}${y ?? ''}`;
  });
}

// Derived store for duration in seconds - used by Timeline and other components
export const durationSeconds = derived(appState, $appState => {
  return $appState?.combinedFileLength && $appState.sections.length > 0
    ? $appState.combinedFileLength
    : 30;
});

// Function to sync file indices with backend response and trigger animations
export function syncIndexes(
  newOrder: [string, number][],
  currentState: AppState
): {
  updatedState: AppState;
  changedIds: string[];
} {
  const allFiles = getAllFiles(currentState.sections);
  const changedIds: string[] = [];

  // Update each file's index based on the new order from backend
  newOrder.forEach(([fileId, newIndex]) => {
    const toUpdate = allFiles.find(f => f.id === fileId);
    if (toUpdate) {
      const oldIndex = toUpdate.index;

      // Check if the index actually changed
      if (oldIndex !== newIndex) {
        changedIds.push(toUpdate.id);
        console.log(`Updating file ${toUpdate.id} index from ${oldIndex} to ${newIndex}`);
        toUpdate.index = newIndex;
        console.log('File after update:', toUpdate);
      } else {
        console.log(`File ${toUpdate.id} index unchanged: ${oldIndex}`);
      }
    } else {
      console.warn(`File with ID ${fileId} not found in sections`);
    }
  });

  console.log(`Changed IDs (${changedIds.length}):`, changedIds);

  // Create new sections array to trigger reactivity
  const newSections = currentState.sections.map(section => ({
    ...section,
    files: [...section.files], // Create new file arrays
  }));

  console.log('Updated sections:', newSections);

  const updatedState = {
    ...currentState,
    sections: newSections,
  };

  return {
    updatedState,
    changedIds,
  };
}

// Function to apply index sync and trigger animations
export function applySyncIndexes(newOrder: [string, number][]): void {
  appState.update(state => {
    const { updatedState, changedIds } = syncIndexes(newOrder, state);

    // Trigger animation for changed files
    if (changedIds.length > 0) {
      triggerFileAnimation(changedIds);
    }

    return updatedState;
  });
}

export function addToFavorites(folderPath: string) {
  appState.update(state => {
    // Ensure favorites array exists
    if (!Array.isArray(state.favorites)) {
      state.favorites = [];
    }

    // Check if the path is already in favorites
    const alreadyExists = state.favorites.some(fav => fav && fav.path === folderPath);

    if (!alreadyExists) {
      state.favorites.push({ path: folderPath });
      console.log(`Added ${folderPath} to favorites`);
    } else {
      console.log(`${folderPath} is already in favorites`);
    }

    return state;
  });
}

export function removeFromFavorites(folderPath: string) {
  appState.update(state => {
    // Ensure favorites array exists
    if (!Array.isArray(state.favorites)) {
      state.favorites = [];
      return state;
    }

    state.favorites = state.favorites.filter(fav => fav && fav.path !== folderPath);
    console.log(`Removed ${folderPath} from favorites`);
    return state;
  });
}

export function isFavorite(folderPath: string): boolean {
  const currentState = get(appState);
  // Ensure favorites array exists and is valid
  if (!currentState || !Array.isArray(currentState.favorites)) {
    return false;
  }
  return currentState.favorites.some(fav => fav && fav.path === folderPath);
}

export function setActiveTab(tab: string) {
  appState.update(state => {
    if (!state.uiSettings) {
      state.uiSettings = {};
    }
    state.uiSettings.activeTab = tab;
    return state;
  });
}

export function setTabContentHeight(height: number) {
  appState.update(state => {
    if (!state.uiSettings) {
      state.uiSettings = {};
    }
    state.uiSettings.tabContentHeight = height;
    return state;
  });
}

export function setThemeColor(property: string, color: string) {
  appState.update(state => {
    if (!state.uiSettings) {
      state.uiSettings = {};
    }
    if (!state.uiSettings.theme) {
      state.uiSettings.theme = {};
    }
    (state.uiSettings.theme as any)[property] = color;
    return state;
  });
}

export function bumpRevision() {
  appState.update(state => {
    state._rev = (state._rev || 0) + 1;
    return state;
  });
}
