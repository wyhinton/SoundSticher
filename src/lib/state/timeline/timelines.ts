import { listen } from '@tauri-apps/api/event';
import { derived, get } from 'svelte/store';
import { persisted } from 'svelte-persisted-store';
import { logger } from '../logging';
import type { OperationDef, OperationId } from '../operation';
import { buildBackendGraphForTimeline } from '../operation';
import type { OpTimelineProgressEvent } from '../opPlaybackService';
import { appState } from '../state.svelte';
import timelinePlaybackService from '../timelinePlaybackService';
import { type Waveform } from '../waveformCache';
import { timelinePlaybackState, timelinePlaybackStoreService } from './timelinePlaybackState';

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
        return {
          ...state,
          [timelineId]: { normalizedProgress: progress, isPlaying: false, looping: true },
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

export interface TimelineLayout {
  docked: TimelineId[];
  floating: TimelineId[];
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
