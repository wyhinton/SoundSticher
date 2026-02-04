/**
 * Timeline Store - Keyed store for multiple timeline instances
 *
 * This module provides isolated state for each timeline instance,
 * allowing multiple timelines to coexist without interfering with each other.
 */

import { writable, derived, get, type Writable, type Readable } from 'svelte/store';
import type { TimelineHierarchy } from './timelineGraph';
import type {
  TimelineItem,
  AudioFileTimelineItem,
  SpacerTimelineItem,
  BaseTimelineItem,
  TimelineItemKind,
  AppState,
} from './state.svelte';
import { buildHierarchyMaps } from './timelineGraph';

// ============================================================================
// Types - Re-export from state.svelte.ts for compatibility
// ============================================================================

export type TimelineId = string;

// Re-export TimelineItem types from the canonical source
export type {
  TimelineItem,
  AudioFileTimelineItem,
  SpacerTimelineItem,
  BaseTimelineItem,
  TimelineItemKind,
};

export interface TimelineState {
  items: TimelineItem[];
  duration: number;
  hierarchy: TimelineHierarchy | null;
  waveformsLoading: boolean;
  playheadPosition: number;
  isPlaying: boolean;
}

export interface TimelineSelection {
  selectedIds: Set<number>;
  previewIds: Set<number>;
  previewActive: boolean;
  lastSelectedIndex: number | null;
}

export interface TimelinePlayback {
  progress: number; // 0-1
  isPlaying: boolean;
  currentTime: number;
  duration: number;
}

// ============================================================================
// Default States
// ============================================================================

const DEFAULT_TIMELINE_STATE: TimelineState = {
  items: [],
  duration: 0,
  hierarchy: null,
  waveformsLoading: false,
  playheadPosition: 0,
  isPlaying: false,
};

const DEFAULT_SELECTION: TimelineSelection = {
  selectedIds: new Set(),
  previewIds: new Set(),
  previewActive: false,
  lastSelectedIndex: null,
};

const DEFAULT_PLAYBACK: TimelinePlayback = {
  progress: 0,
  isPlaying: false,
  currentTime: 0,
  duration: 0,
};

// ============================================================================
// Main Timeline Store (keyed by timelineId)
// ============================================================================

type TimelineStoreState = Record<TimelineId, TimelineState>;
type SelectionStoreState = Record<TimelineId, TimelineSelection>;
type PlaybackStoreState = Record<TimelineId, TimelinePlayback>;

// Core stores
const timelineStoreInternal: Writable<TimelineStoreState> = writable({});
const selectionStoreInternal: Writable<SelectionStoreState> = writable({});
const playbackStoreInternal: Writable<PlaybackStoreState> = writable({});

// ============================================================================
// Timeline Store API
// ============================================================================

export const timelineStore = {
  subscribe: timelineStoreInternal.subscribe,

  /**
   * Get or create a timeline state for a given ID
   */
  get(timelineId: TimelineId): TimelineState {
    const state = get(timelineStoreInternal);
    return state[timelineId] ?? DEFAULT_TIMELINE_STATE;
  },

  /**
   * Create a derived store for a specific timeline
   */
  forTimeline(timelineId: TimelineId): Readable<TimelineState> {
    return derived(timelineStoreInternal, $store => {
      return $store[timelineId] ?? DEFAULT_TIMELINE_STATE;
    });
  },

  /**
   * Initialize a timeline with default state
   */
  init(timelineId: TimelineId, initialState?: Partial<TimelineState>): void {
    timelineStoreInternal.update(state => ({
      ...state,
      [timelineId]: {
        ...DEFAULT_TIMELINE_STATE,
        ...initialState,
      },
    }));
  },

  /**
   * Update timeline state
   */
  update(timelineId: TimelineId, updates: Partial<TimelineState>): void {
    timelineStoreInternal.update(state => ({
      ...state,
      [timelineId]: {
        ...(state[timelineId] ?? DEFAULT_TIMELINE_STATE),
        ...updates,
      },
    }));
  },

  /**
   * Set timeline items
   */
  setItems(timelineId: TimelineId, items: TimelineItem[]): void {
    timelineStoreInternal.update(state => ({
      ...state,
      [timelineId]: {
        ...(state[timelineId] ?? DEFAULT_TIMELINE_STATE),
        items,
      },
    }));
  },

  /**
   * Set timeline duration
   */
  setDuration(timelineId: TimelineId, duration: number): void {
    timelineStoreInternal.update(state => ({
      ...state,
      [timelineId]: {
        ...(state[timelineId] ?? DEFAULT_TIMELINE_STATE),
        duration,
      },
    }));
  },

  /**
   * Set hierarchy
   */
  setHierarchy(timelineId: TimelineId, hierarchy: TimelineHierarchy | null): void {
    timelineStoreInternal.update(state => ({
      ...state,
      [timelineId]: {
        ...(state[timelineId] ?? DEFAULT_TIMELINE_STATE),
        hierarchy,
      },
    }));
  },

  /**
   * Set loading state
   */
  setLoading(timelineId: TimelineId, loading: boolean): void {
    timelineStoreInternal.update(state => ({
      ...state,
      [timelineId]: {
        ...(state[timelineId] ?? DEFAULT_TIMELINE_STATE),
        waveformsLoading: loading,
      },
    }));
  },

  /**
   * Remove a timeline
   */
  remove(timelineId: TimelineId): void {
    timelineStoreInternal.update(state => {
      const { [timelineId]: _, ...rest } = state;
      return rest;
    });
  },

  /**
   * Clear all timelines
   */
  clear(): void {
    timelineStoreInternal.set({});
  },
};

// ============================================================================
// Selection Store API (per-timeline)
// ============================================================================

export const selectionStore = {
  subscribe: selectionStoreInternal.subscribe,

  /**
   * Get selection state for a timeline
   */
  get(timelineId: TimelineId): TimelineSelection {
    const state = get(selectionStoreInternal);
    return state[timelineId] ?? DEFAULT_SELECTION;
  },

  /**
   * Create a derived store for a specific timeline's selection
   */
  forTimeline(timelineId: TimelineId): Readable<TimelineSelection> {
    return derived(selectionStoreInternal, $store => {
      return $store[timelineId] ?? DEFAULT_SELECTION;
    });
  },

  /**
   * Initialize selection for a timeline
   */
  init(timelineId: TimelineId): void {
    selectionStoreInternal.update(state => ({
      ...state,
      [timelineId]: { ...DEFAULT_SELECTION, selectedIds: new Set(), previewIds: new Set() },
    }));
  },

  /**
   * Set selected IDs
   */
  setSelected(timelineId: TimelineId, ids: Set<number>): void {
    selectionStoreInternal.update(state => ({
      ...state,
      [timelineId]: {
        ...(state[timelineId] ?? DEFAULT_SELECTION),
        selectedIds: new Set(ids),
      },
    }));
  },

  /**
   * Add to selection
   */
  addToSelection(timelineId: TimelineId, index: number): void {
    selectionStoreInternal.update(state => {
      const current = state[timelineId] ?? DEFAULT_SELECTION;
      const newSelected = new Set(current.selectedIds);
      newSelected.add(index);
      return {
        ...state,
        [timelineId]: {
          ...current,
          selectedIds: newSelected,
        },
      };
    });
  },

  /**
   * Remove from selection
   */
  removeFromSelection(timelineId: TimelineId, index: number): void {
    selectionStoreInternal.update(state => {
      const current = state[timelineId] ?? DEFAULT_SELECTION;
      const newSelected = new Set(current.selectedIds);
      newSelected.delete(index);
      return {
        ...state,
        [timelineId]: {
          ...current,
          selectedIds: newSelected,
        },
      };
    });
  },

  /**
   * Toggle selection
   */
  toggleSelection(timelineId: TimelineId, index: number): void {
    selectionStoreInternal.update(state => {
      const current = state[timelineId] ?? DEFAULT_SELECTION;
      const newSelected = new Set(current.selectedIds);
      if (newSelected.has(index)) {
        newSelected.delete(index);
      } else {
        newSelected.add(index);
      }
      return {
        ...state,
        [timelineId]: {
          ...current,
          selectedIds: newSelected,
        },
      };
    });
  },

  /**
   * Clear selection
   */
  clear(timelineId: TimelineId): void {
    selectionStoreInternal.update(state => ({
      ...state,
      [timelineId]: {
        ...(state[timelineId] ?? DEFAULT_SELECTION),
        selectedIds: new Set(),
        lastSelectedIndex: null,
      },
    }));
  },

  /**
   * Set preview IDs
   */
  setPreview(timelineId: TimelineId, ids: Set<number>, active: boolean = true): void {
    selectionStoreInternal.update(state => ({
      ...state,
      [timelineId]: {
        ...(state[timelineId] ?? DEFAULT_SELECTION),
        previewIds: new Set(ids),
        previewActive: active,
      },
    }));
  },

  /**
   * Clear preview
   */
  clearPreview(timelineId: TimelineId): void {
    selectionStoreInternal.update(state => ({
      ...state,
      [timelineId]: {
        ...(state[timelineId] ?? DEFAULT_SELECTION),
        previewIds: new Set(),
        previewActive: false,
      },
    }));
  },

  /**
   * Set last selected index
   */
  setLastSelected(timelineId: TimelineId, index: number | null): void {
    selectionStoreInternal.update(state => ({
      ...state,
      [timelineId]: {
        ...(state[timelineId] ?? DEFAULT_SELECTION),
        lastSelectedIndex: index,
      },
    }));
  },

  /**
   * Handle click with shift/multi-select logic
   */
  handleClick(
    timelineId: TimelineId,
    index: number,
    options: {
      isMultiSelect?: boolean;
      isShiftSelect?: boolean;
      itemCount?: number;
    } = {}
  ): void {
    const { isMultiSelect = false, isShiftSelect = false, itemCount = 0 } = options;
    const current = selectionStore.get(timelineId);

    if (isMultiSelect) {
      // Toggle individual item
      selectionStore.toggleSelection(timelineId, index);
      selectionStore.setLastSelected(timelineId, index);
    } else if (isShiftSelect && current.lastSelectedIndex !== null) {
      // Range select
      const start = Math.min(current.lastSelectedIndex, index);
      const end = Math.max(current.lastSelectedIndex, index);
      const newSelection = new Set<number>();
      for (let i = start; i <= end && i < itemCount; i++) {
        newSelection.add(i);
      }
      selectionStore.setSelected(timelineId, newSelection);
    } else {
      // Single select
      selectionStore.setSelected(timelineId, new Set([index]));
      selectionStore.setLastSelected(timelineId, index);
    }
  },

  /**
   * Remove a timeline's selection
   */
  remove(timelineId: TimelineId): void {
    selectionStoreInternal.update(state => {
      const { [timelineId]: _, ...rest } = state;
      return rest;
    });
  },
};

// ============================================================================
// Playback Store API (per-timeline)
// ============================================================================

export const playbackStore = {
  subscribe: playbackStoreInternal.subscribe,

  /**
   * Get playback state for a timeline
   */
  get(timelineId: TimelineId): TimelinePlayback {
    const state = get(playbackStoreInternal);
    return state[timelineId] ?? DEFAULT_PLAYBACK;
  },

  /**
   * Create a derived store for a specific timeline's playback
   */
  forTimeline(timelineId: TimelineId): Readable<TimelinePlayback> {
    return derived(playbackStoreInternal, $store => {
      return $store[timelineId] ?? DEFAULT_PLAYBACK;
    });
  },

  /**
   * Initialize playback for a timeline
   */
  init(timelineId: TimelineId, duration: number = 0): void {
    playbackStoreInternal.update(state => ({
      ...state,
      [timelineId]: {
        ...DEFAULT_PLAYBACK,
        duration,
      },
    }));
  },

  /**
   * Update playback state
   */
  update(timelineId: TimelineId, updates: Partial<TimelinePlayback>): void {
    playbackStoreInternal.update(state => ({
      ...state,
      [timelineId]: {
        ...(state[timelineId] ?? DEFAULT_PLAYBACK),
        ...updates,
      },
    }));
  },

  /**
   * Set progress (0-1)
   */
  setProgress(timelineId: TimelineId, progress: number): void {
    playbackStoreInternal.update(state => {
      const current = state[timelineId] ?? DEFAULT_PLAYBACK;
      return {
        ...state,
        [timelineId]: {
          ...current,
          progress: Math.max(0, Math.min(1, progress)),
          currentTime: progress * current.duration,
        },
      };
    });
  },

  /**
   * Set current time
   */
  setCurrentTime(timelineId: TimelineId, time: number): void {
    playbackStoreInternal.update(state => {
      const current = state[timelineId] ?? DEFAULT_PLAYBACK;
      const clampedTime = Math.max(0, Math.min(time, current.duration));
      return {
        ...state,
        [timelineId]: {
          ...current,
          currentTime: clampedTime,
          progress: current.duration > 0 ? clampedTime / current.duration : 0,
        },
      };
    });
  },

  /**
   * Set duration
   */
  setDuration(timelineId: TimelineId, duration: number): void {
    playbackStoreInternal.update(state => {
      const current = state[timelineId] ?? DEFAULT_PLAYBACK;
      return {
        ...state,
        [timelineId]: {
          ...current,
          duration,
          progress: duration > 0 ? current.currentTime / duration : 0,
        },
      };
    });
  },

  /**
   * Set playing state
   */
  setPlaying(timelineId: TimelineId, isPlaying: boolean): void {
    playbackStoreInternal.update(state => ({
      ...state,
      [timelineId]: {
        ...(state[timelineId] ?? DEFAULT_PLAYBACK),
        isPlaying,
      },
    }));
  },

  /**
   * Remove a timeline's playback state
   */
  remove(timelineId: TimelineId): void {
    playbackStoreInternal.update(state => {
      const { [timelineId]: _, ...rest } = state;
      return rest;
    });
  },
};

// ============================================================================
// Timeline Context - Combines all stores for a single timeline
// ============================================================================

export interface TimelineContext {
  timelineId: TimelineId;
  state: Readable<TimelineState>;
  selection: Readable<TimelineSelection>;
  playback: Readable<TimelinePlayback>;

  // Actions
  setItems: (items: TimelineItem[]) => void;
  setDuration: (duration: number) => void;
  setHierarchy: (hierarchy: TimelineHierarchy | null) => void;
  setLoading: (loading: boolean) => void;

  selectItem: (
    index: number,
    options?: { isMultiSelect?: boolean; isShiftSelect?: boolean }
  ) => void;
  clearSelection: () => void;
  toggleSelection: (index: number) => void;

  seek: (time: number) => void;
  setProgress: (progress: number) => void;
  setPlaying: (isPlaying: boolean) => void;

  destroy: () => void;
}

/**
 * Create a timeline context for a specific timeline ID
 */
export function createTimelineContext(timelineId: TimelineId): TimelineContext {
  // Initialize stores for this timeline
  timelineStore.init(timelineId);
  selectionStore.init(timelineId);
  playbackStore.init(timelineId);

  // Create derived stores
  const state = timelineStore.forTimeline(timelineId);
  const selection = selectionStore.forTimeline(timelineId);
  const playback = playbackStore.forTimeline(timelineId);

  return {
    timelineId,
    state,
    selection,
    playback,

    // State actions
    setItems: items => timelineStore.setItems(timelineId, items),
    setDuration: duration => {
      timelineStore.setDuration(timelineId, duration);
      playbackStore.setDuration(timelineId, duration);
    },
    setHierarchy: hierarchy => timelineStore.setHierarchy(timelineId, hierarchy),
    setLoading: loading => timelineStore.setLoading(timelineId, loading),

    // Selection actions
    selectItem: (index, options = {}) => {
      const currentState = timelineStore.get(timelineId);
      selectionStore.handleClick(timelineId, index, {
        ...options,
        itemCount: currentState.items.length,
      });
    },
    clearSelection: () => selectionStore.clear(timelineId),
    toggleSelection: index => selectionStore.toggleSelection(timelineId, index),

    // Playback actions
    seek: time => playbackStore.setCurrentTime(timelineId, time),
    setProgress: progress => playbackStore.setProgress(timelineId, progress),
    setPlaying: isPlaying => playbackStore.setPlaying(timelineId, isPlaying),

    // Cleanup
    destroy: () => {
      timelineStore.remove(timelineId);
      selectionStore.remove(timelineId);
      playbackStore.remove(timelineId);
    },
  };
}

// ============================================================================
// Debug Mode Store (global - intentionally shared)
// ============================================================================

export const timelineDebugModeStore = writable(false);

export const timelineDebugMode = {
  subscribe: timelineDebugModeStore.subscribe,
  toggle: () => timelineDebugModeStore.update(v => !v),
  set: (value: boolean) => timelineDebugModeStore.set(value),
};

// ============================================================================
// Timeline Initialization
// ============================================================================

/**
 * Initialize timelines for the application.
 * If no timelines exist and there's a selected operation, creates a timeline for that operation.
 * Should be called once during frontend initialization.
 *
 * @param appStateStore - The reactive app state store
 * @returns Cleanup function to call on application destroy
 */
export function initializeTimelines(appStateStore?: Readable<AppState>): () => void {
  // Get the app state - either from passed store or import directly
  let currentAppState: AppState;
  let unsubscribe: (() => void) | null = null;

  const updateFromAppState = (state: AppState) => {
    currentAppState = state;
    const storeState = get(timelineStoreInternal);

    // Check if we have any existing timelines
    const hasExistingTimelines = Object.keys(storeState).length > 0;

    if (!hasExistingTimelines) {
      const selectedOpId = state.uiSettings?.selectedOperationId;

      if (selectedOpId && state.operations?.defs) {
        const operation = state.operations.defs[selectedOpId];

        if (operation) {
          console.log(
            `🔧 Timeline[${selectedOpId}]: Initializing timeline for operation "${operation.name}"`
          );

          // Derive timeline state from the operation
          const timelineState = deriveTimelineStateFromOperation(operation, selectedOpId, state);
          console.log(timelineState);
          // Initialize the timeline stores
          timelineStore.init(selectedOpId, timelineState);
          selectionStore.init(selectedOpId);
          playbackStore.init(selectedOpId, timelineState.duration);

          console.log(
            `✨ Timeline[${selectedOpId}]: Created timeline with ${timelineState.items?.length || 0} items, duration ${(timelineState.duration || 0).toFixed(2)}s`
          );
        }
      }
    }
  };

  if (appStateStore) {
    // Subscribe to app state changes
    unsubscribe = appStateStore.subscribe(updateFromAppState);
  } else {
    // Import appState dynamically to avoid circular dependencies
    import('./state.svelte').then(({ appState }) => {
      unsubscribe = appState.subscribe(updateFromAppState);
    });
  }

  return () => {
    if (unsubscribe) {
      unsubscribe();
    }
  };
}

/**
 * Derive a TimelineState from an operation definition.
 * This creates a minimal timeline state that can be populated by the waveform service.
 */
function deriveTimelineStateFromOperation(
  operation: any, // OperationDef from operation.ts
  operationId: string,
  appState: AppState
): Partial<TimelineState> {
  console.log(`🔧 Deriving timeline state for operation "${operation.name}" (${operationId})`);

  // Create basic timeline items from operation sources
  const timelineItems: AudioFileTimelineItem[] = [];
  let estimatedDuration = 0;

  // Extract files from operation sources (simplified approach)
  const extractedFiles = extractFilesFromOperation(operation, appState);

  if (extractedFiles.length > 0) {
    console.log(`📁 Found ${extractedFiles.length} files in operation sources`);

    // Create timeline items for each file
    extractedFiles.forEach((file, index) => {
      const duration = file.duration || 30; // Default duration if unknown
      const size = duration; // Will be normalized later

      timelineItems.push({
        kind: 'sample',
        id: `${operationId}_item_${index}`,
        fileName: file.path,
        svgPath: '', // Will be populated by waveform service
        startOffset: estimatedDuration / (estimatedDuration + duration), // Normalized later
        size: size / (estimatedDuration + duration), // Normalized later
        active: file.active ?? true,
        duration,
        children: [],
        parentId: undefined,
        depth: 0,
        isGroup: false,
        operationId,
        path: file.path,
      } as AudioFileTimelineItem);

      estimatedDuration += duration;
    });

    // Normalize offsets and sizes
    if (estimatedDuration > 0) {
      let currentOffset = 0;
      timelineItems.forEach(item => {
        const normalizedSize = (item.duration || 0) / estimatedDuration;
        item.startOffset = currentOffset;
        item.size = normalizedSize;
        currentOffset += normalizedSize;
      });
    }
  }

  // Build hierarchy if it's a merge operation
  let hierarchy: TimelineHierarchy | null = null;
  if (operation.kind === 'merge' && timelineItems.length > 0) {
    try {
      // Skip hierarchy building for now to avoid complex type issues
      // Will be handled by the waveform service
      hierarchy = null;
    } catch (error) {
      console.warn(`Failed to build hierarchy for operation ${operationId}:`, error);
    }
  }

  return {
    items: timelineItems,
    duration: estimatedDuration,
    hierarchy,
    waveformsLoading: timelineItems.length > 0, // Will be updated by waveform service
    playheadPosition: 0,
    isPlaying: false,
  };
}

/**
 * Extract file references from operation sources (simplified implementation)
 */
function extractFilesFromOperation(
  operation: any,
  appState: AppState
): Array<{ path: string; active?: boolean; duration?: number }> {
  const files: Array<{ path: string; active?: boolean; duration?: number }> = [];

  if (!operation.sources || !Array.isArray(operation.sources)) {
    return files;
  }

  for (const source of operation.sources) {
    switch (source.type) {
      case 'file':
        // Single file reference
        if (source.fileId) {
          files.push({
            path: source.fileId,
            active: true,
            // Try to get duration from appState if available
            duration: getDurationFromAppState(source.fileId, appState),
          });
        }
        break;

      case 'files':
        // Multiple file references
        if (source.fileIds && Array.isArray(source.fileIds)) {
          source.fileIds.forEach((fileId: string) => {
            files.push({
              path: fileId,
              active: true,
              duration: getDurationFromAppState(fileId, appState),
            });
          });
        }
        break;

      case 'active':
        // All active files from appState
        if (appState.timelineItems) {
          appState.timelineItems.forEach(item => {
            if (
              item.kind === 'sample' &&
              (item as AudioFileTimelineItem).active &&
              (item as AudioFileTimelineItem).fileName
            ) {
              const audioItem = item as AudioFileTimelineItem;
              files.push({
                path: audioItem.fileName,
                active: true,
                duration: audioItem.duration,
              });
            }
          });
        }
        break;

      case 'all':
        // All files from appState (both active and inactive)
        if (appState.timelineItems) {
          appState.timelineItems.forEach(item => {
            if (item.kind === 'sample' && (item as AudioFileTimelineItem).fileName) {
              const audioItem = item as AudioFileTimelineItem;
              files.push({
                path: audioItem.fileName,
                active: audioItem.active,
                duration: audioItem.duration,
              });
            }
          });
        }
        break;

      // Could extend with other source types as needed
    }
  }

  return files;
}

/**
 * Try to get file duration from app state
 */
function getDurationFromAppState(fileId: string, appState: AppState): number | undefined {
  // Look in timeline items
  const timelineItem = appState.timelineItems?.find(item => {
    if (item.kind === 'sample') {
      const audioItem = item as AudioFileTimelineItem;
      return audioItem.fileName === fileId || item.id === fileId;
    }
    return false;
  });

  if (timelineItem && timelineItem.kind === 'sample') {
    const audioItem = timelineItem as AudioFileTimelineItem;
    return audioItem.duration;
  }

  // Could extend to look in other places for duration info

  return undefined;
}

// ============================================================================
// Waveform Service Initialization Guard
// ============================================================================

let waveformServiceInitialized = false;

export function initWaveformServiceOnce(initFn: () => void): void {
  if (waveformServiceInitialized) {
    console.log('🔧 Waveform service already initialized, skipping');
    return;
  }
  waveformServiceInitialized = true;
  console.log('🔧 Initializing waveform service (first time)');
  initFn();
}

export function resetWaveformServiceInit(): void {
  waveformServiceInitialized = false;
}
