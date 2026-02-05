export const files = $state<string[]>([]);
import { persisted } from 'svelte-persisted-store';
import { derived, get, writable } from 'svelte/store';
import { type AbletonColor, getDefaultColor } from '$lib/utils/colors';
import { invokeWithPerf, updateInputs } from './performance';
import { listen } from '@tauri-apps/api/event';
import { GroupsState } from './groups';
import type {
  OperationsState,
  OperationSource,
  SampleOp,
  OperationId,
  OperationDef,
} from './operation';
import { generateOperationId } from './operation';
import { Favorite } from './favorites';
import { TIMELINE_RESIZE } from '$lib/config/timelineConfig';

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

interface Operation {}

export interface AppState {
  _rev?: number; // content revision (groups + geometry + operations)

  _version?: number; // Internal version tracking for migrations
  combineAudioFileProgress?: number;
  combinedFile?: VisualSample;
  combinedFileLength?: number;
  favorites: Favorite[];
  groups?: GroupsState;
  hasNoActiveSamples: boolean;
  isCombiningFile: boolean;
  isLoopingTimelineAudio: boolean;
  operations?: OperationsState;
  playingCombined: boolean;
  playingSection?: number;
  playingSong?: string;
  playProgress?: number;
  sortDirection?: 'asc' | 'desc';
  sortKey?: keyof AudioFileItem;
  timelineItems: TimelineItem[];
  uiSettings?: {
    activeTab?: string;
    debugActiveTab?: string;
    tabContentHeight?: number;
    /** Currently selected operation ID (immutable identifier) */
    selectedOperationId?: OperationId | null;
    /** Set of selected timeline item IDs for multi-selection */
    selectedTimelineItemIds?: Set<string>;
    timelineDebugMode?: boolean;
    showFullSvgPath?: boolean;
    svgPathDisplayMode?: 'full' | 'trim' | 'hide';
    callSiteTrackingEnabled?: boolean;
    /** Timeline height as percentage of viewport (10-60) */
    debugPanelPrismDisplay?: {
      frontend: any;
      backend: any;
    };
    theme?: {
      tabPanelBackgroundColor?: string;
      panelHeaderBackgroundColor?: string;
      previewBackgroundColor?: string;
      previewBorderColor?: string;
      previewHoverBackgroundColor?: string;
      previewPulseColor?: string;
      waveformStrokeColor?: string;
      zIndexes?: {
        dropdown?: number;
        menu?: number;
      };
    }; // Add other UI settings here in the future
  };
}
export interface AudioFileItem {
  active: boolean;
  bitDepth?: number;
  bitRate?: number;
  channels?: number;
  color: AbletonColor;
  duration?: number;
  id: string;
  index: number;
  path: string;
  size?: number;
}

export interface Section {
  color: AbletonColor;
  errors: ErrorKind[];
  files: AudioFileItem[];
  folderPath: string;
  metaData?: FileMetadata[];
}

interface FileMetadata {
  bitDepth?: number;
  bitRate?: number;
  channels?: number;
  duration: number;
  id: string;
  path: string;
  size?: number;
}

/** Kind of timeline item - represents both the item type and operation type */
export type TimelineItemKind = 'sample' | 'merge' | 'spacer';

export interface BaseTimelineItem {
  id: string; // useful for identifying items
  kind: TimelineItemKind; // replaces both type and kind
  startOffset: number; // common field
  /** IDs of child timeline items (for MergeOps that contain other ops) */
  children?: string[];
  /** ID of the parent timeline item (for items inside a MergeOp) */
  parentId?: string;
  /** Visual nesting depth (0 = root level) */
  depth?: number;
  /** Semantic hint that this item is a group container */
  isGroup?: boolean;
  /** The operation ID this item came from (immutable reference) */
  operationId?: OperationId;
  /** The operation display name this item came from (may be changed by user) */
  operationName?: string;
}

export interface AudioFileTimelineItem extends BaseTimelineItem {
  kind: 'sample' | 'merge'; // audio files can be samples or merge operations
  svgPath: string;
  fileName: string;
  size: number;
  active: boolean;
  /** Duration in seconds - from duration cache, NOT waveform */
  duration?: number;
}

export interface SpacerTimelineItem extends BaseTimelineItem {
  kind: 'spacer'; // discriminator
  length: number; // unique property
}

export type TimelineItem = AudioFileTimelineItem | SpacerTimelineItem;

const CURRENT_STATE_VERSION = 1; // Increment this when you need to run migrations

// Function to validate and migrate appState from localStorage
function validateAndMigrateAppState(loadedState: any): AppState {
  const defaultState: AppState = {
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
      activeTab: 'Operations',
      debugActiveTab: 'frontend',
      tabContentHeight: 120,
      selectedOperationId: null,
      timelineDebugMode: false,
      showFullSvgPath: false,
      svgPathDisplayMode: 'trim',
      callSiteTrackingEnabled: false,
      debugPanelPrismDisplay: {
        frontend: {},
        backend: {},
      },
      theme: {
        panelHeaderBackgroundColor: 'rgb(15 21 27)',
        tabPanelBackgroundColor: 'rgb(15 21 27)',
        previewBackgroundColor: 'rgba(255, 165, 0, 0.25)',
        previewBorderColor: 'rgba(255, 165, 0, 0.5)',
        previewHoverBackgroundColor: 'rgba(255, 165, 0, 0.35)',
        previewPulseColor: 'rgba(255, 165, 0, 0.3)',
        waveformStrokeColor: '#3091f1',
        zIndexes: {
          dropdown: 100000,
          menu: 1000,
        },
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
    // Note: sections removed - now managed per operation
    // Ensure timelineItems array exists and is valid
    timelineItems: Array.isArray(loadedState.timelineItems) ? loadedState.timelineItems : [],
    // Migrate old activeTab to new uiSettings structure
    uiSettings: {
      activeTab: loadedState.activeTab || loadedState.uiSettings?.activeTab || 'Operations',
      debugActiveTab: loadedState.uiSettings?.debugActiveTab || 'frontend',
      tabContentHeight: loadedState.uiSettings?.tabContentHeight || 120,
      selectedOperationId: loadedState.uiSettings?.selectedOperationName || null,
      timelineDebugMode: loadedState.uiSettings?.timelineDebugMode || false,
      showFullSvgPath: loadedState.uiSettings?.showFullSvgPath || false,
      svgPathDisplayMode: loadedState.uiSettings?.svgPathDisplayMode || 'trim',
      callSiteTrackingEnabled: loadedState.uiSettings?.callSiteTrackingEnabled || false,
      theme: {
        panelHeaderBackgroundColor:
          loadedState.uiSettings.theme?.panelHeaderBackgroundColor || 'rgb(15 21 27)',
        tabPanelBackgroundColor:
          loadedState.uiSettings?.theme?.tabPanelBackgroundColor || 'rgb(15 21 27)',
        previewBackgroundColor:
          loadedState.uiSettings?.theme?.previewBackgroundColor || 'rgba(255, 165, 0, 0.25)',
        previewBorderColor:
          loadedState.uiSettings?.theme?.previewBorderColor || 'rgba(255, 165, 0, 0.5)',
        previewHoverBackgroundColor:
          loadedState.uiSettings?.theme?.previewHoverBackgroundColor || 'rgba(255, 165, 0, 0.35)',
        previewPulseColor:
          loadedState.uiSettings?.theme?.previewPulseColor || 'rgba(255, 165, 0, 0.3)',
        waveformStrokeColor: loadedState.uiSettings?.theme?.waveformStrokeColor || '#3091f1',
        zIndexes: {
          dropdown: loadedState.uiSettings?.theme?.zIndexes?.dropdown || 100000,
          menu: loadedState.uiSettings?.theme?.zIndexes?.menu || 1000,
        },
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
    // sectionsCount removed - now managed per operation
  });

  return migratedState;
}

export const appState = persisted<AppState>(
  'appState',
  {
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
      activeTab: 'Operations',
      debugActiveTab: 'frontend',
      tabContentHeight: 120,
      selectedOperationId: null,
      timelineDebugMode: false,
      showFullSvgPath: false,
      svgPathDisplayMode: 'trim',
      callSiteTrackingEnabled: false,
      theme: {
        tabPanelBackgroundColor: 'rgb(15 21 27)',
        previewBackgroundColor: 'rgba(255, 165, 0, 0.25)',
        previewBorderColor: 'rgba(255, 165, 0, 0.5)',
        previewHoverBackgroundColor: 'rgba(255, 165, 0, 0.35)',
        previewPulseColor: 'rgba(255, 165, 0, 0.3)',
        waveformStrokeColor: '#3091f1',
        zIndexes: {
          dropdown: 100000,
          menu: 1000,
        },
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

// Convenience function for timeline debug mode
export const timelineDebugMode = {
  subscribe: derived(appState, state => state.uiSettings?.timelineDebugMode ?? false).subscribe,
  toggle: () =>
    appState.update(state => ({
      ...state,
      uiSettings: {
        ...state.uiSettings,
        timelineDebugMode: !(state.uiSettings?.timelineDebugMode ?? false),
      },
    })),
  set: (value: boolean) =>
    appState.update(state => ({
      ...state,
      uiSettings: {
        ...state.uiSettings,
        timelineDebugMode: value,
      },
    })),
};

const defaults: AppState = {
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

// Removed addSource() function - use operation-specific addSourceToCurrentOperation() instead

// Removed deleteSection() function - use operation-specific deleteSectionFromCurrentOperation() instead

// Removed updatePath() function - use operation-specific functions instead

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

// Derived store for duration in seconds - updated to use operation-specific data or default
export const durationSeconds = derived(appState, $appState => {
  return $appState?.combinedFileLength ? $appState.combinedFileLength : 30;
});

// Removed syncIndexes() and applySyncIndexes() functions - use operation-specific functions instead

export function setActiveTab(tab: string) {
  appState.update(state => {
    if (!state.uiSettings) {
      state.uiSettings = {};
    }
    state.uiSettings.activeTab = tab;
    return state;
  });
}

export function setDebugActiveTab(tab: string) {
  appState.update(state => {
    if (!state.uiSettings) {
      state.uiSettings = {};
    }
    state.uiSettings.debugActiveTab = tab;
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

export function setPreviewThemeColors(colors: {
  backgroundColor?: string;
  borderColor?: string;
  hoverBackgroundColor?: string;
  pulseColor?: string;
  waveformStrokeColor?: string;
}) {
  appState.update(state => {
    if (!state.uiSettings) {
      state.uiSettings = {};
    }
    if (!state.uiSettings.theme) {
      state.uiSettings.theme = {};
    }

    if (colors.backgroundColor) {
      state.uiSettings.theme.previewBackgroundColor = colors.backgroundColor;
    }
    if (colors.borderColor) {
      state.uiSettings.theme.previewBorderColor = colors.borderColor;
    }
    if (colors.hoverBackgroundColor) {
      state.uiSettings.theme.previewHoverBackgroundColor = colors.hoverBackgroundColor;
    }
    if (colors.pulseColor) {
      state.uiSettings.theme.previewPulseColor = colors.pulseColor;
    }
    if (colors.waveformStrokeColor) {
      state.uiSettings.theme.waveformStrokeColor = colors.waveformStrokeColor;
    }

    return state;
  });
}

export function setWaveformStrokeColor(color: string) {
  appState.update(state => {
    if (!state.uiSettings) {
      state.uiSettings = {};
    }
    if (!state.uiSettings.theme) {
      state.uiSettings.theme = {};
    }
    state.uiSettings.theme.waveformStrokeColor = color;
    return state;
  });
}

export function setZIndex(component: string, value: number) {
  appState.update(state => {
    if (!state.uiSettings) {
      state.uiSettings = {};
    }
    if (!state.uiSettings.theme) {
      state.uiSettings.theme = {};
    }
    if (!state.uiSettings.theme.zIndexes) {
      state.uiSettings.theme.zIndexes = {};
    }
    (state.uiSettings.theme.zIndexes as any)[component] = value;
    return state;
  });
}

export function bumpRevision() {
  appState.update(state => {
    state._rev = (state._rev || 0) + 1;
    return state;
  });
}

/**
 * Set the currently selected operation by ID
 */
export function setSelectedOperationId(operationId: OperationId | null) {
  appState.update(state => {
    if (!state.uiSettings) {
      state.uiSettings = {};
    }
    state.uiSettings.selectedOperationId = operationId;
    if (operationId) {
      const op = state.operations?.defs?.[operationId];
      state.uiSettings.selectedOperationId = operationId;
    } else {
      state.uiSettings.selectedOperationId = null;
    }
    return state;
  });
}

/**
 * Get the currently selected operation ID
 */
export function getSelectedOperationId(): OperationId | null {
  const state = get(appState);
  // Prefer new ID field, fall back to name field for backward compatibility
  return state.uiSettings?.selectedOperationId ?? state.uiSettings?.selectedOperationId ?? null;
}

/**
 * Get the currently selected operation definition
 */
export function getSelectedOperation(): OperationDef | null {
  const state = get(appState);
  const id = state.uiSettings?.selectedOperationId ?? state.uiSettings?.selectedOperationId;
  if (!id) return null;
  return state.operations?.defs?.[id] ?? null;
}

export function toggleShowFullSvgPath() {
  appState.update(state => {
    if (!state.uiSettings) {
      state.uiSettings = {};
    }
    state.uiSettings.showFullSvgPath = !(state.uiSettings.showFullSvgPath ?? false);
    return state;
  });
}

export function setShowFullSvgPath(value: boolean) {
  appState.update(state => {
    if (!state.uiSettings) {
      state.uiSettings = {};
    }
    state.uiSettings.showFullSvgPath = value;
    return state;
  });
}

export function setSvgPathDisplayMode(mode: 'full' | 'trim' | 'hide') {
  appState.update(state => {
    if (!state.uiSettings) {
      state.uiSettings = {};
    }
    state.uiSettings.svgPathDisplayMode = mode;
    return state;
  });
}

// All operation-section related functions have been removed since operations
// no longer have a sections property. Operations now use sources instead.

/**
 * Current operation sources - gets the sources array from the currently selected MergeOp
 * For MergeOps, all sources are operation references to SampleOps
 */
export const currentOperationSources = derived(appState, $appState => {
  const selectedOperationId =
    $appState.uiSettings?.selectedOperationId ?? $appState.uiSettings?.selectedOperationId;
  if (!selectedOperationId) return [];

  const operation = $appState.operations?.defs?.[selectedOperationId];
  if (!operation || operation.kind !== 'merge') return [];

  return operation.sources;
});

/**
 * Current operation file list - gets all file IDs from SampleOps referenced by the current MergeOp
 * Returns an array of file ID strings extracted from the SampleOps
 */
export const currentOperationFileList = derived(appState, $appState => {
  const selectedOperationId =
    $appState.uiSettings?.selectedOperationId ?? $appState.uiSettings?.selectedOperationId;
  if (!selectedOperationId) return [];

  const operation = $appState.operations?.defs?.[selectedOperationId];
  if (!operation || operation.kind !== 'merge') return [];

  const fileIds: string[] = [];

  // For each source in the MergeOp (which should be operation references)
  for (const source of operation.sources) {
    if (source.type === 'operation') {
      // Get the referenced SampleOp by its operationId
      const sampleOp = $appState.operations?.defs?.[source.operationId];
      if (sampleOp && sampleOp.kind === 'sample') {
        // Extract file IDs from the SampleOp's sources (should have one 'file' type source)
        for (const sampleSource of sampleOp.sources) {
          if (sampleSource.type === 'file') {
            fileIds.push(sampleSource.fileId);
          }
        }
      }
    }
  }

  return fileIds;
});

/**
 * Get the sources array from the currently selected MergeOp
 * For MergeOps, all sources are operation references to SampleOps
 */
export function getCurrentOperationSources(): OperationSource[] {
  const currentState = get(appState);
  const selectedOperationId =
    currentState.uiSettings?.selectedOperationId ?? currentState.uiSettings?.selectedOperationId;
  if (!selectedOperationId) return [];

  const operation = currentState.operations?.defs?.[selectedOperationId];
  if (!operation || operation.kind !== 'merge') return [];

  return operation.sources;
}

/**
 * Add a source operation (by ID) to the current MergeOp
 */
export function addOperationSourceToCurrent(operationId: OperationId) {
  const currentState = get(appState);
  const selectedOperationId =
    currentState.uiSettings?.selectedOperationId ?? currentState.uiSettings?.selectedOperationId;
  if (!selectedOperationId) {
    console.warn('No operation currently selected');
    return;
  }

  appState.update(state => {
    const operation = state.operations?.defs?.[selectedOperationId];
    if (!operation || operation.kind !== 'merge') {
      console.warn('Current operation is not a MergeOp');
      return state;
    }

    // Add the new operation source using operationId
    const newSource: OperationSource = { type: 'operation', operationId };
    operation.sources.push(newSource);

    // Update the operations version
    if (state.operations) {
      state.operations._version = (state.operations._version ?? 0) + 1;
    }
    state._rev = (state._rev ?? 0) + 1;

    return state;
  });
}

/**
 * Remove a source from the current MergeOp by index
 */
export function removeSourceFromCurrentOperation(index: number) {
  const currentState = get(appState);
  const selectedOperationId =
    currentState.uiSettings?.selectedOperationId ?? currentState.uiSettings?.selectedOperationId;
  if (!selectedOperationId) {
    console.warn('No operation currently selected');
    return;
  }

  appState.update(state => {
    const operation = state.operations?.defs?.[selectedOperationId];
    if (!operation || operation.kind !== 'merge') {
      console.warn('Current operation is not a MergeOp');
      return state;
    }

    if (index >= 0 && index < operation.sources.length) {
      operation.sources.splice(index, 1);

      // Update the operations version
      if (state.operations) {
        state.operations._version = (state.operations._version ?? 0) + 1;
      }
      state._rev = (state._rev ?? 0) + 1;
    }

    return state;
  });
}

/**
 * Add a given operation as a source to the currently selected operation by its ID
 * This will add the operation reference to the current MergeOp's sources array
 */
export function addOpAsSourceById(operationId: OperationId) {
  const currentState = get(appState);
  const selectedOperationId =
    currentState.uiSettings?.selectedOperationId ?? currentState.uiSettings?.selectedOperationId;

  if (!selectedOperationId) {
    console.warn('No operation currently selected');
    return;
  }

  if (selectedOperationId === operationId) {
    console.warn('Cannot add an operation as a source to itself');
    return;
  }

  appState.update(state => {
    const currentOperation = state.operations?.defs?.[selectedOperationId];
    if (!currentOperation || currentOperation.kind !== 'merge') {
      console.warn('Current operation is not a MergeOp');
      return state;
    }

    const sourceOperation = state.operations?.defs?.[operationId];
    if (!sourceOperation) {
      console.warn(`Operation "${operationId}" not found`);
      return state;
    }

    // Check if this operation is already a source (by ID)
    const alreadyExists = currentOperation.sources.some(
      source => source.type === 'operation' && source.operationId === operationId
    );

    if (alreadyExists) {
      console.warn(
        `Operation "${operationId}" (${sourceOperation.name}) is already a source of current operation`
      );
      return state;
    }

    // Add the new operation source using operationId
    const newSource: OperationSource = { type: 'operation', operationId };
    currentOperation.sources.push(newSource);

    // Update the operations version
    if (state.operations) {
      state.operations._version = (state.operations._version ?? 0) + 1;
    }
    state._rev = (state._rev ?? 0) + 1;

    console.log(
      `Added operation "${operationId}" (${sourceOperation.name}) as source to current operation`
    );
    return state;
  });
}

/**
 * @deprecated Use addOpAsSourceById() instead
 * Add a given operation as a source to the currently selected operation
 */
export function addOpAsSource(operationIdOrName: string) {
  const currentState = get(appState);

  // Resolve operationIdOrName to an actual ID
  const defs = currentState.operations?.defs;
  let sourceOpId = operationIdOrName;
  if (defs && !defs[operationIdOrName]) {
    // Try to find by name
    const entry = Object.entries(defs).find(([, def]) => def.name === operationIdOrName);
    if (entry) {
      sourceOpId = entry[0];
    }
  }

  addOpAsSourceById(sourceOpId);
}

/**
 * Create SampleOps for each file in a directory and add them to the current MergeOp
 * This function will scan the directory, create a SampleOp for each audio file,
 * and add those operations as sources to the currently selected MergeOp
 */
export async function addSampleOpsFromDirectory(directoryPath: string) {
  const currentState = get(appState);
  const selectedOperationId =
    currentState.uiSettings?.selectedOperationId ?? currentState.uiSettings?.selectedOperationId;

  if (!selectedOperationId) {
    console.warn('No operation currently selected');
    return;
  }
  try {
    // TODO: This would need to be implemented to get files from directory
    // For now, we'll use a placeholder that simulates getting files
    const files = await getFilesFromDirectory(directoryPath);

    appState.update(state => {
      const operation = state.operations?.defs?.[selectedOperationId];
      if (!operation || operation.kind !== 'merge') {
        console.warn('Current operation is not a MergeOp');
        return state;
      }

      if (!state.operations) {
        state.operations = { defs: {}, order: [], _version: 1 };
      }
      if (!state.operations.order) {
        state.operations.order = Object.keys(state.operations.defs);
      }

      // Create a SampleOp for each file and add it to the current MergeOp
      files.forEach((filePath, index) => {
        // Generate a unique operation ID and display name
        const fileName =
          filePath
            .split(/[/\\]/)
            .pop()
            ?.replace(/\.[^/.]+$/, '') || `file_${index}`;
        const sampleOpId = generateOperationId();
        const sampleOpName = `sample_${fileName}`;

        // Create the SampleOp with id and name
        const sampleOp: SampleOp = {
          id: sampleOpId,
          name: sampleOpName,
          kind: 'sample',
          sources: [{ type: 'file', fileId: filePath }],
        };

        // Add the SampleOp to operations using its ID as key
        state.operations!.defs[sampleOpId] = sampleOp;
        state.operations!.order!.push(sampleOpId);

        // Add the SampleOp reference to the current MergeOp's sources using operationId
        const operationSource: OperationSource = { type: 'operation', operationId: sampleOpId };
        operation.sources.push(operationSource);
      });

      // Update versions
      state.operations._version = (state.operations._version ?? 0) + 1;
      state._rev = (state._rev ?? 0) + 1;
      console.log(state.operations);
      console.log(`Created ${files.length} SampleOps from directory: ${directoryPath}`);
      return state;
    });
  } catch (error) {
    console.error('Failed to add SampleOps from directory:', error);
  }
}

/**
 * Get files from a directory using Tauri backend
 */
async function getFilesFromDirectory(directoryPath: string): Promise<string[]> {
  try {
    const { invoke } = await import('@tauri-apps/api/core');

    // Use the existing Tauri command to get files from the directory
    const result: Record<string, string[]> = await invoke('get_file_paths_in_folder', {
      folderPaths: [directoryPath],
    });

    // Extract the files array for our directory
    return result[directoryPath] || [];
  } catch (error) {
    console.error('Failed to get files from directory:', error);

    // Fallback to empty array if the operation fails
    return [];
  }
}

// Call site tracking functions
export function setCallSiteTrackingEnabled(enabled: boolean) {
  appState.update(state => {
    if (!state.uiSettings) {
      state.uiSettings = {};
    }
    state.uiSettings.callSiteTrackingEnabled = enabled;
    return state;
  });
}

export function toggleCallSiteTrackingEnabled() {
  appState.update(state => {
    if (!state.uiSettings) {
      state.uiSettings = {};
    }
    state.uiSettings.callSiteTrackingEnabled = !(state.uiSettings.callSiteTrackingEnabled ?? false);
    return state;
  });
}

// Derived store for call site tracking enabled
export const callSiteTrackingEnabled = derived(
  appState,
  $appState => $appState.uiSettings?.callSiteTrackingEnabled ?? false
);
