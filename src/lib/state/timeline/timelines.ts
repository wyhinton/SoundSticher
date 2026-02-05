import { derived, get, Readable, writable } from 'svelte/store';
import { appState, AudioFileTimelineItem, TimelineItem } from '../state.svelte';
import type { OperationDef, OperationId } from '../operation';
import { logger } from '../logging';
import { operationWaveforms, waveformCache, type Waveform } from '../waveformCache';
import { listen } from '@tauri-apps/api/event';
import type { OpTimelineProgressEvent } from '../opPlaybackService';
import { buildGraphForTimeline } from '../opPlaybackService';

// Timeline progress listener for updating individual timeline views
let timelineProgressUnlisten: (() => void) | null = null;

/**
 * Initialize the timeline-specific progress event listener
 * This replaces the global progress listener for timeline-aware playback
 */
async function initTimelineProgressListener(): Promise<void> {
  if (timelineProgressUnlisten) return;

  timelineProgressUnlisten = await listen<OpTimelineProgressEvent>(
    'op-timeline-progress',
    event => {
      const { timelineId, progress } = event.payload;

      if (!timelineId) {
        logger.opPlayback.info('Received progress event with no timelineId (legacy event)');
        return;
      }

      logger.opPlayback.info(
        `Timeline progress: '${timelineId}' -> ${(progress * 100).toFixed(1)}%`
      );

      // Update the specific timeline's view state
      timelinesStore.update(state => {
        const timeline = state.timelines[timelineId];
        if (!timeline) {
          logger.opPlayback.warning(`Timeline '${timelineId}' not found for progress update`);
          return state;
        }

        // Calculate playheadTime from progress and timeline duration
        if (timeline.waveformState) {
          const currentWaveformState = get(timeline.waveformState);
          const playheadTime = progress * currentWaveformState.totalDuration;

          // Update the timeline's view state with new playhead position
          const updatedTimeline = {
            ...timeline,
            view: {
              ...timeline.view,
              playheadTime,
            },
          };

          return {
            ...state,
            timelines: {
              ...state.timelines,
              [timelineId]: updatedTimeline,
            },
          };
        } else {
          logger.opPlayback.warning(
            `Timeline '${timelineId}' has no waveform state for progress update`
          );
          return state;
        }
      });
    }
  );

  logger.opPlayback.info('Timeline progress listener initialized');
}

/**
 * Cleanup the timeline progress listener
 */
function cleanupTimelineProgressListener(): void {
  if (timelineProgressUnlisten) {
    timelineProgressUnlisten();
    timelineProgressUnlisten = null;
    logger.opPlayback.info('Timeline progress listener cleaned up');
  }
}

export type TimelineId = string;
export type TrackId = string;

export type TimelineSource =
  | { kind: 'operation'; operationId: OperationId }
  | { kind: 'audioFile'; fileId: string }
  | { kind: 'comparison'; a: OperationId; b: OperationId }
  | { kind: 'custom'; tracks: TrackSpec[] };

export type TimeRange = {
  start: number;
  end: number;
};

export interface TrackSpec {
  id: TrackId;
  name?: string;
}

export interface TimelineViewState {
  zoom: number;
  scrollX: number;
  playheadTime: number;
  selection?: TimeRange;
  visibleTracks: TrackId[];
}

export interface TimelineWaveformState {
  timelineId: TimelineId;
  filePaths: string[];
  durations: Map<string, number>;
  waveforms: Map<string, Waveform>;
  totalDuration: number;
  pxPerSecond: number;
  loading: boolean;
  loadingWaveforms: boolean;
  error: string | null;
}

export interface Timeline {
  id: TimelineId;
  source: TimelineSource;
  view: TimelineViewState;
  items: Readable<TimelineItem[]>;
  waveformState?: {
    subscribe: (this: void, run: (value: TimelineWaveformState) => void) => () => void;
    load: (filePaths: string[], timelineWidth: number) => Promise<void>;
    clear: () => Promise<void>;
  };
}

export interface TimelineLayout {
  docked: TimelineId[];
  floating: TimelineId[];
}

export interface TimelinesState {
  timelines: Record<TimelineId, Timeline>;
}

/**
 * Metadata for a timeline item with hierarchy information
 *
 * IMPORTANT: operationId is the stable identifier for the operation that
 * produced this timeline item. Use this for deletion and updates, NOT names.
 */
export interface TimelineItemWithHierarchy {
  id: string; // Timeline item ID (sample file ID or merge group ID)
  path: string;
  active: boolean;
  index: number;
  kind: 'sample' | 'merge';
  operationId: string; // 🔑 Immutable operation ID (source of truth)
  operationName: string; // Display name (for UI only)
  depth: number;
  parentId: string | undefined;
  children: string[];
  isGroup: boolean;
}

const DEFAULT_VIEW_STATE: TimelineViewState = {
  zoom: 1,
  scrollX: 0,
  playheadTime: 0,
  visibleTracks: [],
};

function createTimelineId(): TimelineId {
  const now = Date.now().toString(36);
  const rand = Math.random().toString(36).slice(2, 8);
  return `tl_${now}_${rand}`;
}

export function defaultTimelineViewState(): TimelineViewState {
  return {
    ...DEFAULT_VIEW_STATE,
    visibleTracks: [],
  };
}

export const timelinesStore = writable<TimelinesState>({
  timelines: {},
});

/**
 * Create a timeline-scoped waveform store
 * This replaces the global operationWaveforms store for individual timelines
 */
function createTimelineWaveformStore(timelineId: TimelineId) {
  const { subscribe, set, update } = writable<TimelineWaveformState>({
    timelineId,
    filePaths: [],
    durations: new Map(),
    waveforms: new Map(),
    totalDuration: 0,
    pxPerSecond: 10,
    loading: false,
    loadingWaveforms: false,
    error: null,
  });

  async function load(filePaths: string[], timelineWidth: number) {
    // Set loading state
    update(state => ({ ...state, loading: true, loadingWaveforms: true, error: null }));

    try {
      // Load durations first (these determine layout)
      const durations = new Map<string, number>();
      let totalDuration = 0;

      // TODO: Load durations from duration cache
      // For now, use placeholder durations
      for (const filePath of filePaths) {
        const duration = 30; // Placeholder - should come from duration cache
        durations.set(filePath, duration);
        totalDuration += duration;
      }

      // Update with durations loaded (layout is now stable)
      update(state => ({
        ...state,
        filePaths,
        durations,
        totalDuration,
        pxPerSecond: timelineWidth / totalDuration,
        loading: false,
      }));

      // Load waveforms asynchronously (visual only, doesn't affect layout)
      const waveforms = new Map<string, Waveform>();

      for (const filePath of filePaths) {
        try {
          const waveform = await waveformCache.getOrFetch(filePath, {
            width: Math.round((durations.get(filePath) || 0) * (timelineWidth / totalDuration)),
            height: 100, // Default height
            normalize: true,
          });
          waveforms.set(filePath, waveform);

          // Update progressively as waveforms load
          update(state => ({
            ...state,
            waveforms: new Map(state.waveforms.set(filePath, waveform)),
          }));
        } catch (error) {
          logger.waveform.warning(`Failed to load waveform for ${filePath}:`, error);
        }
      }

      // All waveforms loaded
      update(state => ({ ...state, loadingWaveforms: false }));
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      logger.waveform.error(`Failed to load timeline waveforms for ${timelineId}:`, error);
      update(state => ({
        ...state,
        loading: false,
        loadingWaveforms: false,
        error: errorMessage,
      }));
    }
  }

  async function clear() {
    set({
      timelineId,
      filePaths: [],
      durations: new Map(),
      waveforms: new Map(),
      totalDuration: 0,
      pxPerSecond: 10,
      loading: false,
      loadingWaveforms: false,
      error: null,
    });
  }

  return {
    subscribe,
    load,
    clear,
  };
}

export function createOperationTimelineItems(
  operationId: Readable<string | null>,
  waveformStateStore: { subscribe: (run: (value: TimelineWaveformState) => void) => () => void }
): Readable<TimelineItem[]> {
  return derived(
    [operationId, appState, waveformStateStore],
    ([$operationId, $appState, $waveformState]) => {
      if (!$operationId || !$appState.operations?.defs) {
        return [];
      }

      const operation = $appState.operations.defs[$operationId];
      if (!operation) {
        logger.waveform.warning(`Operation id="${$operationId}" not found in definitions`);
        return [];
      }

      // Get hierarchical file items from the operation sources
      const hierarchicalItems = getHierarchicalTimelineItems(operation, $operationId);

      if (hierarchicalItems.length === 0) {
        logger.waveform.info(
          `No files found in operation "${operation.name}" (id: ${$operationId})`
        );
        return [];
      }

      // ✅ Use durations from timeline's waveform state (source of truth for layout)
      const { durations, totalDuration } = $waveformState;

      // If durations aren't loaded yet, we can't compute layout
      if (durations.size === 0 || totalDuration === 0) {
        logger.waveform.info(
          `Operation "${operation.name}" waiting for durations (${durations.size} loaded)`
        );
        return [];
      }

      // Log waveform loading status (informational, doesn't affect layout)
      const sampleItems = hierarchicalItems.filter(item => item.kind === 'sample');
      const loadedWaveforms = $waveformState.waveforms.size;
      if (loadedWaveforms < sampleItems.length) {
        logger.waveform.info(
          `Operation "${operation.name}" has ${loadedWaveforms}/${sampleItems.length} waveforms loaded (layout is stable)`
        );
      }

      // Build timeline items with start offsets based on DURATIONS (not waveforms)
      // We need to handle both samples (have durations) and MergeOps (span their children)
      const items: TimelineItem[] = [];
      let currentOffset = 0;

      // First pass: compute offsets and sizes for samples
      const sampleOffsets = new Map<string, { offset: number; size: number }>();

      for (const item of hierarchicalItems) {
        if (item.kind === 'sample') {
          const duration = durations.get(item.path);
          if (!duration || duration <= 0) {
            logger.waveform.warning(`No valid duration for ${item.path}, skipping`);
            continue;
          }

          const size = duration / totalDuration;
          sampleOffsets.set(item.id, { offset: currentOffset, size });
          currentOffset += size;
        }
      }

      // Second pass: build timeline items with hierarchy info
      // MergeOps span from their first child to their last child
      for (const item of hierarchicalItems) {
        if (item.kind === 'sample') {
          const layout = sampleOffsets.get(item.id);
          if (!layout) continue;

          const duration = durations.get(item.path);
          const waveform = $waveformState.waveforms.get(item.path);

          items.push({
            kind: 'sample',
            id: item.id,
            fileName: item.path,
            svgPath: waveform?.svgPath || '',
            startOffset: layout.offset,
            size: layout.size,
            active: item.active,
            duration,
            // Hierarchy properties
            children: [],
            parentId: item.parentId,
            depth: item.depth,
            isGroup: false,
            operationId: item.operationId,
            operationName: item.operationName,
          } as AudioFileTimelineItem);
        } else if (item.kind === 'merge') {
          // Calculate MergeOp span from its descendant samples
          let minOffset = 1;
          let maxEnd = 0;

          // Find all descendant samples to compute span
          const findDescendantOffsets = (itemId: string): void => {
            const targetItem = hierarchicalItems.find(i => i.id === itemId);
            if (!targetItem) return;

            if (targetItem.kind === 'sample') {
              const layout = sampleOffsets.get(targetItem.id);
              if (layout) {
                minOffset = Math.min(minOffset, layout.offset);
                maxEnd = Math.max(maxEnd, layout.offset + layout.size);
              }
            } else if (targetItem.kind === 'merge') {
              for (const childId of targetItem.children) {
                findDescendantOffsets(childId);
              }
            }
          };

          for (const childId of item.children) {
            findDescendantOffsets(childId);
          }

          // Only add MergeOp if it spans some samples
          if (maxEnd > minOffset) {
            items.push({
              kind: 'merge',
              id: item.id,
              fileName: item.operationName,
              svgPath: '', // MergeOps don't have waveforms
              startOffset: minOffset,
              size: maxEnd - minOffset,
              active: item.active,
              duration: totalDuration * (maxEnd - minOffset),
              // Hierarchy properties
              children: item.children,
              parentId: item.parentId,
              depth: item.depth,
              isGroup: true,
              operationId: item.operationId,
              operationName: item.operationName,
            } as AudioFileTimelineItem);
          }
        }
      }

      // Sort by startOffset to ensure proper rendering order
      items.sort((a, b) => {
        // MergeOps should render before their children (background)
        const aItem = a as AudioFileTimelineItem;
        const bItem = b as AudioFileTimelineItem;

        // First sort by depth (lower depth = render first = background)
        const depthA = aItem.depth ?? 0;
        const depthB = bItem.depth ?? 0;
        if (depthA !== depthB) {
          return depthA - depthB;
        }

        // Then by startOffset
        return a.startOffset - b.startOffset;
      });

      logger.waveform.info(
        `Generated ${items.length} timeline items for operation "${operation.name}" (id: ${$operationId}) ` +
          `(${items.filter(i => (i as AudioFileTimelineItem).kind === 'merge').length} groups, ` +
          `${items.filter(i => (i as AudioFileTimelineItem).kind === 'sample').length} samples, ` +
          `total duration: ${totalDuration.toFixed(1)}s)`
      );
      console.log(items);
      return items;
    }
  );
}

/**
 * Create a Timeline for a given operation ID
 */
export function createTimelineStateForOp(operationId: OperationId): Timeline {
  const timelineId = createTimelineId();
  const operationIdReadable = writable(operationId);
  const waveformState = createTimelineWaveformStore(timelineId);

  // Load waveform data for this operation
  // This should happen after the timeline is created to get the file paths from the operation
  const loadWaveformData = async () => {
    try {
      const currentAppState = get(appState);
      const operation = currentAppState.operations?.defs?.[operationId];
      if (operation) {
        const hierarchicalItems = getHierarchicalTimelineItems(operation, operationId);
        const filePaths = hierarchicalItems
          .filter(item => item.kind === 'sample')
          .map(item => item.path);

        if (filePaths.length > 0) {
          // Use default timeline width for initial load, this can be updated later
          await waveformState.load(filePaths, 1000);
        }
      }
    } catch (error) {
      logger.waveform.error(`Failed to load waveform data for operation ${operationId}:`, error);
    }
  };

  // Build the backend playback graph for this timeline
  const buildBackendGraph = async () => {
    try {
      logger.opPlayback.info(
        `Building backend graph for timeline ${timelineId}, operation ${operationId}`
      );
      await buildGraphForTimeline(timelineId, operationId);
      logger.opPlayback.info(`Successfully built backend graph for timeline ${timelineId}`);
    } catch (error) {
      logger.opPlayback.error(`Failed to build backend graph for timeline ${timelineId}:`, error);
    }
  };

  // Trigger async loading and backend graph building (don't await to avoid blocking timeline creation)
  loadWaveformData();
  buildBackendGraph();

  return {
    id: timelineId,
    source: { kind: 'operation', operationId },
    view: defaultTimelineViewState(),
    items: createOperationTimelineItems(operationIdReadable, waveformState),
    waveformState,
  };
}

/**
 * Toggle timeline visibility for a specific operation ID
 * If no timeline exists for this operation, create one
 * If timeline exists, remove it (toggle off)
 */
export function toggleTimelineVisibilityByOpId(operationId: OperationId): void {
  const currentState = get(timelinesStore);

  // Check if there's already a timeline for this operation
  const existingTimelineId = Object.keys(currentState.timelines).find(timelineId => {
    const timeline = currentState.timelines[timelineId];
    return (
      timeline &&
      timeline.source.kind === 'operation' &&
      timeline.source.operationId === operationId
    );
  });

  if (existingTimelineId) {
    // Timeline exists - remove it (toggle off)
    logger.waveform.info(`Hiding timeline for operation: ${operationId}`);

    timelinesStore.update(state => {
      const newTimelines = { ...state.timelines };
      delete newTimelines[existingTimelineId];

      return {
        ...state,
        timelines: newTimelines,
      };
    });
  } else {
    // No timeline exists - create one (toggle on)
    logger.waveform.info(`Showing timeline for operation: ${operationId}`);

    const newTimeline = createTimelineStateForOp(operationId);

    timelinesStore.update(state => ({
      ...state,
      timelines: {
        ...state.timelines,
        [newTimeline.id]: newTimeline,
      },
    }));
  }
}

/**
 * Get flattened timeline items with hierarchy for the root operation
 * This is used when we want to show nested MergeOps as distinct visual groups
 */
export function getHierarchicalTimelineItems(
  operation: OperationDef | undefined,
  operationId: string
): TimelineItemWithHierarchy[] {
  if (!operation) return [];

  const appStateValue = get(appState);
  const operations = appStateValue.operations?.defs;

  if (!operations) return [];

  // For the root operation, we only show its contents, not the root itself
  // (the root is implied by the selection)
  if (operation.kind === 'merge') {
    const items: TimelineItemWithHierarchy[] = [];
    let globalIndex = 0;

    // Process each source in the root MergeOp
    for (const source of operation.sources) {
      if (source.type === 'operation') {
        const childOp = operations[source.operationId];
        if (childOp) {
          // Check if this child is itself a MergeOp
          if (childOp.kind === 'merge') {
            // This is a nested MergeOp - flatten it with depth tracking
            const nestedItems = flattenOperationToTimelineItems(
              childOp,
              childOp.id,
              operations,
              0, // Start at depth 0 for nested MergeOps (they're top-level within our view)
              undefined
            );
            // Re-index the items
            for (const item of nestedItems) {
              item.index = globalIndex++;
            }
            items.push(...nestedItems);
          } else if (childOp.kind === 'sample') {
            // Regular sample - add as leaf
            for (const sampleSource of childOp.sources) {
              if (sampleSource.type === 'file') {
                items.push({
                  id: sampleSource.fileId,
                  path: sampleSource.fileId,
                  active: true,
                  index: globalIndex++,
                  kind: 'sample',
                  operationId: childOp.id,
                  operationName: childOp.name,
                  depth: 0,
                  parentId: undefined,
                  children: [],
                  isGroup: false,
                });
              }
            }
          }
        }
      }
    }

    return items;
  } else if (operation.kind === 'sample') {
    // Single sample operation
    const items: TimelineItemWithHierarchy[] = [];
    for (const source of operation.sources) {
      if (source.type === 'file') {
        items.push({
          id: source.fileId,
          path: source.fileId,
          active: true,
          index: 0,
          kind: 'sample',
          operationId: operation.id,
          operationName: operation.name,
          depth: 0,
          parentId: undefined,
          children: [],
          isGroup: false,
        });
      }
    }
    return items;
  }

  return [];
}

/**
 * Flatten an operation graph into timeline items while preserving hierarchy
 *
 * This walks the operation graph recursively and produces a flat array
 * where each item knows its:
 * - kind ('sample' or 'merge')
 * - depth (nesting level)
 * - parentId (immediate parent MergeOp)
 * - children (for MergeOps, the IDs of direct children)
 * - isGroup (true for MergeOps)
 */
function flattenOperationToTimelineItems(
  operation: OperationDef | undefined,
  operationId: string,
  operations: Record<string, OperationDef>,
  depth: number = 0,
  parentId: string | undefined = undefined
): TimelineItemWithHierarchy[] {
  if (!operation) return [];

  const items: TimelineItemWithHierarchy[] = [];
  let globalIndex = 0;

  function processOperation(
    op: OperationDef,
    opId: string,
    currentDepth: number,
    currentParentId: string | undefined
  ): TimelineItemWithHierarchy[] {
    const result: TimelineItemWithHierarchy[] = [];

    if (op.kind === 'sample') {
      // Leaf node - extract file from sources
      for (const source of op.sources) {
        if (source.type === 'file') {
          result.push({
            id: source.fileId,
            path: source.fileId,
            active: true,
            index: globalIndex++,
            kind: 'sample',
            operationId: op.id,
            operationName: op.name,
            depth: currentDepth,
            parentId: currentParentId,
            children: [],
            isGroup: false,
          });
        }
      }
    } else if (op.kind === 'merge') {
      // MergeOp - this is a group container
      const mergeId = `merge:${op.id}`;
      const childIds: string[] = [];
      const childItems: TimelineItemWithHierarchy[] = [];

      // Process each source in the MergeOp
      for (const source of op.sources) {
        if (source.type === 'operation') {
          const childOp = operations[source.operationId];
          if (childOp) {
            // Recursively process child operations
            const childResult = processOperation(
              childOp,
              source.operationId,
              currentDepth + 1,
              mergeId
            );

            // Collect child IDs (direct children only, not grandchildren)
            for (const item of childResult) {
              if (item.depth === currentDepth + 1) {
                childIds.push(item.id);
              }
            }

            childItems.push(...childResult);
          }
        }
      }

      // Add the MergeOp itself as a group container (at the current depth)
      // Note: We insert the MergeOp BEFORE its children for proper ordering
      result.push({
        id: mergeId,
        path: op.name,
        active: true,
        index: globalIndex++,
        kind: 'merge',
        operationId: op.id,
        operationName: op.name,
        depth: currentDepth,
        parentId: currentParentId,
        children: childIds,
        isGroup: true,
      });

      // Add all child items after the MergeOp
      result.push(...childItems);
    }

    return result;
  }

  return processOperation(operation, operationId, depth, parentId);
}

/**
 * Check if an operation has a visible timeline
 */
export function isOperationTimelineVisible(operationId: OperationId): boolean {
  const currentState = get(timelinesStore);

  return Object.values(currentState.timelines).some(
    timeline =>
      timeline &&
      timeline.source.kind === 'operation' &&
      timeline.source.operationId === operationId
  );
}

/**
 * Derived store that provides an array of all operation timelines
 */
export const operationTimelines = derived(timelinesStore, $timelinesStore => {
  return Object.values($timelinesStore.timelines).filter(
    timeline => timeline && timeline.source.kind === 'operation'
  );
});

/**
 * Timeline progress event management
 * These functions manage the timeline-specific progress events
 */
export const timelineProgressManager = {
  initTimelineProgressListener,
  cleanupTimelineProgressListener,
};

// Auto-initialize timeline progress listener when module loads
// This ensures timeline progress updates work as soon as the module is imported
if (typeof window !== 'undefined') {
  // Only in browser environment
  initTimelineProgressListener().catch(error => {
    logger.opPlayback.error('Failed to initialize timeline progress listener:', error);
  });
}
