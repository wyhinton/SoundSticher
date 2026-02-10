import { listen } from '@tauri-apps/api/event';
import { derived, get, Readable, writable } from 'svelte/store';
import { persisted } from 'svelte-persisted-store';
import { durationCache } from '../durationCache';
import { logger } from '../logging';
import type { OperationDef, OperationId } from '../operation';
import { buildBackendGraphForTimeline, getOperationById } from '../operation';
import type { OpTimelineProgressEvent } from '../opPlaybackService';
import { appState, AudioFileTimelineItem, TimelineItem } from '../state.svelte';
import timelinePlaybackService from '../timelinePlaybackService';
import { dispatch, type ToggleTimelineVisibilityCommand } from '../undo/undo';
import { waveformCache, type Waveform } from '../waveformCache';
import { timelinePlaybackState, timelinePlaybackStoreService } from './timelinePlaybackState';
import { WAVEFORM_CONFIG } from '$lib/config/timelineConfig';

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

      // Update the runtime playhead state (NOT the persisted timeline state)
      timelinePlaybackState.update(state => {
        const currentAppState = get(appState);
        const timelineData = currentAppState.timelines?.timelines[timelineId];
        if (!timelineData) {
          logger.opPlayback.warning(`Timeline '${timelineId}' not found in appState`);
          return state;
        }

        // Calculate playheadTime from progress - use a default duration
        // The actual duration will come from the waveform state managed by TimelineViewer
        const playheadTime = progress * 30; // fallback; actual playhead is driven by viewer

        return {
          ...state,
          [timelineId]: { playheadTime, isPlaying: false, looping: true },
        };
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
  selection?: TimeRange;
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
  activeTimelineId: TimelineId | null;
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
};

/**
 * Generate a cryptographically unique timeline ID.
 * Uses the Web Crypto API for strong randomness + timestamp for sortability.
 * Format: `tl_<timestamp>_<128-bit random hex>`
 *
 * This ensures:
 * - No collisions (128-bit cryptographic randomness + timestamp)
 * - Sortable by creation time (timestamp prefix)
 * - Human-readable format
 */
function createTimelineId(): TimelineId {
  const timestamp = Date.now().toString(36); // e.g., "1234567890" in base36

  // Generate 16 bytes (128 bits) of cryptographic randomness
  const randomBytes = new Uint8Array(16);
  crypto.getRandomValues(randomBytes);

  // Convert to hex string
  const randomHex = Array.from(randomBytes)
    .map(b => b.toString(16).padStart(2, '0'))
    .join('')
    .slice(0, 24); // Use first 24 hex chars (96 bits) for readability

  return `tl_${timestamp}_${randomHex}`;
}

export function defaultTimelineViewState(): TimelineViewState {
  return {
    ...DEFAULT_VIEW_STATE,
  };
}

// ============================================================================
// SERIALIZATION FOR PERSISTENCE
// ============================================================================

// ============================================================================
// LEGACY SERIALIZATION REMOVED
// ============================================================================
// Timeline data is now stored as plain serializable objects in appState.timelines.
// The old timelinesStore, serializeTimeline, deserializeTimeline, and
// timelineStoreSerializer have been removed.

// Timeline data is now stored in appState.timelines and synced automatically.

/**
 * Initialize timeline synchronization with the backend.
 * With the new system, timeline keys are derived from appState,
 * so manual sync is no longer needed on key changes.
 */
export function initializeTimelineSync(): void {
  // No-op: timeline sync is handled via appState subscriptions
}

// Initialize timeline sync when module loads
// (Retained for backwards compatibility but is a no-op)

/**
 * Separate persisted store for timeline view states (zoom, scroll, selection, etc.)
 * This is kept separate from Timeline objects to allow view state to persist independently
 */
export const timelineViewStates = persisted<Record<TimelineId, TimelineViewState>>(
  'timeline-views:v1',
  {}
);

/**
 * Get the view state for a specific timeline, or return default if not found
 */
export function getTimelineViewState(timelineId: TimelineId): TimelineViewState {
  const viewStates = get(timelineViewStates);
  return viewStates[timelineId] || defaultTimelineViewState();
}

/**
 * Update the view state for a specific timeline
 */
export function updateTimelineViewState(
  timelineId: TimelineId,
  update: Partial<TimelineViewState>
): void {
  timelineViewStates.update(states => ({
    ...states,
    [timelineId]: {
      ...getTimelineViewState(timelineId),
      ...update,
    },
  }));
}

/**
 * Create a timeline-scoped waveform store
 * This replaces the global operationWaveforms store for individual timelines
 */
function createTimelineWaveformStore(timelineId: TimelineId) {
  console.log('CREATING STORE');
  //THIS BECOMES PERISSTED AND
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
    logger.waveform.operation(
      `Loading timeline "${timelineId}" (${filePaths.length} files, ${timelineWidth}px timeline)`
    );

    update(state => ({
      ...state,
      filePaths,
      loading: true,
      loadingWaveforms: false,
      error: null,
    }));

    try {
      // STEP 1: Load durations FIRST (from duration cache)
      logger.waveform.operation(`Step 1: Loading durations for ${filePaths.length} files`);
      const durationsMap = await durationCache.getBatch(filePaths);

      // Convert to our format and compute total
      const durations = new Map<string, number>();
      let totalDuration = 0;

      for (const [filePath, duration] of durationsMap.entries()) {
        if (duration && duration > 0) {
          durations.set(filePath, duration);
          totalDuration += duration;
        } else {
          logger.waveform.warning(`No valid duration for ${filePath}, skipping from layout`);
        }
      }

      if (totalDuration === 0) {
        throw new Error('No valid durations found for any files');
      }

      // STEP 2: Compute layout metrics
      const pxPerSecond = timelineWidth / totalDuration;
      logger.waveform.operation(
        `Step 2: Layout computed - total: ${totalDuration.toFixed(2)}s, ${pxPerSecond.toFixed(2)}px/sec`
      );

      // Update state with durations (layout is now stable)
      update(state => ({
        ...state,
        durations,
        totalDuration,
        pxPerSecond,
        loading: false,
        loadingWaveforms: true,
      }));

      // STEP 3: Request waveforms with computed widths
      logger.waveform.operation(`Step 3: Requesting waveforms with computed widths`);

      // Request waveforms for each file with its computed width
      const waveformPromises = filePaths.map(async filePath => {
        const duration = durations.get(filePath);
        if (!duration) return null;

        const widthPx = Math.max(1, Math.floor(duration * pxPerSecond));

        try {
          const waveform = await waveformCache.getOrFetch(filePath, {
            width: widthPx,
            height: WAVEFORM_CONFIG.DEFAULT_HEIGHT, // Timeline default height
            normalize: true,
          });

          // Update waveforms progressively
          update(state => {
            const newWaveforms = new Map(state.waveforms);
            newWaveforms.set(filePath, waveform);
            return { ...state, waveforms: newWaveforms };
          });

          return { filePath, waveform };
        } catch (error) {
          logger.waveform.error(`Failed to load waveform for ${filePath}:`, error);
          return null;
        }
      });

      await Promise.allSettled(waveformPromises);

      update(state => ({
        ...state,
        loadingWaveforms: false,
      }));

      logger.waveform.operation(
        `Timeline "${timelineId}" loaded: ${durations.size} durations, waveforms loading complete`
      );
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      update(state => ({
        ...state,
        loading: false,
        loadingWaveforms: false,
        error: errorMessage,
      }));

      logger.waveform.error(`Failed to load timeline waveforms for ${timelineId}: ${errorMessage}`);
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
      const operation = getOperationById(operationId);
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
  timelinePlaybackStoreService.addTimeline(timelineId);
  loadWaveformData();
  // buildBackendGraph();

  return {
    id: timelineId,
    source: { kind: 'operation', operationId },
    items: createOperationTimelineItems(operationIdReadable, waveformState),
    waveformState,
  };
}

/**
 * Toggle timeline visibility for a specific operation ID using the undo/redo system
 * If no timeline exists for this operation, create one
 * If timeline exists, remove it (toggle off)
 *
 * This operation is undoable/redoable.
 */
export function toggleTimelineVisibilityByOpIdDispatch(operationId: OperationId): void {
  logger.waveform.info(`Toggling timeline visibility for operation: ${operationId}`);

  // Create and dispatch the undoable command
  const command: ToggleTimelineVisibilityCommand = {
    type: 'toggle-timeline-visibility',
    operationId,
  };

  dispatch(command, 'Toggle Timeline Visibility');
}

export function toggleTimelineVisibilityByOpId(operationId: OperationId): void {
  const state = get(appState);
  const operation = state.operations?.defs[operationId];
  if (!operation) {
    logger.waveform.warning(`Operation ${operationId} not found`);
    return;
  }

  const isCurrentlyVisible = operation.visible || false;

  if (isCurrentlyVisible) {
    // Timeline exists - remove it (toggle off)
    logger.waveform.info(`Hiding timeline for operation: ${operationId}`);

    const timelineId = `tl_op_${operationId}`;

    appState.update(s => {
      const newDefs = { ...s.operations!.defs };
      const existingOp = newDefs[operationId];
      if (existingOp) {
        newDefs[operationId] = { ...existingOp, visible: false } as typeof existingOp;
      }

      const newTimelines = { ...(s.timelines?.timelines ?? {}) };
      delete newTimelines[timelineId];

      const newActiveId =
        s.timelines?.activeTimelineId === timelineId
          ? (Object.keys(newTimelines)[0] ?? null)
          : (s.timelines?.activeTimelineId ?? null);

      return {
        ...s,
        operations: { ...s.operations!, defs: newDefs },
        timelines: { timelines: newTimelines, activeTimelineId: newActiveId },
      };
    });

    // Clean up playback state
    timelinePlaybackStoreService.removeTimeline(timelineId);
    timelinePlaybackService.clearTimeline(timelineId);
  } else {
    // No timeline exists - create one (toggle on)
    logger.waveform.info(`Showing timeline for operation: ${operationId}`);

    const timelineId = `tl_op_${operationId}`;

    appState.update(s => {
      const newDefs = { ...s.operations!.defs };
      const existingOp2 = newDefs[operationId];
      if (existingOp2) {
        newDefs[operationId] = { ...existingOp2, visible: true } as typeof existingOp2;
      }

      const existingTimelines = s.timelines?.timelines ?? {};
      const newTimeline: import('../state.svelte').TimelineData = {
        id: timelineId,
        source: { kind: 'operation', operationId },
        items: [],
      };

      const newActiveId = s.timelines?.activeTimelineId ?? timelineId;

      return {
        ...s,
        operations: { ...s.operations!, defs: newDefs },
        timelines: {
          timelines: { ...existingTimelines, [timelineId]: newTimeline },
          activeTimelineId: newActiveId,
        },
      };
    });

    timelinePlaybackStoreService.addTimeline(timelineId);
    buildBackendGraphForTimeline(timelineId, operationId);
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
 * Check if an operation has a visible timeline (reads from operation.visible in appState)
 */
export function isOperationTimelineVisible(operationId: OperationId): boolean {
  const state = get(appState);
  const operation = state.operations?.defs[operationId];
  return operation?.visible || false;
}

/**
 * Derived store that provides an array of all visible operation timelines (from appState)
 * Each entry is a serializable TimelineData object.
 */
export const visibleOperationTimelines = derived(appState, $appState => {
  const timelines = $appState.timelines?.timelines;
  if (!timelines) return [];
  return Object.values(timelines).filter(
    (tl): tl is NonNullable<typeof tl> => tl != null && tl.source.kind === 'operation'
  );
});

/**
 * Derived store for the active timeline ID (from appState)
 */
export const activeTimelineId = derived(appState, $appState => {
  return $appState.timelines?.activeTimelineId ?? null;
});

/**
 * Set the active timeline ID (stored in appState)
 */
export function setActiveTimeline(timelineId: TimelineId | null): void {
  appState.update(state => ({
    ...state,
    timelines: {
      ...(state.timelines ?? { timelines: {}, activeTimelineId: null }),
      activeTimelineId: timelineId,
    },
  }));

  if (timelineId) {
    logger.timeline?.info(`Set active timeline: ${timelineId}`);
  } else {
    logger.timeline?.info('Cleared active timeline');
  }
}

/**
 * Get the current active timeline ID (from appState)
 */
export function getActiveTimelineId(): TimelineId | null {
  const state = get(appState);
  return state.timelines?.activeTimelineId ?? null;
}

/**
 * Get the current active timeline data (from appState)
 */
export function getActiveTimeline(): import('../state.svelte').TimelineData | null {
  const state = get(appState);
  const activeId = state.timelines?.activeTimelineId;
  return activeId ? (state.timelines?.timelines[activeId] ?? null) : null;
}

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
